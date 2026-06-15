//! Executed trades and aggregated L2 book snapshots — the engine's read-side outputs.

use crate::domain::ids::{OrderId, Price, Qty, Seq};
use crate::domain::order::Side;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A single execution between an aggressing (taker) order and a resting (maker) order.
///
/// Trades always print at the **maker's** resting price (price-time priority gives the resting
/// order its price). `taker_side` is the aggressor's side, used by the UI to color the tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Trade {
    pub seq: Seq,
    pub taker_order: OrderId,
    pub maker_order: OrderId,
    pub price: Price,
    pub quantity: Qty,
    pub taker_side: Side,
}

/// One aggregated price level in an L2 snapshot: total resting quantity at a price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BookLevel {
    pub price: Price,
    pub quantity: Qty,
}

/// An aggregated level-2 view of the book: bids descending (best first), asks ascending.
///
/// Built on demand for publishing — never in the hot matching path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BookSnapshot {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    /// Sequence number of the last event reflected in this snapshot.
    pub seq: Seq,
}

impl BookSnapshot {
    /// Best (highest) bid price, if any.
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.first().map(|l| l.price)
    }

    /// Best (lowest) ask price, if any.
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first().map(|l| l.price)
    }

    /// Spread in ticks (best ask − best bid), if both sides are present.
    pub fn spread(&self) -> Option<u64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => a.get().checked_sub(b.get()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lvl(p: u64, q: u64) -> BookLevel {
        BookLevel {
            price: Price(p),
            quantity: Qty(q),
        }
    }

    #[test]
    fn snapshot_reports_best_prices_and_spread() {
        let snap = BookSnapshot {
            bids: vec![lvl(100, 5), lvl(99, 3)],
            asks: vec![lvl(102, 4), lvl(103, 7)],
            seq: Seq(10),
        };
        assert_eq!(snap.best_bid(), Some(Price(100)));
        assert_eq!(snap.best_ask(), Some(Price(102)));
        assert_eq!(snap.spread(), Some(2));
    }

    #[test]
    fn empty_snapshot_has_no_spread() {
        let snap = BookSnapshot::default();
        assert_eq!(snap.best_bid(), None);
        assert_eq!(snap.spread(), None);
    }
}
