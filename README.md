<div align="center">

# LimitBook ⚡

**A production-grade, low-latency order-matching engine in Rust — with a real-time trading-terminal UI.**

[![CI](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

</div>

A matching engine that accepts buy/sell orders, matches them by **price-time priority**, executes
trades, maintains a live order book, and streams everything to a web dashboard in real time.

## Why Rust

The entire point of building this in Rust is **deterministic microsecond tail latency with no
garbage-collector pauses.** The matching core is a `no_std`-in-spirit library: zero async, zero
I/O, zero logging, and **no heap allocation in the hot matching path**. Correctness and
measurable p99/p999 latency are the headline.

## Headline metrics

Measured on the development machine (Windows, x86-64, `--release`) via the bundled latency
harness (`cargo run --release --example latency`). Run it yourself — numbers print to stdout.

| Metric | Target | Measured |
|---|---|---|
| Throughput (single thread) | ≥ 1M orders/sec | **≈ 11.2M ops/sec** |
| Match latency p50 | — | **≈ 0.1 µs** |
| Match latency p99 | < 20 µs | **≈ 0.2 µs** |
| Match latency p99.9 | reported | **≈ 0.4 µs** |
| GC pauses | zero | ✅ zero (no GC) |

> p50/p99 sit at the host timer's ~100 ns resolution floor, so true per-operation cost is
> sub-100 ns. The rare multi-µs `max` is an OS-scheduling/allocator-growth outlier, not steady
> state. `criterion` throughput benches (`cargo bench`) run in CI.

## Architecture

```
┌─────────────────────────────────────────────┐
│  FRONTEND (React + TS + Tailwind)            │
│  live order book · depth chart · trade tape  │
│  order-entry form · latency/throughput panel │
└───────────────▲─────────────────────────────┘
                │ WebSocket (read) + REST (write)
┌───────────────┴─────────────────────────────┐
│  GATEWAY (Rust + Axum, async/tokio)          │
│  REST order intake · WS broadcast · metrics  │
└───────────────▲─────────────────────────────┘
                │ in-process bounded channel (no network)
┌───────────────┴─────────────────────────────┐
│  ENGINE CORE (pure Rust, no async/IO/alloc   │
│  in the hot path) · order book · matcher     │
└──────────────────────────────────────────────┘
```

**Hard rule:** the engine core is a standalone library crate with NO async runtime, NO network,
NO I/O, NO logging in the hot path. The gateway wraps it; the UI only *reads* engine state via
the gateway. See [DESIGN.md](DESIGN.md) for the data-structure and concurrency rationale.

## Matching rules

- **Price priority** — best bid (highest) and best ask (lowest) match first.
- **Time priority (FIFO)** — at one price level, the earliest order fills first.
- **Trades print at the maker's resting price**, never the aggressor's.
- **Partial fills** sweep across multiple resting orders and price levels; the remainder rests or
  cancels per order type.
- **Self-trade prevention** (default `cancel-newest`; also `cancel-oldest`, `decrement-both`).

**Order types:** Limit · Market · IOC · FOK · Post-Only · Stop / Stop-Limit · Cancel · Amend.

## Repository layout

```
engine-core/   pure-Rust matching engine (library) — book, matcher, single-writer runtime
gateway/       Axum + tokio service: REST + WebSocket + Prometheus + simulator
frontend/      React + TypeScript + Vite + Tailwind dashboard
```

## Quick start (local)

```bash
# 1. Engine + gateway (listens on http://127.0.0.1:8080)
cargo run -p gateway

# 2. Frontend (separate terminal) → http://localhost:5173
cd frontend && npm install && npm run dev
```

Click **Start flow** in the simulator panel to fill the book with synthetic order flow, or submit
orders by hand in the order-entry panel.

## Quick start (Docker)

```bash
docker compose up --build      # → http://localhost:3000
```

> The compose stack and Dockerfiles are authored but not validated locally (Docker is not
> installed on the dev machine). They build the gateway in release and serve the frontend behind
> nginx, which proxies REST + WebSocket to the gateway.

## HTTP / WebSocket API

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/orders` | Submit an order → `202` with assigned id (outcome arrives via WS) |
| `DELETE` | `/orders/{id}` | Cancel a resting order |
| `PUT` | `/orders/{id}` | Amend (cancel-replace) |
| `GET` | `/book` | Latest L2 snapshot |
| `GET` | `/trades` | Recent executions |
| `POST` | `/sim/start` `/sim/stop` | Control the synthetic-flow simulator |
| `GET` | `/metrics` | Prometheus scrape endpoint |
| `GET` | `/ws` | WebSocket: book snapshots, trades, order events |

Example:

```bash
curl -X POST localhost:8080/orders \
  -H 'content-type: application/json' \
  -d '{"side":"buy","order_type":"limit","price":10000,"quantity":5}'
# → 202 {"id":1,"accepted":true}
```

Prices are integer **ticks** (1 tick = 0.01 quote units).

## Development

```bash
cargo test --all                                 # 50+ unit + integration + proptest
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
cargo run --release --example latency            # p50/p99/p999 latency
cargo bench                                       # criterion throughput
cd frontend && npm run build                      # type-check + production build
```

See [DESIGN.md](DESIGN.md), [CONTRIBUTING.md](CONTRIBUTING.md), and [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © 2026 Surge77
