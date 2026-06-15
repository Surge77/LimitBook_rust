//! Pure price-time matching predicates.
//!
//! The matching *loop* lives on [`crate::engine::MatchingEngine`] (it needs the engine's
//! sequence counter and event sink), but the price-crossing rule — the single most
//! correctness-critical predicate in the project — is isolated here and exhaustively tested.

use crate::domain::{Price, Side};

/// Does an aggressor on `taker_side` with optional limit `taker_limit` cross a resting maker at
/// `maker_price`?
///
/// - A **buy** crosses a sell when it is willing to pay at least the ask: `maker_price <= limit`.
/// - A **sell** crosses a bid when it accepts at most the bid: `maker_price >= limit`.
/// - A **market** order (`taker_limit == None`) always crosses available liquidity.
#[inline]
pub fn price_crosses(taker_side: Side, taker_limit: Option<Price>, maker_price: Price) -> bool {
    match taker_limit {
        None => true,
        Some(limit) => match taker_side {
            Side::Buy => maker_price <= limit,
            Side::Sell => maker_price >= limit,
        },
    }
}

/// Whether a stop order with `stop_price` on `side` is triggered by a `last_trade` print.
///
/// - A **buy stop** triggers when the market rises to or through the stop: `last >= stop`.
/// - A **sell stop** triggers when the market falls to or through the stop: `last <= stop`.
#[inline]
pub fn stop_triggered(side: Side, stop_price: Price, last_trade: Price) -> bool {
    match side {
        Side::Buy => last_trade >= stop_price,
        Side::Sell => last_trade <= stop_price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_crosses_when_ask_at_or_below_limit() {
        assert!(price_crosses(Side::Buy, Some(Price(100)), Price(100)));
        assert!(price_crosses(Side::Buy, Some(Price(100)), Price(99)));
        assert!(!price_crosses(Side::Buy, Some(Price(100)), Price(101)));
    }

    #[test]
    fn sell_crosses_when_bid_at_or_above_limit() {
        assert!(price_crosses(Side::Sell, Some(Price(100)), Price(100)));
        assert!(price_crosses(Side::Sell, Some(Price(100)), Price(101)));
        assert!(!price_crosses(Side::Sell, Some(Price(100)), Price(99)));
    }

    #[test]
    fn market_always_crosses() {
        assert!(price_crosses(Side::Buy, None, Price(1)));
        assert!(price_crosses(Side::Sell, None, Price(u64::MAX)));
    }

    #[test]
    fn buy_stop_triggers_on_rise() {
        assert!(stop_triggered(Side::Buy, Price(100), Price(100)));
        assert!(stop_triggered(Side::Buy, Price(100), Price(105)));
        assert!(!stop_triggered(Side::Buy, Price(100), Price(99)));
    }

    #[test]
    fn sell_stop_triggers_on_fall() {
        assert!(stop_triggered(Side::Sell, Price(100), Price(100)));
        assert!(stop_triggered(Side::Sell, Price(100), Price(95)));
        assert!(!stop_triggered(Side::Sell, Price(100), Price(101)));
    }
}
