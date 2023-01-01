mod ask;
mod bid;

use std::cmp::Ordering;

pub use ask::*;
pub use bid::*;

use crate::{offer::Offer, state::GState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_OFFER_RESPONSE: usize = 1000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", untagged)]
pub enum OfferSortBy {
    DateAscending,
    DateDescending,
    TotalCostAscending,
    TotalCostDescending,
}

impl Default for OfferSortBy {
    fn default() -> Self { Self::DateDescending }
}

pub(crate) fn offer_sort(sort_by: OfferSortBy, a: &Offer, b: &Offer) -> Ordering {
    match (sort_by, a, b) {
        (
            OfferSortBy::DateDescending,
            Offer::Ask { datetime: a, .. } | Offer::Bid { datetime: a, .. },
            Offer::Ask { datetime: b, .. } | Offer::Bid { datetime: b, .. },
        ) => b.cmp(a),
        (
            OfferSortBy::DateAscending,
            Offer::Ask { datetime: a, .. } | Offer::Bid { datetime: a, .. },
            Offer::Ask { datetime: b, .. } | Offer::Bid { datetime: b, .. },
        ) => a.cmp(b),
        (
            OfferSortBy::TotalCostDescending,
            Offer::Ask {
                price_per_item: a_ppi,
                item_amount: a_ta,
                ..
            }
            | Offer::Bid {
                price_per_item: a_ppi,
                item_amount: a_ta,
                ..
            },
            Offer::Ask {
                price_per_item: b_ppi,
                item_amount: b_ta,
                ..
            }
            | Offer::Bid {
                price_per_item: b_ppi,
                item_amount: b_ta,
                ..
            },
        ) => dbg!((b_ta.saturating_mul(*b_ppi)).cmp(&a_ta.saturating_mul(*a_ppi))),
        (
            OfferSortBy::TotalCostAscending,
            Offer::Ask {
                price_per_item: a_ppi,
                item_amount: a_ta,
                ..
            }
            | Offer::Bid {
                price_per_item: a_ppi,
                item_amount: a_ta,
                ..
            },
            Offer::Ask {
                price_per_item: b_ppi,
                item_amount: b_ta,
                ..
            }
            | Offer::Bid {
                price_per_item: b_ppi,
                item_amount: b_ta,
                ..
            },
        ) => dbg!((a_ta.saturating_mul(*a_ppi)).cmp(&b_ta.saturating_mul(*b_ppi))),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferQueryParams {
    pub limit: Option<usize>,
    #[serde(flatten)]
    pub sort_by: Option<OfferSortBy>,
}

impl Default for OfferQueryParams {
    fn default() -> Self {
        Self {
            limit: Some(100),
            sort_by: Some(OfferSortBy::DateDescending),
        }
    }
}

pub async fn get_offers_for_user(
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
        let mut offers = offer_ids
            .iter()
            .filter_map(|id| state.get_offers().get(id))
            .map(|kv| kv.value().clone())
            .map(|value| value.read().clone())
            .collect::<Vec<_>>();

        offers.sort_by(|a, b| offer_sort(sort_by.clone(), a, b));

        let offers = offers.iter().take(limit).cloned().collect::<Vec<_>>();

        return Ok(Json(offers));
    }

    Err(Json(json!({})))
}
