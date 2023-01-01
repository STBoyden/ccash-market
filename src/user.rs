use crate::offer::OfferUID;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UserUID(pub Uuid);

impl fmt::Display for UserUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    username: String,
    offer_ids: Vec<OfferUID>,
}

impl User {
    pub(crate) fn new(username: &str) -> Self {
        Self {
            username: username.into(),
            offer_ids: vec![],
        }
    }

    pub(crate) fn get_username(&self) -> &str { &self.username }

    pub(crate) fn add_offer_id(&mut self, offer_id: OfferUID) {
        self.offer_ids.insert(0, offer_id);
    }

    pub(crate) fn get_offer_ids(&self) -> &[OfferUID] { &self.offer_ids }
}
