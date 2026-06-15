//! Prometheus metrics: recorder setup and the `/metrics` scrape handler.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::state::AppState;

/// Install the global Prometheus recorder and return a handle used to render `/metrics`.
/// We render manually rather than letting the exporter open its own HTTP listener, so the
/// scrape endpoint lives behind the same Axum router (and CORS policy) as everything else.
///
/// Errors only if a global recorder was already installed.
pub fn install() -> Result<PrometheusHandle, metrics_exporter_prometheus::BuildError> {
    PrometheusBuilder::new().install_recorder()
}

/// Build a recorder handle **without** installing it globally — for tests, where multiple
/// instances must coexist in one process.
pub fn test_handle() -> PrometheusHandle {
    PrometheusBuilder::new().build_recorder().handle()
}

pub async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, state.prometheus.render())
}
