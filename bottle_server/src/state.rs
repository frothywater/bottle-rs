use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use tokio::sync::RwLock;

use std::collections::HashMap;
use std::sync::Arc;

use bottle_panda::PandaCache;
use bottle_pixiv::PixivCache;
use bottle_twitter::TwitterCache;
use bottle_yandere::YandereCache;

use crate::background_job::*;

pub type DatabasePool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: DatabasePool,

    /// Cache for community entities fetched from APIs
    pub twitter_cache: Arc<RwLock<TwitterCache>>,
    pub pixiv_cache: Arc<RwLock<PixivCache>>,
    pub yandere_cache: Arc<RwLock<YandereCache>>,
    pub panda_cache: Arc<RwLock<PandaCache>>,

    // Background job queues and state channels
    /// Feed update job queues: community -> job sender
    pub feed_update_queues: HashMap<String, FeedUpdateJobQueue>,
    /// Feed update job state: feed -> state sender
    pub feed_update_state_sender_map: FeedUpdateJobStateSenderMap,
    /// Feed update job state: feed -> state receiver
    pub feed_update_state_map: FeedUpdateJobStateReceiverMap,

    /// Image download job queue
    pub image_download_queue: ImageDownloadJobQueue,
    /// Image download job state
    pub image_download_job_state: ImageDownloadJobStateReceiver,

    /// Panda download job queue
    pub panda_download_queue: PandaDownloadJobQueue,
    /// Panda download job state: gallery -> state sender
    pub panda_download_state_sender_map: PandaDownloadJobStateSenderMap,
    /// Panda download job state: gallery -> state receiver
    pub panda_download_state_map: PandaDownloadJobStateReceiverMap,
    pub panda_gallery_title_map: Arc<RwLock<HashMap<PandaGalleryID, String>>>,
}
