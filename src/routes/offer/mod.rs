mod ask;
mod bid;

pub use ask::*;
pub use bid::*;

use crate::{offer::Offer, state::GState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferResponse {
    pub message: String,
}

pub async fn get_offers_for_user(
    State(state): State<GState>,
    Path(username): Path<String>,
) -> Result<Json<Vec<Offer>>, Json<Value>> {
    let state = state.read();
    let users = state.get_users();

    let mut users_filtered = users.iter().filter(|kv| {
        let v = kv.value();
        v.read().get_username() == username
    });

    if let Some(kv) = users_filtered.next() {
        let v = kv.value().clone();
        let v = v.read();

        let offer_ids = v.get_offer_ids();
        let offers = offer_ids
            .iter()
            .filter_map(|id| state.get_offers().get(id))
            .map(|kv| kv.value().clone())
            .map(|value| value.read().clone())
            .collect::<Vec<_>>();

        return Ok(Json(offers));
    }

    Err(Json(json!({})))
}
