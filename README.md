<div align="center">

# LimitBook ⚡

**A production-grade, low-latency order-matching engine in Rust — with a real-time trading-terminal UI.**

[![CI](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Surge77/LimitBook_rust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

</div>

> ⚠️ **Status: under active construction.** Headline latency/throughput numbers below are
> placeholders and will be replaced with **real measured `criterion` results** once the
> benchmark milestone lands. This README is updated continuously as milestones complete.

---

## Why Rust

The entire point of building this in Rust is **deterministic microsecond tail latency with no
garbage-collector pauses.** The matching core is a `no_std`-in-spirit library: zero async, zero
I/O, zero logging, and **no heap allocation in the hot matching path**. Correctness and
measurable p99/p999 latency are the headline.

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
the gateway and never reaches into the matching core.

## Headline metrics (placeholder — pending real benchmark)

| Metric | Target | Measured |
|---|---|---|
| Throughput (single thread) | ≥ 1M orders/sec | _TBD_ |
| Match latency p50 | — | _TBD_ |
| Match latency p99 | < 20 µs | _TBD_ |
| Match latency p999 | reported | _TBD_ |
| GC pauses | zero | ✅ zero (no GC) |

## Features

**Order types:** Limit · Market · IOC · FOK · Post-Only · Stop / Stop-Limit · Cancel · Cancel-Replace (amend).

**Matching:** strict price-time priority (FIFO at each level), partial fills across levels,
configurable self-trade prevention (STP).

**Gateway:** REST order intake, throttled WebSocket broadcast, Prometheus metrics, built-in
synthetic order-flow simulator.

**Frontend:** live L2 book, depth chart, trade tape, order entry, live metrics, simulator controls.

## Repository layout

```
engine-core/   pure Rust matching engine (library)
gateway/       Axum + tokio service wrapping the engine
frontend/      React + TypeScript + Vite + Tailwind dashboard
```

## Quick start

```bash
# Engine + gateway
cargo run -p gateway

# Frontend (separate terminal)
cd frontend && npm install && npm run dev
```

Full build, test, benchmark, and Docker instructions land with the corresponding milestones.

## Development

```bash
cargo test                                   # unit + integration + proptest
cargo clippy --all-targets -- -D warnings    # lint gate
cargo fmt --all                              # format
cargo bench                                  # criterion latency/throughput
```

See [DESIGN.md](DESIGN.md) for data-structure and architecture rationale,
[CONTRIBUTING.md](CONTRIBUTING.md) for workflow, and [SECURITY.md](SECURITY.md) for reporting.

## License

[MIT](LICENSE) © 2026 Surge77
