use super::OfferResponse;
use crate::state::GState;
use axum::{extract::State, response::Result, Json};
use ccash_rs::CCashUser;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBid {
    #[serde(flatten)]
    pub ccash_user: CCashUser,
    pub commodity_name: String,
    pub total_cost: u64,
    pub cost_per_item: u64,
}

pub async fn create_bid(
    State(state): State<GState>,
    Json(CreateBid {
        ccash_user,
        commodity_name,
        total_cost,
        cost_per_item,
    }): Json<CreateBid>,
) -> Result<Json<OfferResponse>, Json<Value>> {
    let user_id = state.write().get_or_add_user(&ccash_user);
    let commodity_id = state.write().get_or_add_commodity(
        &commodity_name,
        cost_per_item / total_cost,
        user_id,
    );

    let bid_id = state.write().add_bid(
        commodity_id,
        user_id,
        cost_per_item / total_cost,
        cost_per_item,
    );

    if let Some(user) = state.write().get_users_mut().get_mut(&user_id) {
        user.write().add_offer_id(bid_id);
    }

    if let Some(commodity) = state.write().get_commodities_mut().get_mut(&commodity_id) {
        commodity.write().add_owner_id(user_id);
    }

    Ok(Json(OfferResponse {
        message: format!(
            "Bid for {} \"{commodity_name}\" item(s) at {cost_per_item} CSH each by {}",
            cost_per_item / total_cost,
            ccash_user.get_username()
        ),
    }))
}
