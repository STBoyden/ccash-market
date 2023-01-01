use crate::{
    routes::{
        create_ask, create_bid, get_asks, get_asks_for_user, get_bids, get_bids_for_user,
        get_commodity_from_id, get_offers, get_offers_for_user, get_user_from_id,
        get_users, properties,
    },
    state::GState,
};
use axum::{
    // error_handling::HandleErrorLayer,
    extract::State,
    http::{self, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{Redirect, Response},
    routing::{get, post, IntoMakeService},
    BoxError,
};
use ccash_rs::{methods as m, CCashSession, CCashUser};
use parking_lot::RwLock;
use std::sync::Arc;
// use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub(crate) struct Router {
    inner: axum::Router<GState>,
    ccash_session: Arc<RwLock<Option<CCashSession>>>,
    ccash_uri: Option<String>,
    state: GState,
}

impl Router {
    pub(crate) fn new(state: GState, ccash_uri: Option<&String>) -> Self {
        let ccash_uri = ccash_uri.cloned();

        Router {
            inner: axum::Router::new(),
            ccash_session: Arc::new(RwLock::new(None)),
            ccash_uri,
            state,
        }
    }

    #[allow(clippy::type_complexity)]
    async fn auth<B>(
        State((ccash_session, ccash_uri)): State<(
            Arc<RwLock<Option<CCashSession>>>,
            Option<String>,
        )>,
        mut req: Request<B>,
        next: Next<B>,
    ) -> Result<Response, StatusCode> {
        let auth_header = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        let Some(auth_header) = auth_header else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        if !auth_header.starts_with("Basic ") {
            return Err(StatusCode::EXPECTATION_FAILED);
        }

        let auth_header = auth_header.replace("Basic ", "");

        let Ok(decoded) = base64::decode(auth_header) else {
            return Err(StatusCode::EXPECTATION_FAILED);
        };

        let Ok(decoded) =  String::from_utf8(decoded) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let splits = decoded.split(':').collect::<Vec<&str>>();
        let Some((username, password)) = splits.first().zip(splits.last()) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Ok(user) = CCashUser::new(username, password) else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        if let Some(ccash_uri) = ccash_uri {
            if ccash_session.read().is_none() {
                let mut session = CCashSession::new(&ccash_uri);

                if session.establish_connection().await.is_err() {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }

                *ccash_session.write() = Some(session.clone());
            }

            let session = ccash_session.read().clone().unwrap();

            if let Ok(contains) = m::contains_user(&session, &user).await {
                if !contains {
                    return Err(StatusCode::NOT_FOUND);
                }
            }

            if let Ok(correct) = m::verify_password(&session, &user).await {
                if !correct {
                    return Err(StatusCode::NOT_FOUND);
                }
            } else {
                return Err(StatusCode::NOT_FOUND);
            }
        }

        req.extensions_mut().insert(user);
        Ok(next.run(req).await)
    }

    #[allow(dead_code)]
    async fn error_handler(
        _method: Method,
        _uri: Uri,
        error: BoxError,
    ) -> (StatusCode, String) {
        dbg!(&error);

        (StatusCode::NOT_ACCEPTABLE, format!("{error}"))
    }

    fn v1_routes(&self) -> axum::Router<GState> {
        axum::Router::new()
            .route("/get/users", get(get_users))
            .route("/get/asks", get(get_asks))
            .route("/get/asks/:username", get(get_asks_for_user))
            .route("/get/bids", get(get_bids))
            .route("/get/bids/:username", get(get_bids_for_user))
            .route("/get/offers", get(get_offers))
            .route("/get/offers/:username", get(get_offers_for_user))
            .route("/get/user/:id", get(get_user_from_id))
            .route("/get/commodity/:id", get(get_commodity_from_id))
            .merge(
                axum::Router::new()
                    .route("/create/ask", post(create_ask))
                    .route("/create/bid", post(create_bid))
                    .route_layer(middleware::from_fn_with_state(
                        (self.ccash_session.clone(), self.ccash_uri.clone()),
                        Self::auth,
                    ))
                    .with_state(Arc::clone(&self.state)),
            )
            .with_state(Arc::clone(&self.state))
    }

    fn api_routes(&self) -> axum::Router<GState> {
        axum::Router::new()
            .nest("/v1", self.v1_routes())
            .route("/properties", get(properties))
            .route(
                "/help",
                get(|| async {
                    Redirect::permanent("https://github.com/STBoyden/ccash-market")
                }),
            )
            .with_state(Arc::clone(&self.state))
    }

    async fn not_found() -> &'static str {
        "Route not found. Please use \"/help\" for help with routes."
    }

    pub(crate) fn build(self) -> IntoMakeService<axum::Router> {
        let api_routes = self.api_routes();

        self.inner
            .nest("/api", api_routes)
            .route("/", get(|| async { Redirect::permanent("/api/help") }))
            .layer(TraceLayer::new_for_http())
            // .layer(
            //     ServiceBuilder::new()
            //         .layer(HandleErrorLayer::new(Self::error_handler).into()),
            // )
            .fallback(Self::not_found)
            .with_state(self.state)
            .into_make_service()
    }
}
