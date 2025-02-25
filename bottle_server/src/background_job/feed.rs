use std::{collections::HashMap, sync::Arc};

use serde::Serialize;
use tokio::{
    sync::{mpsc, watch, RwLock},
    task,
    time::{self, Duration},
};

use bottle_core::feed::SaveResult;

use crate::{
    error::Result,
    state::{AppState, DatabasePool},
    util::{self, FeedContextWrapper, FeedIdentifier, FeedWrapper},
};

use super::{entity::GeneralJobState, util::DEFAULT_DELAY_MS};

#[derive(Debug, Clone)]
pub enum FeedUpdateJobState {
    Ready,
    Running { fetched: u64 },
    Success { fetched: u64 },
    Failed { error: String },
}

impl FeedUpdateJobState {
    pub fn finished(&self) -> bool {
        matches!(
            self,
            FeedUpdateJobState::Success { .. } | FeedUpdateJobState::Failed { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedUpdateJobStateResponse {
    community: String,
    feed_id: i32,
    fetched: u64,
    state: GeneralJobState,
    error: Option<String>,
}

impl FeedUpdateJobStateResponse {
    pub fn new(id: &FeedIdentifier, state: &FeedUpdateJobState) -> Self {
        Self {
            community: id.community.clone(),
            feed_id: id.feed_id,
            state: match state {
                FeedUpdateJobState::Ready => GeneralJobState::Ready,
                FeedUpdateJobState::Running { .. } => GeneralJobState::Running,
                FeedUpdateJobState::Success { .. } => GeneralJobState::Success,
                FeedUpdateJobState::Failed { .. } => GeneralJobState::Failed,
            },
            fetched: match state {
                FeedUpdateJobState::Running { fetched } => *fetched,
                FeedUpdateJobState::Success { fetched } => *fetched,
                _ => 0,
            },
            error: match state {
                FeedUpdateJobState::Failed { error } => Some(error.clone()),
                _ => None,
            },
        }
    }
}

pub type FeedUpdateJobQueue = mpsc::UnboundedSender<FeedIdentifier>;
pub type FeedUpdateJobStateSender = watch::Sender<FeedUpdateJobState>;
pub type FeedUpdateJobStateReceiver = watch::Receiver<FeedUpdateJobState>;
pub type FeedUpdateJobStateSenderMap = Arc<RwLock<HashMap<FeedIdentifier, FeedUpdateJobStateSender>>>;
pub type FeedUpdateJobStateReceiverMap = Arc<RwLock<HashMap<FeedIdentifier, FeedUpdateJobStateReceiver>>>;

/// Used in server handler. Return true if the job sent successfully.
pub async fn send_feed_update(app_state: &AppState, id: FeedIdentifier) -> Result<bool> {
    let state = app_state
        .feed_update_state_map
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
                .feed_update_state_sender_map
                .read()
                .await
                .get(&id)
                .expect("job sender not found")
                .clone();
            state_sender.send(FeedUpdateJobState::Ready)?;
        }
    } else {
        // 3. Establish state channel if not exists
        // (2) Watch channel: job state
        let (state_sender, state_receiver) = watch::channel(FeedUpdateJobState::Ready);
        app_state
            .feed_update_state_sender_map
            .write()
            .await
            .insert(id.clone(), state_sender);
        app_state
            .feed_update_state_map
            .write()
            .await
            .insert(id.clone(), state_receiver);
    }
    app_state
        .feed_update_queues
        .get(&id.community)
        .expect("community not found")
        .send(id)?;

    Ok(true)
}

/// Set up before server started
pub fn listen_feed_update(pool: DatabasePool, state_sender_map: FeedUpdateJobStateSenderMap) -> FeedUpdateJobQueue {
    // (1) MPSC unbounded channel: job queue
    // Allow only one job per community to avoid rate limiting
    let (job_sender, mut job_receiver) = mpsc::unbounded_channel::<FeedIdentifier>();

    task::spawn(async move {
        while let Some(id) = job_receiver.recv().await {
            let state_sender = state_sender_map
                .read()
                .await
                .get(&id)
                .expect("job state sender not found")
                .clone();

            let result = update_feed(pool.clone(), &id, state_sender.clone(), DEFAULT_DELAY_MS).await;

            if let Err(e) = result {
                tracing::error!("Feed update job failed: {}. {}", id, e);
                let _ = state_sender.send(FeedUpdateJobState::Failed { error: e.to_string() });
            }
        }
    });

    job_sender
}

async fn update_feed(
    pool: DatabasePool,
    id: &FeedIdentifier,
    state_sender: FeedUpdateJobStateSender,
    delay_ms: u64,
) -> Result<()> {
    // 1. Prepare the feed
    let (feed, mut context) = {
        let db = &mut pool.get().expect("cannot access database");
        let feed = FeedWrapper::from_id(db, id)?;

        // 2. Handle before update
        feed.handle_before_update(db)?;

        // 3. Refresh the account if necessary
        feed.refresh_account(db).await?;
        let context = feed.get_context(db)?;
        (feed, context)
    };

    // 4. Fetch and save the feed
    let mut fetched = 0;
    let mut results = Vec::new();
    tracing::info!("Feed update job started: {}", id);
    loop {
        let (result, new_context) =
            util::retry(|| util::timeout(update_feed_inner(pool.clone(), &feed, &context))).await?;
        context = new_context;

        let post_count = result.post_ids.len() as u64;
        fetched += post_count;
        tracing::info!("Feed {} updated {} posts", feed.id(), post_count);
        state_sender.send(FeedUpdateJobState::Running { fetched })?;

        let should_stop = result.should_stop;
        results.push(result);
        if should_stop {
            break;
        }

        time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // 5. Handle after update
    {
        let db = &mut pool.get().expect("cannot access database");
        feed.handle_after_update(db, results.iter())?;
    }

    tracing::info!("Feed update job done: {}. Updated {} posts", id, fetched);
    state_sender.send(FeedUpdateJobState::Success { fetched })?;
    Ok(())
}

async fn update_feed_inner(
    pool: DatabasePool,
    feed: &FeedWrapper,
    context: &FeedContextWrapper,
) -> Result<(SaveResult, FeedContextWrapper)> {
    let db = &mut pool.get().expect("cannot access database");
    let mut context = context.clone();
    let result = feed.fetch_and_save(db, &mut context).await?;
    Ok((result, context))
}
