//! In-process API tests: drive the real router (with a real engine + drain thread) via
//! `tower::ServiceExt::oneshot`, so no TCP socket is opened.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use engine_core::{spawn, RuntimeConfig};
use gateway::state::{spawn_drain, AppState};
use gateway::{build_router, metrics};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    let (engine, rx) = spawn(RuntimeConfig::default()).expect("engine");
    let state = AppState::new(engine, metrics::test_handle());
    spawn_drain(rx, state.clone());
    build_router(state)
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn valid_order_is_accepted() {
    let app = test_router();
    let req = post(
        "/orders",
        json!({ "side": "buy", "order_type": "limit", "price": 10000, "quantity": 5 }),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body = body_json(resp).await;
    assert_eq!(body["accepted"], json!(true));
    assert!(body["id"].as_u64().is_some());
}

#[tokio::test]
async fn limit_order_without_price_is_rejected_422() {
    let app = test_router();
    let req = post(
        "/orders",
        json!({ "side": "buy", "order_type": "limit", "quantity": 5 }),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["error"], json!("validation_failed"));
    assert!(body["fields"].as_array().is_some());
}

#[tokio::test]
async fn zero_quantity_is_rejected_422() {
    let app = test_router();
    let req = post(
        "/orders",
        json!({ "side": "sell", "order_type": "market", "quantity": 0 }),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn resting_order_appears_in_book_snapshot() {
    let app = test_router();
    // Submit a resting bid.
    let req = post(
        "/orders",
        json!({ "side": "buy", "order_type": "limit", "price": 9_900, "quantity": 7 }),
    );
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::ACCEPTED
    );

    // The drain thread publishes a coalesced snapshot within ~1 frame; poll briefly.
    let mut found = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/book").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = body_json(resp).await;
        if body["best_bid"] == json!(9_900) {
            found = true;
            break;
        }
    }
    assert!(found, "resting bid did not appear in /book snapshot");
}

#[tokio::test]
async fn sim_start_and_stop_toggle() {
    let app = test_router();
    let start = post("/sim/start", json!({ "rate": 500 }));
    let resp = app.clone().oneshot(start).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["running"], json!(true));
    assert_eq!(body["rate"], json!(500));

    let stop = post("/sim/stop", json!({}));
    let resp = app.oneshot(stop).await.unwrap();
    assert_eq!(body_json(resp).await["running"], json!(false));
}
