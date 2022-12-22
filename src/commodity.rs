use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Commodity {
    name: String,
    value: u32,
    amount: u32,
}
