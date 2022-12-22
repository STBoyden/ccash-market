use crate::{routes::properties, state::GState};
use axum::routing::{get, IntoMakeService};

pub struct Router {
    inner: axum::Router<GState>,
    state: GState,
}

impl Router {
    pub fn new(state: GState) -> Self {
        Router {
            inner: axum::Router::new(),
            state,
        }
    }

    async fn error() -> &'static str {
        "Route not found. Please use \"/help\" for help with routes."
    }

    pub fn build(self) -> IntoMakeService<axum::Router> {
        self.inner
            .route("/api/properties", get(properties))
            .fallback(Self::error)
            .with_state(self.state)
            .into_make_service()
    }
}
