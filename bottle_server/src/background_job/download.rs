use std::path::Path;

use futures::stream::StreamExt;
use serde::Serialize;
use tokio::{
    sync::{mpsc, watch},
    task,
};

use bottle_download::{DownloadTask, LocalImage};

use crate::{
    error::Result,
    state::{AppState, DatabasePool},
    util,
};

use super::entity::GeneralJobState;
use super::util::{DEFAULT_DOWNLOAD_CONCURRENCY, DEFAULT_DOWNLOAD_OVERWRITE};

#[derive(Debug, Clone)]
pub enum ImageDownloadJobState {
    Ready,
    Running {
        total: u64,
        success: u64,
        failure: u64,
    },
    Success {
        total: u64,
    },
    PartialSuccess {
        total: u64,
        success: u64,
        failures: Vec<ImageDownloadFailure>,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageDownloadFailure {
    url: String,
    error: String,
}

impl ImageDownloadJobState {
    fn finished(&self) -> bool {
        !matches!(self, ImageDownloadJobState::Running { .. })
    }

    fn new_running(total: u64) -> Self {
        Self::Running {
            total,
            success: 0,
            failure: 0,
        }
    }

    fn new_partial(total: u64, failures: Vec<ImageDownloadFailure>) -> Self {
        Self::PartialSuccess {
            total,
            success: total - failures.len() as u64,
            failures,
        }
    }

    fn adding_success(&self) -> Self {
        match self {
            ImageDownloadJobState::Running {
                total,
                success,
                failure,
            } => ImageDownloadJobState::Running {
                total: *total,
                success: *success + 1,
                failure: *failure,
            },
            _ => self.clone(),
        }
    }

    fn adding_failure(&self) -> Self {
        match self {
            ImageDownloadJobState::Running {
                total,
                success,
                failure,
            } => ImageDownloadJobState::Running {
                total: *total,
                success: *success,
                failure: *failure + 1,
            },
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImageDownloadJobStateResponse {
    state: GeneralJobState,
    total: u64,
    success: u64,
    failure: u64,
    error: Option<String>,
    failures: Option<Vec<ImageDownloadFailure>>,
}

impl From<&ImageDownloadJobState> for ImageDownloadJobStateResponse {
    fn from(state: &ImageDownloadJobState) -> Self {
        match state {
            ImageDownloadJobState::Ready => Default::default(),
            ImageDownloadJobState::Running {
                total,
                success,
                failure,
            } => Self {
                state: GeneralJobState::Running,
                total: *total,
                success: *success,
                failure: *failure,
                ..Default::default()
            },
            ImageDownloadJobState::Success { total } => Self {
                state: GeneralJobState::Success,
                total: *total,
                success: *total,
                ..Default::default()
            },
            ImageDownloadJobState::PartialSuccess {
                total,
                success,
                failures,
            } => Self {
                state: GeneralJobState::Failed,
                total: *total,
                success: *success,
                failure: failures.len() as u64,
                failures: Some(failures.clone()),
                ..Default::default()
            },
            ImageDownloadJobState::Failed { error } => Self {
                state: GeneralJobState::Failed,
                error: Some(error.clone()),
                ..Default::default()
            },
        }
    }
}

pub type ImageDownloadJobQueue = mpsc::UnboundedSender<()>;
pub type ImageDownloadJobStateReceiver = watch::Receiver<ImageDownloadJobState>;

/// Used in server handler
pub async fn send_image_download(app_state: &AppState) -> Result<()> {
    // 1. Skip if the job is already running
    let state = app_state.image_download_job_state.borrow().clone();
    if !state.finished() {
        tracing::warn!("Image download job is already running");
        return Err(anyhow::anyhow!("Image download job is already running"))?;
    }

    app_state.image_download_queue.send(())?;
    Ok(())
}

/// Set up before server started
pub fn listen_image_download(
    pool: DatabasePool,
    image_dir: impl AsRef<Path>,
) -> (ImageDownloadJobQueue, ImageDownloadJobStateReceiver) {
    // (1) MPSC channel: job queue
    let (job_sender, mut job_receiver) = mpsc::unbounded_channel();

    // (2) watch channel: job state
    let (state_sender, state_receiver) = watch::channel(ImageDownloadJobState::Ready);
    let state_sender2 = state_sender.clone();

    let image_dir = image_dir.as_ref().to_path_buf();
    task::spawn(async move {
        while (job_receiver.recv().await).is_some() {
            let result = download_images(
                pool.clone(),
                state_sender.clone(),
                &image_dir,
                DEFAULT_DOWNLOAD_CONCURRENCY,
                DEFAULT_DOWNLOAD_OVERWRITE,
            )
            .await;

            if let Err(e) = result {
                tracing::error!("Image download job failed: {}", e);
                let _ = state_sender2.send(ImageDownloadJobState::Failed { error: e.to_string() });
            }
        }
    });

    (job_sender, state_receiver)
}

enum ImageDownloadMessage {
    Success,
    Failed,
}

async fn download_images(
    pool: DatabasePool,
    // (2) watch channel: job state
    state_sender: watch::Sender<ImageDownloadJobState>,
    image_dir: impl AsRef<Path>,
    max_concurrency: usize,
    overwrite: bool,
) -> Result<()> {
    // 1. Prepare download futures
    let tasks = {
        let conn = &mut pool.get()?;
        bottle_library::get_download_tasks(conn, image_dir)?
    };
    if tasks.is_empty() {
        tracing::info!("Image download job done. No images to download");
        state_sender.send(ImageDownloadJobState::Success { total: 0 })?;
        return Ok(());
    }

    let task_count = tasks.len() as u64;
    // (3) MPSC channel: monitor subtask results
    let (subtask_sender, mut subtask_receiver) = mpsc::channel(1);
    let futures = tasks
        .iter()
        .map(|task| download_image(pool.clone(), subtask_sender.clone(), task, overwrite))
        .collect::<Vec<_>>();
    let stream = futures::stream::iter(futures).buffer_unordered(max_concurrency);

    // 2. Listen to subtask results and update job state
    let mut state = ImageDownloadJobState::new_running(task_count);
    state_sender.send(state.clone())?;
    let state_sender2 = state_sender.clone();
    let state_update_task = task::spawn(async move {
        while let Some(msg) = subtask_receiver.recv().await {
            state = match msg {
                ImageDownloadMessage::Success => state.adding_success(),
                ImageDownloadMessage::Failed => state.adding_failure(),
            };
            let _ = state_sender.send(state.clone());
        }
    });

    // 3. Download images
    tracing::info!("Image download job started. Downloading {} images", task_count);
    let images = stream.collect::<Vec<_>>().await;
    state_update_task.abort();

    // 4. Collect failures and send final state
    let failures = tasks
        .iter()
        .zip(images.iter())
        .filter_map(|(task, result)| {
            result.as_ref().err().map(|e| ImageDownloadFailure {
                url: task.url.clone(),
                error: e.to_string(),
            })
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        tracing::info!("Image download job done. Downloaded all {} images", task_count);
        state_sender2.send(ImageDownloadJobState::Success { total: task_count })?;
    } else {
        tracing::warn!(
            "Image download job done. Downloaded {} images, failed to download {} images",
            task_count - failures.len() as u64,
            failures.len()
        );
        state_sender2.send(ImageDownloadJobState::new_partial(task_count, failures))?;
    };

    Ok(())
}

async fn download_image(
    pool: DatabasePool,
    // (3) MPSC channel: monitor subtask results
    subtask_sender: mpsc::Sender<ImageDownloadMessage>,
    task: &DownloadTask,
    overwrite: bool,
) -> Result<LocalImage> {
    // 1. Download image
    let result = util::retry(|| util::timeout(bottle_download::download_image(task, overwrite))).await;

    // 2. Update database if succeed
    match &result {
        Ok(image) => {
            tracing::info!("Downloaded image: {}", image.relpath);
            let conn = &mut pool.get()?;
            bottle_library::update_from_local_image(conn, task.image_id, image)?;
            subtask_sender.send(ImageDownloadMessage::Success).await?;
        }
        Err(e) => {
            tracing::error!("Failed to download image {}: {}", task.url, e);
            subtask_sender.send(ImageDownloadMessage::Failed).await?;
        }
    }

    result
}
