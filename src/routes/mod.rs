use crate::state::{AppProperties, AppState};
use axum::{extract::State, Json};

pub async fn properties(State(state): State<AppState>) -> Json<AppProperties> {
    Json(state.as_properties())
}
