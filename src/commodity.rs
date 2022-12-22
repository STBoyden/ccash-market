use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::user::UserUID;

pub type CommodityUID = Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Commodity {
    name: String,
    size: usize,
    owners: Vec<UserUID>,
}

impl Commodity {
    pub(crate) fn get_owner_ids(&self) -> &[UserUID] { &self.owners }
}
