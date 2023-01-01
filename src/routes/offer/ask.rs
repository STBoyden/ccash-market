use super::{
    offer_sort, OfferQueryParams, OfferResponse, OfferSortBy, MAX_OFFER_RESPONSE,
};
use crate::{offer::Offer, state::GState};
use axum::{
    extract::{Path, Query, State},
    response::Result,
    Extension, Json,
};
use ccash_rs::CCashUser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAsk {
    pub commodity_name: String,
    pub total_cost: u64,
    pub cost_per_item: u64,
}

pub async fn create_ask(
    Extension(ccash_user): Extension<CCashUser>,
    State(state): State<GState>,
    Json(CreateAsk {
        commodity_name,
        total_cost,
        cost_per_item,
    }): Json<CreateAsk>,
) -> Result<Json<OfferResponse>, Json<Value>> {
    let Some(total) = total_cost.checked_div(cost_per_item) else {
        return Err(Json(json!("cost_per_item or total_cost cannot be 0")));
    };

    let user_id = state.write().get_or_add_user(&ccash_user);
    let commodity_id =
        state
            .write()
            .get_or_add_commodity(&commodity_name, total, user_id);

    let ask_id = state
        .write()
        .add_ask(commodity_id, user_id, total, cost_per_item);

    if let Some(user) = state.write().get_users_mut().get_mut(&user_id) {
        user.write().add_offer_id(ask_id);
    }

    if let Some(commodity) = state.write().get_commodities_mut().get_mut(&commodity_id) {
        commodity.write().add_owner_id(user_id);
    }

    Ok(Json(OfferResponse {
        message: format!(
            "Ask for {total} \"{commodity_name}\" item(s) at {cost_per_item} CSH each \
             by {}",
            ccash_user.get_username()
        ),
    }))
}

pub async fn get_asks(
    params: Option<Query<OfferQueryParams>>,
    State(state): State<GState>,
) -> Result<Json<Vec<Offer>>, Json<Value>> {
    let Query(OfferQueryParams { limit, sort_by }) = params.unwrap_or_default();
    let mut limit = limit.unwrap_or(100);
    let sort_by = sort_by.unwrap_or(OfferSortBy::DateDescending);

    if limit == 0 || limit > MAX_OFFER_RESPONSE {
        limit = MAX_OFFER_RESPONSE;
    }

    let state = state.read();

    let offer_ids = state.get_offers();
    let mut asks = offer_ids
        .iter()
        .filter(|kv| {
            let v = kv.value().clone();
            let v = v.read();

            matches!(v.clone(), Offer::Ask { .. })
        })
        .map(|kv| kv.value().read().clone())
        .collect::<Vec<_>>();

    asks.sort_by(|a, b| offer_sort(sort_by.clone(), a, b));

    let asks = asks.iter().take(limit).cloned().collect::<Vec<_>>();

    Ok(Json(asks))
}

pub async fn get_asks_for_user(
    params: Option<Query<OfferQueryParams>>,
    State(state): State<GState>,
    Path(username): Path<String>,
) -> Result<Json<Vec<Offer>>, Json<Value>> {
    let Query(OfferQueryParams { limit, sort_by }) = params.unwrap_or_default();
    let mut limit = limit.unwrap_or(100);
    let sort_by = sort_by.unwrap_or(OfferSortBy::DateDescending);

    if limit == 0 || limit > MAX_OFFER_RESPONSE {
        limit = MAX_OFFER_RESPONSE;
    }

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
        let mut asks = offer_ids
            .iter()
            .filter(|offer_id| {
                if let Some(kv) = state.get_offers().get(offer_id) {
                    let v = kv.value().clone();

                    return matches!(v.read().clone(), Offer::Ask { .. });
                }

                false
            })
            .filter_map(|ask_id| state.get_offers().get(ask_id))
            .map(|kv| kv.value().read().clone())
            .collect::<Vec<_>>();

        asks.sort_by(|a, b| offer_sort(sort_by.clone(), a, b));

        let asks = asks.iter().take(limit).cloned().collect::<Vec<_>>();

        return Ok(Json(asks));
    }

    Err(Json(json!({}))) // TODO
}
