//! Per-order match-latency harness.
//!
//! Reports p50/p99/p999 latency in microseconds for a single matching operation under a
//! steady-state book, plus sustained throughput. Run with:
//!
//! ```text
//! cargo run --release --example latency
//! ```
//!
//! Build in release (`--release`) or the numbers are meaningless — debug builds add bounds-check
//! and overflow-check overhead that swamps the microsecond signal.

use std::time::Instant;

use engine_core::domain::{AccountId, Order, OrderId, OrderType, Price, Qty, Seq, Side};
use engine_core::{MatchingEngine, StpPolicy};

const WARMUP: usize = 50_000;
const SAMPLES: usize = 1_000_000;
const MID: u64 = 10_000;
const DEPTH: u64 = 200; // resting orders kept on the passive side

fn maker(id: u64, side: Side, price: u64) -> Order {
    Order {
        id: OrderId(id),
        account: AccountId(1),
        side,
        order_type: OrderType::Limit,
        limit_price: Some(Price(price)),
        stop_price: None,
        quantity: Qty(1),
        seq: Seq(0),
    }
}

fn taker(id: u64, side: Side, price: u64) -> Order {
    Order {
        id: OrderId(id),
        account: AccountId(2),
        side,
        order_type: OrderType::Limit,
        limit_price: Some(Price(price)),
        stop_price: None,
        quantity: Qty(1),
        seq: Seq(0),
    }
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn main() {
    let mut eng = MatchingEngine::with_capacity(4 * DEPTH as usize, StpPolicy::CancelNewest);
    let mut next_id: u64 = 1;

    // Seed a resting ask book around the mid.
    for k in 0..DEPTH {
        let _ = eng.submit(maker(next_id, Side::Sell, MID + 1 + k));
        next_id += 1;
    }

    // Steady state: each iteration adds one resting ask and sends one crossing buy that fills
    // exactly one maker, keeping the book depth stable.
    let mut run = |iterations: usize, record: Option<&mut Vec<u128>>| {
        let mut sink = Vec::new();
        let timings = record.unwrap_or(&mut sink);
        for _ in 0..iterations {
            let add_id = next_id;
            next_id += 1;
            let _ = eng.submit(maker(add_id, Side::Sell, MID + 1 + DEPTH));

            let take_id = next_id;
            next_id += 1;
            let buy = taker(take_id, Side::Buy, MID + 1 + DEPTH);
            let start = Instant::now();
            let _ = eng.submit(buy);
            timings.push(start.elapsed().as_nanos());
        }
    };

    run(WARMUP, None);

    let mut timings = Vec::with_capacity(SAMPLES);
    let wall = Instant::now();
    run(SAMPLES, Some(&mut timings));
    let elapsed = wall.elapsed();

    timings.sort_unstable();
    let p50 = percentile(&timings, 0.50) as f64 / 1000.0;
    let p99 = percentile(&timings, 0.99) as f64 / 1000.0;
    let p999 = percentile(&timings, 0.999) as f64 / 1000.0;
    let max = *timings.last().unwrap_or(&0) as f64 / 1000.0;
    // Each sample is one add + one matching submit = 2 ops.
    let ops = SAMPLES as f64 * 2.0;
    let per_sec = ops / elapsed.as_secs_f64();

    println!("LimitBook engine-core — match latency ({SAMPLES} samples, release)");
    println!("  p50   : {p50:>8.3} µs");
    println!("  p99   : {p99:>8.3} µs");
    println!("  p99.9 : {p999:>8.3} µs");
    println!("  max   : {max:>8.3} µs");
    println!("  throughput: {:.2} M ops/sec", per_sec / 1e6);
}
