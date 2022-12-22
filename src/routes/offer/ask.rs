use crate::state::GState;
use axum::{extract::State, response::Result, Json};
use ccash_rs::CCashUser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAsk {
    pub ccash_user: CCashUser,
}

pub async fn create_ask(
    State(_state): State<GState>,
    Json(CreateAsk { ccash_user, .. }): Json<CreateAsk>,
) -> Result<Json<Value>, Json<Value>> {
    Ok(Json(json!({ "msg": format!("{ccash_user:#?}") })))
}
