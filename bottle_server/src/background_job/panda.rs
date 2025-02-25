use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::StreamExt;
use itertools::Itertools;
use serde::Serialize;
use tokio::{
    sync::{mpsc, watch, RwLock},
    task,
    time::{self, Duration},
};

use bottle_core::{library::RemoteImage, Database};
use bottle_download::{DownloadTask, LocalImage};
use bottle_panda::download::{PandaDownloadTask, PandaImageTask};
use panda_client::PandaClient;

use crate::util;
use crate::{
    error::Result,
    state::{AppState, DatabasePool},
};

use super::entity::GeneralJobState;
use super::util::{DEFAULT_DELAY_MS, DEFAULT_DOWNLOAD_CONCURRENCY, DEFAULT_DOWNLOAD_OVERWRITE};

const GUESSED_PAGE_SIZE: i32 = 20;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PandaGalleryID(pub i64);

#[derive(Debug, Clone)]
pub struct PandaDownloadJob(PandaDownloadTask);

impl PandaDownloadJob {
    pub fn id(&self) -> PandaGalleryID {
        PandaGalleryID(self.0.gid)
    }
}

#[derive(Debug, Clone)]
pub enum PandaDownloadJobState {
    Ready,
    FetchingMetadata {
        total: i32,
        success: i32,
    },
    Running {
        total: i32,
        success: i32,
        failure: i32,
    },
    Success {
        total: i32,
    },
    PartialSuccess {
        total: i32,
        success: i32,
        failures: Vec<PandaImageDownloadFailure>,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct PandaImageDownloadFailure {
    gid: i64,
    index: i32,
    error: String,
}

impl PandaDownloadJobState {
    fn finished(&self) -> bool {
        matches!(
            self,
            PandaDownloadJobState::Success { .. }
                | PandaDownloadJobState::Failed { .. }
                | PandaDownloadJobState::PartialSuccess { .. }
        )
    }

    fn new_running(tasks: &[PandaImageTask]) -> Self {
        Self::Running {
            total: tasks.len() as i32,
            success: tasks.iter().filter(|t| t.downloaded).count() as i32,
            failure: 0,
        }
    }

    fn new_partial(total: i32, failures: Vec<PandaImageDownloadFailure>) -> Self {
        Self::PartialSuccess {
            total,
            success: total - failures.len() as i32,
            failures,
        }
    }

    fn adding_success(&self) -> Self {
        match self {
            PandaDownloadJobState::Running {
                total,
                success,
                failure,
            } => PandaDownloadJobState::Running {
                total: *total,
                success: *success + 1,
                failure: *failure,
            },
            _ => self.clone(),
        }
    }

    fn adding_failure(&self) -> Self {
        match self {
            PandaDownloadJobState::Running {
                total,
                success,
                failure,
            } => PandaDownloadJobState::Running {
                total: *total,
                success: *success,
                failure: *failure + 1,
            },
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PandaDownloadJobStateResponse {
    pub gid: i64,
    pub title: String,
    pub state: GeneralJobState,
    pub metadata_fetched: bool,
    pub total_pages: i32,
    pub success_pages: i32,
    pub total_images: i32,
    pub success_images: i32,
    pub failure_images: i32,
    pub failures: Option<Vec<PandaImageDownloadFailure>>,
    pub error: Option<String>,
}

impl PandaDownloadJobStateResponse {
    pub fn new(id: &PandaGalleryID, title: String, state: &PandaDownloadJobState) -> Self {
        let mut result = match state {
            PandaDownloadJobState::Ready => Default::default(),
            PandaDownloadJobState::FetchingMetadata { total, success } => Self {
                state: GeneralJobState::Running,
                total_pages: *total,
                success_pages: *success,
                ..Default::default()
            },
            PandaDownloadJobState::Running {
                total,
                success,
                failure,
            } => Self {
                state: GeneralJobState::Running,
                metadata_fetched: true,
                total_images: *total,
                success_images: *success,
                failure_images: *failure,
                ..Default::default()
            },
            PandaDownloadJobState::Success { total } => Self {
                state: GeneralJobState::Success,
                metadata_fetched: true,
                total_images: *total,
                ..Default::default()
            },
            PandaDownloadJobState::PartialSuccess {
                total,
                success,
                failures,
            } => Self {
                state: GeneralJobState::Failed,
                metadata_fetched: true,
                total_images: *total,
                success_images: *success,
                failure_images: failures.len() as i32,
                failures: Some(failures.clone()),
                ..Default::default()
            },
            PandaDownloadJobState::Failed { error } => Self {
                state: GeneralJobState::Failed,
                error: Some(error.clone()),
                ..Default::default()
            },
        };
        result.gid = id.0;
        result.title = title;
        result
    }
}

pub type PandaDownloadJobQueue = mpsc::UnboundedSender<PandaDownloadJob>;
pub type PandaDownloadJobStateSender = watch::Sender<PandaDownloadJobState>;
pub type PandaDownloadJobStateReceiver = watch::Receiver<PandaDownloadJobState>;
pub type PandaDownloadJobStateSenderMap = Arc<RwLock<HashMap<PandaGalleryID, PandaDownloadJobStateSender>>>;
pub type PandaDownloadJobStateReceiverMap = Arc<RwLock<HashMap<PandaGalleryID, PandaDownloadJobStateReceiver>>>;

/// Used in server handler, return true if the job sent successfully
pub async fn send_panda_download(app_state: &AppState, task: PandaDownloadTask) -> Result<bool> {
    let job = PandaDownloadJob(task);
    let id = job.id();

    let state = app_state
        .panda_download_state_map
        .read()
        .await
        .get(&id)
        .map(|rx| rx.borrow().clone());

    if let Some(state) = state {
        if !state.finished() {
            // 1. Skip if the job is already running
            return Ok(false);
        } else {
            // 2. Reset the state to ready if the job is finished
            let state_sender = app_state
                .panda_download_state_sender_map
                .read()
                .await
                .get(&id)
                .expect("job sender not found")
                .clone();
            state_sender.send(PandaDownloadJobState::Ready)?;
        }
    } else {
        // 3. Establish state channel if not exists
        // (2) Watch channel: job state
        let (state_sender, state_receiver) = watch::channel(PandaDownloadJobState::Ready);
        app_state
            .panda_download_state_sender_map
            .write()
            .await
            .insert(id.clone(), state_sender);
        app_state
            .panda_download_state_map
            .write()
            .await
            .insert(id.clone(), state_receiver);

        // Record gallery title for response
        app_state
            .panda_gallery_title_map
            .write()
            .await
            .insert(id.clone(), job.0.title.clone());
    }
    app_state.panda_download_queue.send(job)?;

    Ok(true)
}

/// Set up before server started
pub fn listen_panda_download(
    pool: DatabasePool,
    state_sender_map: PandaDownloadJobStateSenderMap,
    image_dir: impl AsRef<Path>,
) -> Result<PandaDownloadJobQueue> {
    // (1) MPSC unbounded channel: job queue
    let (job_sender, mut job_receiver) = mpsc::unbounded_channel::<PandaDownloadJob>();

    let image_dir = image_dir.as_ref().to_path_buf();
    task::spawn(async move {
        while let Some(job) = job_receiver.recv().await {
            let state_sender = state_sender_map
                .read()
                .await
                .get(&job.id())
                .expect("job state sender not found")
                .clone();

            let gid = job.id().0;
            let result = download_gallery(
                &pool,
                state_sender.clone(),
                job,
                &image_dir,
                DEFAULT_DOWNLOAD_CONCURRENCY,
                DEFAULT_DOWNLOAD_OVERWRITE,
                DEFAULT_DELAY_MS,
            )
            .await;

            if let Err(e) = result {
                tracing::error!("Panda download job failed: Gallery {}. {}", gid, e);
                let _ = state_sender.send(PandaDownloadJobState::Failed { error: e.to_string() });
            }
        }
    });

    Ok(job_sender)
}

enum PandaDownloadMessage {
    Success,
    Failed,
}

#[allow(clippy::too_many_arguments)]
async fn download_gallery(
    pool: &DatabasePool,
    // (2) Watch channel: job state
    state_sender: watch::Sender<PandaDownloadJobState>,
    job: PandaDownloadJob,
    image_dir: impl AsRef<Path>,
    max_concurrency: usize,
    overwrite: bool,
    delay_ms: u64,
) -> Result<()> {
    use bottle_core::feed::Account;

    // 1. Get account and then panda client
    let client = {
        let db = &mut pool.get()?;
        let account = bottle_panda::PandaAccount::default(db)?;
        let auth = account.auth(db)?.ok_or(bottle_core::Error::NotLoggedIn(
            "Downloading panda gallery needs an account".to_string(),
        ))?;
        PandaClient::new(auth)?
    };

    // 1. Fetch incomplete post/media metadata, and update the task
    let task = job.0;
    tracing::info!("Panda download job started: Gallery {} {}", task.gid, task.title);
    let gallery_task = {
        let db = &mut pool.get()?;
        fetch_metadata(db, &client, state_sender.clone(), task, delay_ms).await
    }?;

    // 2. Prepare download futures
    // (3) MPSC channel: monitor subtask results
    let (subtask_sender, mut subtask_receiver) = mpsc::channel(1);
    let image_tasks = gallery_task.image_tasks.iter().filter(|task| !task.downloaded);
    let futures = image_tasks
        .clone()
        .map(|task| {
            download_image_wrapped(
                pool.clone(),
                client.clone(),
                subtask_sender.clone(),
                task,
                &gallery_task,
                image_dir.as_ref(),
                overwrite,
            )
        })
        .collect::<Vec<_>>();
    let stream = futures::stream::iter(futures).buffer_unordered(max_concurrency);

    // 3. Listen to subtask results and update job state
    let mut state = PandaDownloadJobState::new_running(&gallery_task.image_tasks);
    state_sender.send(state.clone())?;
    let state_sender2 = state_sender.clone();
    let state_update_task = task::spawn(async move {
        while let Some(msg) = subtask_receiver.recv().await {
            state = match msg {
                PandaDownloadMessage::Success => state.adding_success(),
                PandaDownloadMessage::Failed => state.adding_failure(),
            };
            let _ = state_sender.send(state.clone());
        }
    });

    // 4. Download images
    tracing::info!(
        "Panda gallery {}: Downloading {} images",
        gallery_task.gid,
        image_tasks.clone().count()
    );
    let images = stream.collect::<Vec<_>>().await;
    state_update_task.abort();

    // 5. Collect failures and send final state
    let failures = image_tasks
        .zip(images.iter())
        .filter_map(|(task, result)| {
            result.as_ref().err().map(|e| PandaImageDownloadFailure {
                gid: task.gid,
                index: task.index,
                error: e.to_string(),
            })
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        tracing::info!(
            "Panda download job done: Gallery {}. Downloaded all {} images",
            gallery_task.gid,
            gallery_task.media_count
        );
        state_sender2.send(PandaDownloadJobState::Success {
            total: gallery_task.media_count,
        })?;
    } else {
        tracing::warn!(
            "Panda download job done: Gallery {}. Downloaded {} images, failed to download {} images",
            gallery_task.gid,
            gallery_task.media_count - failures.len() as i32,
            failures.len()
        );
        state_sender2.send(PandaDownloadJobState::new_partial(gallery_task.media_count, failures))?;
    };

    Ok(())
}

async fn download_image_wrapped(
    pool: DatabasePool,
    client: PandaClient,
    subtask_sender: mpsc::Sender<PandaDownloadMessage>,
    task: &PandaImageTask,
    gallery_task: &PandaDownloadTask,
    image_dir: impl AsRef<Path>,
    overwrite: bool,
) -> Result<LocalImage> {
    let result = download_image(&pool, &client, task, gallery_task, image_dir, overwrite).await;
    let _ = match &result {
        Ok(_) => {
            tracing::info!("Panda gallery {}: Downloaded image {}", task.gid, task.index);
            subtask_sender.send(PandaDownloadMessage::Success).await
        }
        Err(e) => {
            tracing::error!(
                "Panda gallery {}: Failed to download image {}: {}",
                task.gid,
                task.index,
                e
            );
            subtask_sender.send(PandaDownloadMessage::Failed).await
        }
    };
    result
}

async fn download_image(
    pool: &DatabasePool,
    client: &PandaClient,
    task: &PandaImageTask,
    gallery_task: &PandaDownloadTask,
    image_dir: impl AsRef<Path>,
    overwrite: bool,
) -> Result<LocalImage> {
    // 1. Fetch image info
    let result = client.image(task.gid as u64, &task.token, task.index as u32).await?;

    // 2. Download image
    let index_prefix = format!(
        "{:0width$}",
        task.index,
        width = gallery_task.media_count.to_string().len()
    );
    let download_task = DownloadTask {
        url: result.url.clone(),
        root_dir: image_dir.as_ref().to_path_buf(),
        subdir: PathBuf::from("panda").join(task.gid.to_string()),
        filename: PathBuf::from(format!("{}_{}", index_prefix, result.filename)),
        // Only a placeholder, create image record after downloading
        image_id: 0,
    };
    let local_image = util::retry(|| util::timeout(bottle_download::download_image(&download_task, overwrite))).await?;

    // 3. Update image and panda_media
    let db = &mut pool.get()?;
    let image_id = if let Some(image_id) = task.image_id {
        image_id
    } else {
        // Image does not exist, create new image
        let remote_image = RemoteImage {
            filename: result.filename.clone(),
            url: result.url.clone(),
            page_index: Some(task.index),
        };
        let image_view = bottle_library::add_remote_image(db, &remote_image, gallery_task.work_id)?;
        image_view.id
    };
    bottle_panda::download::save_image(db, task.gid, &result)?;
    bottle_library::update_from_local_image(db, image_id, &local_image)?;
    if task.index == 0 {
        // Update work cover image
        bottle_library::update_work_from_local_image(db, gallery_task.work_id, &local_image)?;
    }

    Ok(local_image)
}

/// Fetch and update all missing post/media data, returning the updated task
async fn fetch_metadata<'a>(
    db: Database<'a>,
    client: &PandaClient,
    // (2) Watch channel: job state
    state_sender: watch::Sender<PandaDownloadJobState>,
    task: PandaDownloadTask,
    delay_ms: u64,
) -> Result<PandaDownloadTask> {
    let mut task = task;
    let mut page_token = task
        .image_tasks
        .iter()
        .map(|j| (j.index, j.token.clone()))
        .collect::<HashMap<_, _>>();

    let mut page_count: Option<i32> = None;
    let mut page_size: Option<i32> = None;
    let mut media_count = task.media_count;
    let mut existing_indices = task.image_tasks.iter().map(|m| m.index).collect::<HashSet<_>>();
    let mut pages = pages_to_fetch(media_count, page_count, page_size, &existing_indices);
    tracing::info!(
        "Panda gallery {}: Incomplete pages: {}",
        task.gid,
        pages.iter().rev().join(", ")
    );

    // Send initial state with guessed page count
    let guessed_page_count = guessed_page_count(media_count, GUESSED_PAGE_SIZE);
    let mut state = PandaDownloadJobState::FetchingMetadata {
        total: guessed_page_count,
        success: guessed_page_count - pages.len() as i32,
    };
    state_sender.send(state.clone())?;

    // Fetch missing preview pages until all pages are fetched
    while let Some(page) = pages.pop() {
        let result = util::retry(|| util::timeout(client.gallery(task.gid as u64, &task.token, page as u32))).await?;

        // Update page count and page size
        page_count = Some(result.preview_page_count as i32);
        page_size = Some(result.previews.len() as i32);

        // 1. Save gallery metadata if not complete
        // And check if media count is consistent. If not, update page count
        let count_inconsistent = result.gallery.image_count as i32 != task.media_count;
        if count_inconsistent {
            media_count = result.gallery.image_count as i32;
            task.media_count = media_count;
            tracing::warn!(
                "Panda gallery {}: Media count changed from {} to {}",
                task.gid,
                task.media_count,
                result.gallery.image_count
            );
        }
        if !task.has_detail || count_inconsistent {
            bottle_panda::download::update_gallery(db, &result.gallery, &result.detail)?;
        }

        // 2. Check if media tokens are consistent. If not, refetch all media
        let mut media_inconsistent = false;
        for preview in result.previews.iter() {
            if let Some(token) = page_token.get(&(preview.index as i32)) {
                if token != &preview.token {
                    media_inconsistent = true;
                    tracing::warn!(
                        "Panda gallery {}: Media token changed at media {}",
                        task.gid,
                        preview.index
                    );
                    break;
                }
            }
        }
        if media_inconsistent {
            bottle_panda::download::remove_previews(db, task.gid)?;
            task.image_tasks.clear();
            page_token.clear();
            existing_indices.clear();
            tracing::warn!(
                "Panda gallery {}: Media tokens inconsistent, refetching all media",
                task.gid
            )
        }

        // 3. Save the gallery previews and update existing media indices
        if !media_inconsistent {
            bottle_panda::download::save_previews(db, &result)?;
            let mut added_indices = Vec::new();
            // Update new image tasks and existing media indices
            for preview in result.previews.iter() {
                if !existing_indices.contains(&(preview.index as i32)) {
                    existing_indices.insert(preview.index as i32);
                    added_indices.push(preview.index);
                    task.image_tasks.push(PandaImageTask {
                        gid: task.gid,
                        index: preview.index as i32,
                        token: preview.token.clone(),
                        image_id: None,
                        downloaded: false,
                    });
                }
            }
            tracing::info!(
                "Panda gallery {}: Saved media {} on page {}",
                task.gid,
                added_indices.iter().join(", "),
                page
            );
        }

        // Determine next page to fetch
        pages = pages_to_fetch(media_count, page_count, page_size, &existing_indices);

        // Send state update
        state = PandaDownloadJobState::FetchingMetadata {
            total: page_count.unwrap_or(guessed_page_count),
            success: page_count.unwrap_or(guessed_page_count) - pages.len() as i32,
        };
        state_sender.send(state.clone())?;

        // Sleep before next fetch
        if pages.is_empty() {
            break;
        }
        time::sleep(Duration::from_millis(delay_ms)).await;
    }

    tracing::info!("Panda gallery {}: Metadata fetched", task.gid);
    Ok(task)
}

fn pages_to_fetch(
    media_count: i32,
    page_count: Option<i32>,
    page_size: Option<i32>,
    existing_indices: &HashSet<i32>,
) -> Vec<i32> {
    let page_size = page_size.unwrap_or(GUESSED_PAGE_SIZE);
    let page_count = page_count.unwrap_or(guessed_page_count(media_count, page_size));

    let required_indices = HashSet::from_iter(0..media_count)
        .difference(existing_indices)
        .cloned()
        .collect::<HashSet<_>>();

    (0..page_count)
        .filter(|i| {
            let start = i * page_size;
            let end = (i + 1) * page_size;
            let indices = (start..end).collect::<HashSet<_>>();
            indices.intersection(&required_indices).count() > 0
        })
        // Reverse the order since Vec can only pop from the end
        .rev()
        .collect()
}

fn guessed_page_count(media_count: i32, page_size: i32) -> i32 {
    (media_count as f64 / page_size as f64).ceil() as i32
}
