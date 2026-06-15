//! Order-side, order-type, and the working-order representation.

use crate::domain::ids::{AccountId, OrderId, Price, Qty, Seq};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Which side of the book an order sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Side {
    /// Bid — wants to buy. Best price is the **highest**.
    Buy,
    /// Ask/offer — wants to sell. Best price is the **lowest**.
    Sell,
}

impl Side {
    /// The opposing side — the side an incoming order matches against.
    #[inline]
    pub const fn opposite(self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

/// The behavior of an order. Encodes price-handling and time-in-force in one exhaustive enum so
/// the matcher dispatches with a single `match` (no ambiguous flag combinations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum OrderType {
    /// Limit: match at the limit price or better; rest the remainder in the book (good-til-cancel).
    Limit,
    /// Market: sweep the opposing side at any price until filled; never rests.
    Market,
    /// Immediate-or-cancel: fill whatever is available now, cancel the remainder.
    ImmediateOrCancel,
    /// Fill-or-kill: fill the entire quantity immediately or reject the whole order.
    FillOrKill,
    /// Post-only: must rest as a maker; reject if it would immediately match.
    PostOnly,
    /// Stop (market): dormant until the last trade price crosses `stop_price`, then behaves as
    /// a market order.
    Stop,
    /// Stop-limit: dormant until triggered, then behaves as a limit order at `limit_price`.
    StopLimit,
}

impl OrderType {
    /// Whether an unmatched remainder of this order type may rest in the book.
    #[inline]
    pub const fn rests_when_unfilled(self) -> bool {
        matches!(self, OrderType::Limit | OrderType::PostOnly)
    }

    /// Whether this order type is dormant until a stop trigger fires.
    #[inline]
    pub const fn is_stop(self) -> bool {
        matches!(self, OrderType::Stop | OrderType::StopLimit)
    }
}

/// Self-trade-prevention policy: what to do when an incoming order would match a resting order
/// owned by the **same** account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StpPolicy {
    /// Cancel the incoming (aggressing) order's remaining quantity. The project default.
    #[default]
    CancelNewest,
    /// Cancel the resting (oldest) order and continue matching the aggressor.
    CancelOldest,
    /// Cancel the smaller of the two quantities from both orders, then continue.
    DecrementBoth,
}

/// A working order — either freshly submitted or resting in the book.
///
/// `limit_price` is `None` for market orders. `stop_price` is `Some` only for stop / stop-limit
/// orders. `seq` is assigned by the engine on acceptance and drives time priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Order {
    pub id: OrderId,
    pub account: AccountId,
    pub side: Side,
    pub order_type: OrderType,
    /// Limit price; `None` for market orders.
    pub limit_price: Option<Price>,
    /// Trigger price for stop / stop-limit orders.
    pub stop_price: Option<Price>,
    /// Remaining unfilled quantity.
    pub quantity: Qty,
    /// Engine-assigned time-priority sequence (0 until accepted).
    pub seq: Seq,
}

impl Order {
    /// Construct a plain limit order (test/convenience helper).
    pub fn limit(id: OrderId, account: AccountId, side: Side, price: Price, quantity: Qty) -> Self {
        Order {
            id,
            account,
            side,
            order_type: OrderType::Limit,
            limit_price: Some(price),
            stop_price: None,
            quantity,
            seq: Seq(0),
        }
    }

    /// Construct a market order (test/convenience helper).
    pub fn market(id: OrderId, account: AccountId, side: Side, quantity: Qty) -> Self {
        Order {
            id,
            account,
            side,
            order_type: OrderType::Market,
            limit_price: None,
            stop_price: None,
            quantity,
            seq: Seq(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_opposite_is_involutive() {
        assert_eq!(Side::Buy.opposite(), Side::Sell);
        assert_eq!(Side::Sell.opposite().opposite(), Side::Sell);
    }

    #[test]
    fn only_limit_and_post_only_rest() {
        assert!(OrderType::Limit.rests_when_unfilled());
        assert!(OrderType::PostOnly.rests_when_unfilled());
        assert!(!OrderType::Market.rests_when_unfilled());
        assert!(!OrderType::ImmediateOrCancel.rests_when_unfilled());
        assert!(!OrderType::FillOrKill.rests_when_unfilled());
    }

    #[test]
    fn stop_types_flagged() {
        assert!(OrderType::Stop.is_stop());
        assert!(OrderType::StopLimit.is_stop());
        assert!(!OrderType::Limit.is_stop());
    }

    #[test]
    fn stp_policy_defaults_to_cancel_newest() {
        assert_eq!(StpPolicy::default(), StpPolicy::CancelNewest);
    }
}
