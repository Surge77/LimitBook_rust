//! Domain value types: identifiers, orders, trades, events, and errors.
//!
//! Every type here is a plain data value with no behavior that touches I/O. Serialization is
//! provided only when the `serde` feature is enabled (the gateway turns it on); the matcher
//! itself never needs it.

pub mod error;
pub mod event;
pub mod ids;
pub mod order;
pub mod trade;

pub use error::EngineError;
pub use event::{EngineEvent, RejectReason};
pub use ids::{AccountId, OrderId, Price, Qty, Seq, TICK_CENTS};
pub use order::{Order, OrderType, Side, StpPolicy};
pub use trade::{BookLevel, BookSnapshot, Trade};
