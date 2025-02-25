mod background_job;
mod error;
mod payload;
mod router;
mod state;
mod util;

use axum::Router;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use tokio::sync::{mpsc, RwLock};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use util::ConnectionOptions;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bottle_panda::PandaCache;
use bottle_pixiv::PixivCache;
use bottle_twitter::TwitterCache;
use bottle_yandere::YandereCache;

use crate::{state::AppState, util::FeedIdentifier};

#[tokio::main]
async fn main() {
    dotenv().ok();

    // 1. Initialize logger
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .unwrap()
        .add_directive("hyper::proto=info".parse().unwrap())
        .add_directive("hyper::client=info".parse().unwrap())
        .add_directive("reqwest=info".parse().unwrap())
        .add_directive("html5ever=info".parse().unwrap())
        .add_directive("selectors=info".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    // 2. Initialize database
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(16)
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(30)),
        }))
        .build(manager)
        .unwrap();

    // 3. Initialize static file server
    let image_dir = env::var("IMAGE_DIR").expect("IMAGE_DIR must be set");
    let image_dir = PathBuf::from(image_dir)
        .canonicalize()
        .expect("IMAGE_DIR must be a valid path");
    let serve_dir = ServeDir::new(&image_dir);

    // 4. Initialize cache
    let twitter_cache = Arc::new(RwLock::new(TwitterCache::new()));
    let pixiv_cache = Arc::new(RwLock::new(PixivCache::new()));
    let yandere_cache = Arc::new(RwLock::new(YandereCache::new()));
    let panda_cache = Arc::new(RwLock::new(PandaCache::new()));

    // 5. Initialize background jobs
    let feed_update_state_sender_map = Arc::new(RwLock::new(HashMap::new()));
    let feed_update_state_map = Arc::new(RwLock::new(HashMap::new()));
    let feed_update_queue = |community: &str| -> (String, mpsc::UnboundedSender<FeedIdentifier>) {
        (
            community.to_string(),
            background_job::listen_feed_update(pool.clone(), feed_update_state_sender_map.clone()),
        )
    };
    let feed_update_queues = HashMap::from([
        feed_update_queue("twitter"),
        feed_update_queue("pixiv"),
        feed_update_queue("yandere"),
        feed_update_queue("panda"),
    ]);

    let (image_download_queue, image_download_job_state) =
        background_job::listen_image_download(pool.clone(), &image_dir);

    let panda_download_state_sender_map = Arc::new(RwLock::new(HashMap::new()));
    let panda_download_state_map = Arc::new(RwLock::new(HashMap::new()));
    let panda_download_queue =
        background_job::listen_panda_download(pool.clone(), panda_download_state_sender_map.clone(), &image_dir)
            .expect("cannot start panda download job");
    let panda_gallery_title_map = Arc::new(RwLock::new(HashMap::new()));

    // 6. Setup state and router
    let app_state = AppState {
        pool,
        twitter_cache,
        pixiv_cache,
        yandere_cache,
        panda_cache,
        feed_update_queues,
        feed_update_state_sender_map,
        feed_update_state_map,
        image_download_queue,
        image_download_job_state,
        panda_download_queue,
        panda_download_state_sender_map,
        panda_download_state_map,
        panda_gallery_title_map,
    };

    let app = Router::new()
        .merge(router::account::account_router())
        .merge(router::feed::feed_router())
        .merge(router::work::work_router())
        .merge(router::library::library_router())
        .merge(router::api::api_router())
        .merge(router::job::job_router())
        .nest_service("/image", serve_dir)
        .layer(TraceLayer::new_for_http().on_request(()))
        .with_state(app_state);

    // 7. Start server
    let addr = env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS must be set");
    tracing::info!("Server starting at {}", addr);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
