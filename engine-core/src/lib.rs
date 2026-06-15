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
//! Modules are filled in across milestones M2–M4. This is the scaffold entry point.

// Subsequent milestones populate these modules:
//   M2: domain  (newtypes, enums, Order, Trade, EngineEvent, EngineError)
//   M3: book, matcher
//   M4: engine  (single-writer loop)
