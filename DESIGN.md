# DESIGN

Why the engine is built the way it is. The headline goal is **deterministic microsecond tail
latency with no GC pauses**, so every choice below is in service of correctness first and a
small, predictable hot path second.

## Layering and the trust boundary

```
frontend (React)  ──WS(read)/REST(write)──▶  gateway (Axum/tokio)  ──bounded channel──▶  engine-core
```

`engine-core` is a standalone library with **no async runtime, no network, no I/O, and no
logging**. It cannot even depend on the gateway. The gateway validates every inbound order at
the HTTP boundary (`dto::NewOrderRequest::into_order`) and the engine validates again defensively
— the engine trusts only what crosses that boundary.

## Fixed-point prices (integer ticks)

Prices are `Price(u64)` counted in **ticks**, where 1 tick = 0.01 quote units. Floats are never
used for price. Rationale:

- **Exactness.** A 1-ULP floating error in a comparison would mismatch an order — a wrong trade.
  Integer comparison is exact.
- **Ordering.** `u64` keys sort trivially and let `BTreeMap` give the best price in O(log n) from
  either end.
- **Speed.** Integer compare/branch, no FP unit, no rounding.

The frontend divides by 100 for display; the wire format is always ticks.

## Order book data structure

Each side is a `BTreeMap<Price, Level>`:

- **Best bid** = largest key (`bids.keys().next_back()`); **best ask** = smallest key
  (`asks.keys().next()`). Both O(log n); iteration walks levels in price order for sweeps.

Each `Level` is a **FIFO queue implemented as an intrusive doubly-linked list inside a
[`slab`](https://docs.rs/slab) arena**. Order nodes carry `prev`/`next` arena indices, and a
`HashMap<OrderId, nodeKey>` locates any resting order in O(1).

| Operation | Complexity |
|---|---|
| Best price | O(log n) levels |
| Enqueue at a level (rest) | O(1) |
| Match front of best level | O(1) |
| Cancel / amend-in-place by id | O(1) |

### No allocation on the hot path

The slab **reuses freed slots**, so resting an order after a cancel reuses memory — no
allocation. Matching is pure pointer (index) rewiring. The only amortized allocations are when a
brand-new price level appears (`BTreeMap` node) or the arena grows past its pre-sized capacity —
never on a steady-state fill. The engine pre-sizes the arena (`with_capacity`).

## Single-writer execution (LMAX-style)

One OS thread owns the `MatchingEngine`. Commands arrive over a **bounded**
`std::sync::mpsc::sync_channel` (backpressure, no `Mutex`, no shared mutable state in the
matching loop); output flows out over another bounded channel. Consequences:

- **Determinism.** The same command sequence produces identical trades and events every run
  (verified by the `matching_is_deterministic` property test).
- **No locks in the matcher.** Nothing else can touch the book, so no synchronization is needed
  on the per-fill path.

We use `std`'s bounded channel rather than an external crate to keep `engine-core` dependency
-light (its only runtime dep is `slab`; `serde` is an optional feature).

### Snapshot coalescing on the writer thread

Only the writer may read the book, so L2 snapshots are produced **on the engine thread**,
throttled to ~60fps via `recv_timeout`. Snapshotting per command would dominate at a million
orders/sec; events (trades/acks) are still forwarded immediately.

## Order types and matching rules

Strict **price-time priority**: best price first, then FIFO (lowest `Seq`) within a level. One
exhaustive `OrderType` enum drives a single `match`, avoiding ambiguous flag combinations:
Limit, Market, IOC, FOK, Post-Only, Stop, Stop-Limit. A repriced amend re-enters as an aggressor
so the book can never end up crossed.

## Self-trade prevention

Default policy is **cancel-newest** (`StpPolicy::CancelNewest`): when an incoming order would
match a resting order from the same account, the aggressor's remaining quantity is canceled
rather than rested — otherwise the account's own orders could cross. `CancelOldest` and
`DecrementBoth` are also supported.

## Error model

Two distinct types keep "the order was well-formed but declined" separate from "something
operationally failed":

- `RejectReason` — business rejection (post-only would cross, FOK unfillable, no liquidity, …),
  delivered as an `OrderRejected` event.
- `EngineError` — operational failure (unknown order on cancel, invalid amend), returned via
  `Result`.

No `.unwrap()`/`.expect()` in library paths; no `unsafe` anywhere in the engine.
