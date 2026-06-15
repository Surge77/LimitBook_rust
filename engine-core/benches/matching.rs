//! Throughput benchmarks for the matching engine.
//!
//! `cargo bench` reports orders/sec for a realistic mixed limit-order flow and for a
//! match-heavy flow. Per-order tail latency (p50/p99/p999) is measured separately by the
//! `latency` example (`cargo run --release --example latency`), which records and sorts
//! individual operation timings.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use engine_core::domain::{AccountId, Order, OrderId, OrderType, Price, Qty, Seq, Side};
use engine_core::{MatchingEngine, StpPolicy};

/// Tiny deterministic LCG so benches are reproducible without an RNG dependency.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

const MID: u64 = 10_000;

/// Generate `n` limit orders random-walking around `MID`. Two accounts alternate so aggressive
/// orders can actually match (single account would be self-trade-prevented).
fn gen_orders(n: usize, spread: u64) -> Vec<Order> {
    let mut rng = Lcg(0x1234_5678_9abc_def0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let side = if rng.next() & 1 == 0 {
            Side::Buy
        } else {
            Side::Sell
        };
        let offset = (rng.next() % (2 * spread + 1)) as i64 - spread as i64;
        let price = (MID as i64 + offset).max(1) as u64;
        let qty = 1 + rng.next() % 10;
        out.push(Order {
            id: OrderId(i as u64 + 1),
            account: AccountId(1 + (i as u64 & 1)),
            side,
            order_type: OrderType::Limit,
            limit_price: Some(Price(price)),
            stop_price: None,
            quantity: Qty(qty),
            seq: Seq(0),
        });
    }
    out
}

fn bench_throughput(c: &mut Criterion) {
    let n = 100_000;

    // Wide spread -> most orders rest, few match: stresses the insert/cancel path.
    let resting_heavy = gen_orders(n, 50);
    // Narrow spread -> orders cross constantly: stresses the matching path.
    let match_heavy = gen_orders(n, 3);

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(n as u64));

    group.bench_function("mixed_resting_heavy", |b| {
        b.iter_batched(
            || {
                (
                    MatchingEngine::with_capacity(2 * n, StpPolicy::CancelNewest),
                    resting_heavy.clone(),
                )
            },
            |(mut eng, orders)| {
                for o in orders {
                    let _ = eng.submit(o);
                }
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("match_heavy", |b| {
        b.iter_batched(
            || {
                (
                    MatchingEngine::with_capacity(2 * n, StpPolicy::CancelNewest),
                    match_heavy.clone(),
                )
            },
            |(mut eng, orders)| {
                for o in orders {
                    let _ = eng.submit(o);
                }
            },
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
