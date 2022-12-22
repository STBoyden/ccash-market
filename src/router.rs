use axum::routing::{get, IntoMakeService};

use crate::{routes::properties, state::AppState};

pub struct Router {
    inner: axum::Router<AppState>,
    state: AppState,
}

impl Router {
    pub fn new(state: AppState) -> Self {
        Router {
            inner: axum::Router::new(),
            state,
        }
    }

    pub fn build(self) -> IntoMakeService<axum::Router> {
        self.inner
            .route("/properties", get(properties))
            .with_state(self.state)
            .into_make_service()
    }
}
