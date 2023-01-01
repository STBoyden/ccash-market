use crate::{
    commodity::{Commodity, CommodityUID},
    state::GState,
    user::{User, UserUID},
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;
use uuid::Uuid;

pub async fn get_user_from_id(
    Path(id): Path<Uuid>,
    State(state): State<GState>,
) -> Result<Json<User>, Json<Value>> {
    let state = state.read();
    let Some(kv) = state.get_users().get(&UserUID(id)) else {
        // let response = Response::builder()
        //     .status(StatusCode::NOT_FOUND)
        //     .body(
        //         Json(json!({"message" : format!("ID \"{id}\" not found")}))
        //     ).unwrap();

        // return Err(response);

        todo!()
    };

    let v = kv.value().clone();
    let v = v.read().clone();

    Ok(Json(v))
}

pub async fn get_commodity_from_id(
    Path(id): Path<Uuid>,
    State(state): State<GState>,
) -> Result<Json<Commodity>, Json<Value>> {
    let state = state.read();
    let Some(kv) = state.get_commodities().get(&CommodityUID(id)) else {
        todo!()
    };

    let v = kv.value().clone();
    let v = v.read().clone();

    Ok(Json(v))
}
