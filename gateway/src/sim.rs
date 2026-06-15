//! Synthetic order-flow simulator: a random walk around a moving mid price, so the dashboard
//! looks alive on demand and the engine can be stress-tested.

use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use engine_core::domain::{AccountId, Order, OrderId, OrderType, Price, Qty, Seq, Side};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Deserialize;
use serde_json::json;

use crate::state::AppState;

const TICK_MS: u64 = 10; // submit a batch every 10ms; rate is orders/sec
const NUM_ACCOUNTS: u64 = 8;
const START_MID: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct SimStartRequest {
    pub rate: Option<u64>,
}

pub async fn sim_start(
    State(state): State<AppState>,
    Json(req): Json<SimStartRequest>,
) -> impl IntoResponse {
    if let Some(rate) = req.rate {
        state
            .sim_rate
            .store(rate.clamp(1, 1_000_000), Ordering::Relaxed);
    }
    state.sim_running.store(true, Ordering::Relaxed);
    Json(json!({ "running": true, "rate": state.sim_rate.load(Ordering::Relaxed) }))
}

pub async fn sim_stop(State(state): State<AppState>) -> impl IntoResponse {
    state.sim_running.store(false, Ordering::Relaxed);
    Json(json!({ "running": false }))
}

/// Spawn the persistent simulator task. It idles until `sim_running` is set, then submits a
/// batch of random orders each tick proportional to the configured rate.
pub fn spawn_simulator(state: AppState) {
    tokio::spawn(async move {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        let mut mid: i64 = START_MID;
        loop {
            if !state.sim_running.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
            let rate = state.sim_rate.load(Ordering::Relaxed).max(1);
            let batch = (rate * TICK_MS / 1000).max(1);

            // Random-walk the mid within a sane band.
            mid += rng.gen_range(-2..=2);
            mid = mid.clamp(1_000, 100_000);

            for _ in 0..batch {
                let order = random_order(&mut rng, mid, state.alloc_id());
                if state.engine.try_submit(order).is_err() {
                    break; // queue full — yield this tick
                }
            }
            tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
        }
    });
}

fn random_order(rng: &mut StdRng, mid: i64, id: u64) -> Order {
    let side = if rng.gen_bool(0.5) {
        Side::Buy
    } else {
        Side::Sell
    };
    // Bias prices to cross sometimes: buys slightly above mid, sells slightly below, with noise.
    let drift = match side {
        Side::Buy => rng.gen_range(-3..=4),
        Side::Sell => rng.gen_range(-4..=3),
    };
    let price = (mid + drift).clamp(1, 1_000_000) as u64;
    let qty = rng.gen_range(1..=20);
    let account = 1 + rng.gen_range(0..NUM_ACCOUNTS);
    Order {
        id: OrderId(id),
        account: AccountId(account),
        side,
        order_type: OrderType::Limit,
        limit_price: Some(Price(price)),
        stop_price: None,
        quantity: Qty(qty),
        seq: Seq(0),
    }
}
