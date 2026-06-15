//! # engine-core
//!
//! A pure-Rust, price-time-priority order-matching engine — the core of a stock/crypto exchange.
//!
//! ## Design constraints (enforced throughout this crate)
//!
//! - **No async runtime, no network, no I/O, no logging** anywhere in this crate.
//! - **No heap allocation in the hot matching path.** Order nodes live in a [`slab`] arena;
//!   collections are pre-sized and buffers reused.
//! - **No `.unwrap()` / `.expect()`** in library paths — every fallible operation returns a
//!   [`EngineError`] and propagates with `?`.
//! - **No `unsafe`** without a comment proving soundness.
//!
//! The gateway crate wraps this engine behind channels; this crate never depends on the gateway.
//!
//! Module map:
//!   - [`domain`]: value types (newtypes, enums, `Order`, `Trade`, `EngineEvent`, `EngineError`).
//!   - [`book`]: the two-sided limit order book (slab arena + per-side `BTreeMap`).
//!   - [`matcher`]: pure price-time crossing predicates.
//!   - [`engine`]: the synchronous `MatchingEngine` orchestrator.
//!   - `runtime`: single-writer thread + channels, populated in milestone M4.

pub mod book;
pub mod domain;
pub mod engine;
pub mod matcher;

pub use domain::{
    AccountId, BookLevel, BookSnapshot, EngineError, EngineEvent, Order, OrderId, OrderType, Price,
    Qty, RejectReason, Seq, Side, StpPolicy, Trade, TICK_CENTS,
};
pub use engine::MatchingEngine;
