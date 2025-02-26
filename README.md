# Bottle.rs

A web server application that integrates multiple illustration social platforms (Twitter, Pixiv, etc.) into a unified feed management system. Serves as the backend of the Bottle project. The frontend is at [`Bottle.app`](https://github.com/frothywater/Bottle).

## Features

- **Feeds**: Integrate multiple illustration platforms using a unified feed interface. User can subscribe to different types of feeds (timeline, bookmarks, etc.) from different platforms. Bottle tracks them and updates new contents. Authentication logic for different platforms are supported by an account interface.
- **Library**: Manage favorite illustration by adding them from feeds to the library, and organizes them with albums and folders. Use work and image interface to represent library entities with downloadable resources.
- **Artist View**: Browse feed posts and library works grouped by artist to appreciate unique styles of different artists.
- **Local Collection**: Built in local-first principle, all feeds and library content are stored in a SQLite database with rich metadata. Images in the library are downloaded and served locally instead of the original source, preventing the infamous link rot.
- **Background Jobs**: Update feeds and download favorite images in the background, backed up by a robust job queue system implemented with [Tokio](https://tokio.rs) [`mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html) (multi-producer, single-consumer) and [`watch`](https://docs.rs/tokio/latest/tokio/sync/watch/index.html) (multi-producer, multi-consumer) channels. User can monitor each job's state anytime. Supports concurrent processing with configurable limits.
- **Thumbnails**: Create thumbnails for downloaded images automatically and serves them for faster browsing.
- **Caching**: Store API responses in a temporary cache for later use.

## Modules

The codes are organized as a Cargo workspace consisting of several modules:
- `bottle_server`: Main server implementation, including router, API endpoint handlers, state management, static file server and background job queue system.
- `bottle_core`: Core traits and interfaces, including `Community`, `Account`, `Feed`, `Post` and their related entities. Also defines all request and response payload types, database entity schema and error types.
- `bottle_library`: Library related codes, including operations on works, images, albums and folders. Also responsible for preparing images to download and updating information after that.
- `bottle_download`: Image downloading and thumbnail conversion.
- `bottle_util`: Utility codes, including `diesel` extensions, internally used macros and some helpers for parsing and serialization.
- `bottle_*(community)`: Platform-specific codes for main feed functionalities, implementing the core traits defined in `bottle_core`. Includes operations of fetching content from client, persisting to database, sending response to frontend, supporting artist view and managing cache. Responsible for converting entities between different formats such as those in database, in API response and in server response. Defines related database model structs.
- `*(community)_client`: API client/HTML scraper for each illustration community service.

## Database Schema

Key Tables:
- `work`: Saved posts in the library with community metadata and user-added information.
- `image`: Saved images with optional local paths.
- `album`: Albums consisting selected works.
- `folder`: Folders to organize albums.
- Platform-specific tables:
  - `twitter_account`: Twitter account with credentials.
  - `tweet`: Tweet metadata.
  - `twitter_media`: Tweet media metadata.
  - `twitter_user`: Twitter user metadata.
  - `twitter_watch_list`: Feeds subscribed to Twitter.
  - `twitter_watch_list_history`: Metadata for each feed update, optionally containing essential information for the next update.
- etc.

## Deployment
The application can be deployed as a light-weight stand-alone linux binary of a few MBs via the Docker environment [clux/muslrust](https://github.com/clux/muslrust). The environment variables to be set are the following:
```env
SERVER_ADDRESS=0.0.0.0:6000
DATABASE_URL=path/to/db.sqlite
IMAGE_DIR=/path/to/images
CLIENT_LOG_DIR=/path/to/logs
```

## Dependencies
- [`axum`](https://docs.rs/axum/latest/axum/): Web server framework for handling HTTP requests.
- [`diesel`](https://diesel.rs): ORM for SQLite database interactions.
- [`tokio`](https://tokio.rs): Async runtime for concurrent operations and the background queue.
- [`reqwest`](https://docs.rs/reqwest/latest/reqwest/): HTTP client for API operations and HTML scraping.
- [`serde`](https://serde.rs): Serialization framework for parsing JSON API responses.
- [`scraper`](https://docs.rs/scraper/latest/scraper/): HTML parsing library for extracting needed information with CSS selectors.
- [`image`](https://docs.rs/image/latest/image/): Image encoding/decoding and manipulations.
- etc.

## Endpoints
```
GET /metadata
GET /:community/accounts
GET /:community/account/:id

POST /feed
GET /:community/feeds
GET /:community/feed/:id
DELETE /:community/feed/:id
POST /:community/feed/:id
GET /:community/feed/:id/posts
GET /:community/feed/:id/users
GET /:community/feed/:id/user/:user_id
GET /:community/feeds/update
GET /:community/feed/:id/update

GET /:community/works
POST /:community/post/:id/work
DELETE /work/:id
GET /:community/work/users
GET /:community/work/user/:user_id

GET /jobs
GET /images/download

POST /album
GET /albums
POST /album/:id/rename
POST /album/:id/reorder
DELETE /album/:id
POST /album/:id/works
GET /album/:id/works
DELETE /album/:id/works
POST /folder
GET /folders
POST /folder/:id/rename
POST /folder/:id/reorder
DELETE /folder/:id

POST /twitter/api
POST /pixiv/api
POST /yandere/api
POST /panda/api
GET /panda/api/post/:gid
GET /panda/api/post/:gid/media/:page
GET /panda/galleries/download
GET /panda/gallery/:id/download
```
