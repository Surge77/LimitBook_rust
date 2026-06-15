//! LimitBook gateway binary: a thin bootstrap over the [`gateway`] library.
//!
//! Spins up the engine on its own thread, bridges its output to a tokio broadcast for WebSocket
//! fan-out, starts the simulator task, and serves the router on localhost.

use std::net::SocketAddr;

use engine_core::{spawn, RuntimeConfig};
use gateway::state::{spawn_drain, AppState};
use gateway::{build_router, metrics, sim};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gateway=info,tower_http=info".into()),
        )
        .init();

    let prometheus = metrics::install().expect("failed to install Prometheus recorder");

    let (engine, rx) = spawn(RuntimeConfig::default()).expect("failed to start engine");
    let state = AppState::new(engine, prometheus);

    spawn_drain(rx, state.clone());
    sim::spawn_simulator(state.clone());

    let app = build_router(state);

    // Bind address is configurable so the container can listen on 0.0.0.0 while local dev stays
    // on loopback.
    let bind = std::env::var("LIMITBOOK_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let addr: SocketAddr = bind.parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    tracing::info!("LimitBook gateway listening on http://{addr}");

    axum::serve(listener, app).await.expect("server error");
}
