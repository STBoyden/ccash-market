use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) type UserUID = Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    username: String,
    offer_ids: Vec<Uuid>,
}

impl User {
    pub(crate) fn new(username: &str) -> Self {
        Self {
            username: username.into(),
            offer_ids: vec![],
        }
    }

    pub(crate) fn get_username(&self) -> &str { &self.username }
}
