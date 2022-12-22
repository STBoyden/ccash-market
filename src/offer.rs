use crate::commodity::Commodity;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub enum Offer {
    Bid {
        account_name: String,
        item: Commodity,
        amount: u32,
    },
    Ask {
        account_name: String,
        item: Commodity,
        amount: u32,
    },
}
