//! Engine output events and business-level rejection reasons.

use crate::domain::ids::{OrderId, Price, Qty};
use crate::domain::trade::Trade;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Why the engine rejected an order. Distinct from [`crate::domain::error::EngineError`]:
/// a rejection is a normal business outcome (the order was well-formed but cannot be accepted),
/// not an operational failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum RejectReason {
    /// Quantity was zero.
    ZeroQuantity,
    /// A limit/stop-limit order arrived without a limit price.
    MissingLimitPrice,
    /// A stop/stop-limit order arrived without a stop price.
    MissingStopPrice,
    /// A post-only order would have crossed the spread and traded as a taker.
    PostOnlyWouldCross,
    /// A fill-or-kill order could not be filled in full against available liquidity.
    FillOrKillUnfillable,
    /// A market order found no liquidity to execute against.
    NoLiquidity,
    /// Self-trade prevention canceled the aggressor (policy = cancel-newest).
    SelfTradePrevented,
    /// An order with this id already exists.
    DuplicateOrderId,
}

/// Everything the engine emits. The gateway translates these to wire messages.
///
/// `BookUpdated` is a lightweight marker: it signals that the aggregated book changed so the
/// gateway can throttle and then pull a [`crate::domain::trade::BookSnapshot`]. Carrying a full
/// snapshot in every event would allocate on the hot path.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum EngineEvent {
    /// The order passed validation and entered the matching pipeline.
    OrderAccepted { id: OrderId },
    /// The order was rejected for the given business reason.
    OrderRejected { id: OrderId, reason: RejectReason },
    /// A fill occurred.
    Trade(Trade),
    /// A resting order was canceled (by request or by an order-type rule such as IOC remainder).
    OrderCanceled { id: OrderId, remaining: Qty },
    /// A resting order was amended; `repriced` indicates it lost time priority.
    OrderAmended {
        id: OrderId,
        new_quantity: Qty,
        new_price: Option<Price>,
        repriced: bool,
    },
    /// The aggregated book changed; consumers should refresh their snapshot.
    BookUpdated,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::OrderId;

    #[test]
    fn events_are_comparable() {
        let a = EngineEvent::OrderAccepted { id: OrderId(1) };
        let b = EngineEvent::OrderAccepted { id: OrderId(1) };
        assert_eq!(a, b);
        assert_ne!(a, EngineEvent::BookUpdated);
    }

    #[test]
    fn reject_reasons_are_distinct() {
        assert_ne!(RejectReason::ZeroQuantity, RejectReason::NoLiquidity);
    }
}
