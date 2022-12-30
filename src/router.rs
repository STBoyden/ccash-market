use crate::{
    routes::{create_ask, create_bid, get_users, properties},
    state::GState,
};
use axum::{
    response::Redirect,
    routing::{get, post, IntoMakeService},
};

pub(crate) struct Router {
    inner: axum::Router<GState>,
    state: GState,
}

impl Router {
    pub(crate) fn new(state: GState) -> Self {
        Router {
            inner: axum::Router::new(),
            state,
        }
    }

    async fn error() -> &'static str {
        "Route not found. Please use \"/help\" for help with routes."
    }

    pub(crate) fn build(self) -> IntoMakeService<axum::Router> {
        self.inner
            .route("/api/properties", get(properties))
            .route("/api/v1/get_users", get(get_users))
            .route("/api/v1/create_ask", post(create_ask))
            .route("/api/v1/create_bid", post(create_bid))
            .route(
                "/help",
                get(|| async {
                    Redirect::permanent("https://github.com/STBoyden/ccash-market")
                }),
            )
            .fallback(Self::error)
            .with_state(self.state)
            .into_make_service()
    }
}
