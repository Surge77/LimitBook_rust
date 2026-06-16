<div align="center">

# LimitBook ⚡

**A low-latency order-matching engine in Rust — with a real-time trading-terminal UI.**

[![CI](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

</div>

LimitBook accepts buy/sell orders, matches them by **price-time priority**, executes trades,
maintains a live order book, and streams everything to a web dashboard in real time. The matching
core is a pure-Rust library with **no garbage collector and no heap allocation in the hot path**,
so latency stays in the microsecond range under load.

## Performance

Measured on the development machine (Windows, x86-64, `--release`) via the bundled latency
harness — run it yourself with `cargo run --release --example latency`:

| Metric | Measured |
|---|---|
| Throughput (single thread) | **≈ 11.2M orders/sec** |
| Match latency p50 | **≈ 0.1 µs** |
| Match latency p99 | **≈ 0.2 µs** |
| Match latency p99.9 | **≈ 0.4 µs** |
| GC pauses | zero (no GC) |

> p50/p99 sit at the host timer's ~100 ns resolution floor, so true per-operation cost is
> sub-100 ns.

## Matching rules

- **Price priority** — best bid (highest) and best ask (lowest) match first.
- **Time priority (FIFO)** — at one price level, the earliest order fills first.
- **Trades print at the maker's resting price**, never the aggressor's.
- **Partial fills** sweep across multiple resting orders and price levels; the remainder rests or
  cancels per order type.
- **Self-trade prevention** (default `cancel-newest`; also `cancel-oldest`, `decrement-both`).

**Order types:** Limit · Market · IOC · FOK · Post-Only · Stop / Stop-Limit · Cancel · Amend.

## Quick start (local)

```bash
# 1. Engine + gateway (listens on http://127.0.0.1:8080)
cargo run -p gateway

# 2. Frontend (separate terminal) → http://localhost:5173
cd frontend && npm install && npm run dev
```

Open the dashboard — synthetic order flow starts automatically, so the book, trades, and metrics
are live immediately. Use the simulator panel to stop/adjust the flow, or submit orders by hand in
the order-entry panel.

## Quick start (Docker)

```bash
docker compose up --build      # → http://localhost:3000
```

Builds the gateway in release and serves the frontend behind nginx, which proxies REST and
WebSocket to the gateway.

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

## License

[MIT](LICENSE) © 2026 Surge77
