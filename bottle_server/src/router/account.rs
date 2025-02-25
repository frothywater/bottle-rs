use axum::{
    extract::{Path, State},
    response::Json,
    routing::get,
    Router,
};

use bottle_core::feed::{Account, AccountView};
use bottle_panda::PandaAccount;
use bottle_pixiv::PixivAccount;
use bottle_twitter::TwitterAccount;
use bottle_yandere::YandereAccount;

use crate::{error::Result, state::AppState};

pub fn account_router() -> Router<AppState> {
    Router::new()
        .route("/:community/accounts", get(get_accounts))
        .route("/:community/account/:id", get(get_account))
}

async fn get_accounts(
    State(app_state): State<AppState>,
    Path(community): Path<String>,
) -> Result<Json<Vec<AccountView>>> {
    let db = &mut app_state.pool.get()?;
    let accounts = match community.as_str() {
        "twitter" => TwitterAccount::all(db)?
            .into_iter()
            .map(|a| a.view())
            .collect::<Vec<_>>(),
        "pixiv" => PixivAccount::all(db)?.into_iter().map(|a| a.view()).collect::<Vec<_>>(),
        "yandere" => YandereAccount::all(db)?
            .into_iter()
            .map(|a| a.view())
            .collect::<Vec<_>>(),
        "panda" => PandaAccount::all(db)?.into_iter().map(|a| a.view()).collect::<Vec<_>>(),
        _ => return Err(bottle_core::Error::ObjectNotFound(format!("Community {}", community)))?,
    };

    Ok(Json(accounts))
}

async fn get_account(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
) -> Result<Json<AccountView>> {
    let db = &mut app_state.pool.get()?;
    let account = match community.as_str() {
        "twitter" => TwitterAccount::get(db, id)?.map(|a| a.view()),
        "pixiv" => PixivAccount::get(db, id)?.map(|a| a.view()),
        "yandere" => YandereAccount::get(db, id)?.map(|a| a.view()),
        "panda" => PandaAccount::get(db, id)?.map(|a| a.view()),
        _ => None,
    }
    .ok_or(bottle_core::Error::ObjectNotFound(format!(
        "Account {} at Community {}",
        id, community
    )))?;

    Ok(Json(account))
}
