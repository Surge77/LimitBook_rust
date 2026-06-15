//! REST handlers. Writes are asynchronous (the engine is single-writer): they return `202
//! Accepted` with the assigned order id; the fill/reject outcome is delivered over the WebSocket.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use engine_core::domain::{OrderId, Price, Qty};
use serde_json::json;

use crate::dto::{AmendRequest, NewOrderRequest, OrderAck};
use crate::state::AppState;

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

/// `POST /orders` — submit an order. Returns 202 with the server-assigned id, or 422 with
/// field-level validation errors.
pub async fn submit_order(
    State(state): State<AppState>,
    Json(req): Json<NewOrderRequest>,
) -> impl IntoResponse {
    let id = state.alloc_id();
    match req.into_order(id) {
        Ok(order) => {
            metrics::counter!("limitbook_orders_submitted_total").increment(1);
            match state.engine.submit(order) {
                Ok(()) => {
                    (StatusCode::ACCEPTED, Json(OrderAck { id, accepted: true })).into_response()
                }
                Err(_) => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({ "error": "engine unavailable" })),
                )
                    .into_response(),
            }
        }
        Err(errors) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "validation_failed", "fields": errors })),
        )
            .into_response(),
    }
}

/// `DELETE /orders/{id}` — request cancellation (asynchronous; outcome via WebSocket).
pub async fn cancel_order(State(state): State<AppState>, Path(id): Path<u64>) -> impl IntoResponse {
    match state.engine.cancel(OrderId(id)) {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "id": id }))).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "engine unavailable" })),
        )
            .into_response(),
    }
}

/// `PUT /orders/{id}` — amend (cancel-replace).
pub async fn amend_order(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(req): Json<AmendRequest>,
) -> impl IntoResponse {
    if req.quantity == 0 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "validation_failed", "fields": [{ "field": "quantity", "message": "must be greater than zero" }] })),
        )
            .into_response();
    }
    match state
        .engine
        .amend(OrderId(id), Qty(req.quantity), req.price.map(Price))
    {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "id": id }))).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "engine unavailable" })),
        )
            .into_response(),
    }
}

/// `GET /book` — the latest cached L2 snapshot.
pub async fn get_book(State(state): State<AppState>) -> impl IntoResponse {
    let book = state.latest_book.read().ok().and_then(|g| g.clone());
    match book {
        Some(b) => (StatusCode::OK, Json(b)).into_response(),
        None => (
            StatusCode::OK,
            Json(json!({ "type": "book", "bids": [], "asks": [] })),
        )
            .into_response(),
    }
}

/// `GET /trades` — recent executions (most recent last).
pub async fn get_trades(State(state): State<AppState>) -> impl IntoResponse {
    let trades: Vec<_> = state
        .recent_trades
        .read()
        .map(|g| g.iter().cloned().collect())
        .unwrap_or_default();
    (StatusCode::OK, Json(trades))
}
