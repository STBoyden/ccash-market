use crate::{commodity::CommodityUID, user::UserUID};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct OfferUID(pub Uuid);

impl fmt::Display for OfferUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Offer {
    Ask {
        user_id: UserUID,
        commodity_id: CommodityUID,
        datetime: DateTime<Utc>,
        item_amount: u64,
        price_per_item: u64,
    },
    Bid {
        user_id: UserUID,
        commodity_id: CommodityUID,
        datetime: DateTime<Utc>,
        item_amount: u64,
        price_per_item: u64,
    },
}
