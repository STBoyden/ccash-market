mod offer;
mod util;

use crate::state::{AppProperties, GState, Users};
use axum::{extract::State, Json};
pub use offer::*;
pub use util::*;

pub async fn properties(State(state): State<GState>) -> Json<AppProperties> {
    Json(state.read().as_properties())
}

pub async fn get_users(State(state): State<GState>) -> Json<Users> {
    Json(state.read().get_users().clone())
}
