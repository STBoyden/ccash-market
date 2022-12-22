use crate::{commodity::CommodityUID, user::UserUID};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) type OfferUID = Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub(crate) enum Offer {
    Ask {
        user_id: UserUID,
        commodity_id: CommodityUID,
        item_amount: u64,
        price_per_item: u64,
    },
    Bid {
        user_id: UserUID,
        commodity_id: CommodityUID,
        item_amount: u64,
        price_per_item: u64,
    },
}
