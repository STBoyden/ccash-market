mod ask;
mod bid;

pub use ask::*;
pub use bid::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferResponse {
    pub message: String,
}
