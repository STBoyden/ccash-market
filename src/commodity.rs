use crate::user::UserUID;
use dashmap::DashSet;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct CommodityUID(pub Uuid);

impl fmt::Display for CommodityUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Commodity {
    name: String,
    size: u64,
    owners: DashSet<UserUID>,
}

impl Commodity {
    pub(crate) fn new(
        name: &str,
        initial_amount: Option<u64>,
        owner_ids: Option<DashSet<UserUID>>,
    ) -> Self {
        let size = initial_amount.unwrap_or(0);
        let owners = owner_ids.unwrap_or(DashSet::new());

        Self {
            name: name.to_owned(),
            size,
            owners,
        }
    }

    pub(crate) fn get_owner_ids(&self) -> Vec<UserUID> {
        self.owners
            .iter()
            .map(|item| *item.key())
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_name(&self) -> &str { &self.name }

    pub(crate) fn add_owner_id(&mut self, user_id: UserUID) {
        self.owners.insert(user_id);
    }
}
