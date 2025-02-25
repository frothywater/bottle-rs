use serde::Serialize;

use super::download::ImageDownloadJobStateResponse;
use super::feed::FeedUpdateJobStateResponse;
use super::panda::PandaDownloadJobStateResponse;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GeneralJobState {
    #[default]
    Ready,
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobsStateResponse {
    pub feed_update_jobs: Vec<FeedUpdateJobStateResponse>,
    pub image_download_job: ImageDownloadJobStateResponse,
    pub panda_download_jobs: Vec<PandaDownloadJobStateResponse>,
}
