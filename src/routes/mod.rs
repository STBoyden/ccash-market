use crate::state::{AppProperties, GState};
use axum::{extract::State, Json};

pub async fn properties(State(state): State<GState>) -> Json<AppProperties> {
    Json(state.read().as_properties())
}
