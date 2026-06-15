//! LimitBook gateway library: the matching-engine wrapper's HTTP/WebSocket surface.
//!
//! The binary ([`main.rs`](../main.rs)) is a thin bootstrap over [`build_router`] and
//! [`state::spawn_drain`]; keeping the wiring here makes it testable in-process.

pub mod dto;
pub mod metrics;
pub mod routes;
pub mod sim;
pub mod state;
pub mod ws;

use axum::http::{HeaderValue, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// Build the Axum router with all routes, CORS restricted to localhost dev origins, and tracing.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(routes::health))
        .route("/orders", post(routes::submit_order))
        .route("/orders/{id}", delete(routes::cancel_order))
        .route("/orders/{id}", put(routes::amend_order))
        .route("/book", get(routes::get_book))
        .route("/trades", get(routes::get_trades))
        .route("/sim/start", post(sim::sim_start))
        .route("/sim/stop", post(sim::sim_stop))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/ws", get(ws::ws_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
