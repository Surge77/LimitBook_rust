//! Behavior tests for the matching engine — the correctness proof for the project.
//!
//! Covers: empty book, exact match, partial fill, multi-level sweep, FIFO time priority, every
//! order type, all three STP policies, cancel of a nonexistent order, amends that keep vs lose
//! time priority, post-only rejection, fill-or-kill atomicity, IOC remainder cancel, and stop
//! triggering on a last-trade cross.

use engine_core::domain::{
    AccountId, EngineEvent, Order, OrderId, OrderType, Price, Qty, RejectReason, Seq, Side, Trade,
};
use engine_core::MatchingEngine;

// ---- builders -------------------------------------------------------------------------------

fn ord(
    id: u64,
    acct: u64,
    side: Side,
    ot: OrderType,
    limit: Option<u64>,
    stop: Option<u64>,
    qty: u64,
) -> Order {
    Order {
        id: OrderId(id),
        account: AccountId(acct),
        side,
        order_type: ot,
        limit_price: limit.map(Price),
        stop_price: stop.map(Price),
        quantity: Qty(qty),
        seq: Seq(0),
    }
}

fn limit(id: u64, acct: u64, side: Side, price: u64, qty: u64) -> Order {
    ord(id, acct, side, OrderType::Limit, Some(price), None, qty)
}

// ---- event helpers --------------------------------------------------------------------------

fn trades(evs: &[EngineEvent]) -> Vec<Trade> {
    evs.iter()
        .filter_map(|e| match e {
            EngineEvent::Trade(t) => Some(*t),
            _ => None,
        })
        .collect()
}

fn rejected_with(evs: &[EngineEvent], reason: RejectReason) -> bool {
    evs.iter()
        .any(|e| matches!(e, EngineEvent::OrderRejected { reason: r, .. } if *r == reason))
}

// ---- resting & basic matching ---------------------------------------------------------------

#[test]
fn limit_order_rests_in_empty_book() {
    let mut eng = MatchingEngine::default();
    let evs = eng.submit(limit(1, 1, Side::Buy, 100, 10)).to_vec();
    assert!(trades(&evs).is_empty());
    assert_eq!(eng.best_bid(), Some(Price(100)));
    assert_eq!(eng.snapshot(0).bids[0].quantity, Qty(10));
}

#[test]
fn exact_match_clears_both_orders() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 10));
    let evs = eng.submit(limit(2, 2, Side::Buy, 100, 10)).to_vec();
    let t = trades(&evs);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].price, Price(100));
    assert_eq!(t[0].quantity, Qty(10));
    assert_eq!(t[0].taker_side, Side::Buy);
    assert!(eng.best_bid().is_none());
    assert!(eng.best_ask().is_none());
}

#[test]
fn trade_prints_at_resting_maker_price() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 10)); // maker ask at 100
    let evs = eng.submit(limit(2, 2, Side::Buy, 105, 10)).to_vec(); // aggressive buy up to 105
    let t = trades(&evs);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].price, Price(100)); // executes at the maker's price, not the taker's
}

#[test]
fn partial_fill_leaves_remainder_resting() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 4));
    let evs = eng.submit(limit(2, 2, Side::Buy, 100, 10)).to_vec();
    let t = trades(&evs);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].quantity, Qty(4));
    // 6 remain as a resting bid
    assert_eq!(eng.best_bid(), Some(Price(100)));
    assert_eq!(eng.snapshot(0).bids[0].quantity, Qty(6));
}

#[test]
fn multi_level_sweep_fills_across_prices() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 5));
    eng.submit(limit(2, 1, Side::Sell, 101, 5));
    eng.submit(limit(3, 1, Side::Sell, 102, 5));
    let evs = eng.submit(limit(4, 2, Side::Buy, 101, 8)).to_vec();
    let t = trades(&evs);
    // fills 5 @100 then 3 @101; 102 untouched (above limit)
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].price, t[0].quantity), (Price(100), Qty(5)));
    assert_eq!((t[1].price, t[1].quantity), (Price(101), Qty(3)));
    assert_eq!(eng.best_ask(), Some(Price(101))); // 2 left at 101
    assert_eq!(eng.snapshot(0).asks[0].quantity, Qty(2));
}

#[test]
fn fifo_time_priority_at_a_level() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 5)); // earliest
    eng.submit(limit(2, 1, Side::Sell, 100, 5)); // later
    let evs = eng.submit(limit(3, 2, Side::Buy, 100, 5)).to_vec();
    let t = trades(&evs);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].maker_order, OrderId(1)); // oldest fills first
}

// ---- order types ----------------------------------------------------------------------------

#[test]
fn market_order_sweeps_best_available() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 3));
    eng.submit(limit(2, 1, Side::Sell, 101, 3));
    let evs = eng
        .submit(ord(3, 2, Side::Buy, OrderType::Market, None, None, 5))
        .to_vec();
    let t = trades(&evs);
    assert_eq!(t.iter().map(|x| x.quantity.get()).sum::<u64>(), 5);
    assert_eq!(eng.best_ask(), Some(Price(101)));
}

#[test]
fn market_order_with_no_liquidity_is_rejected() {
    let mut eng = MatchingEngine::default();
    let evs = eng
        .submit(ord(1, 1, Side::Buy, OrderType::Market, None, None, 5))
        .to_vec();
    assert!(rejected_with(&evs, RejectReason::NoLiquidity));
}

#[test]
fn ioc_fills_what_it_can_and_cancels_remainder() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 4));
    let evs = eng
        .submit(ord(
            2,
            2,
            Side::Buy,
            OrderType::ImmediateOrCancel,
            Some(100),
            None,
            10,
        ))
        .to_vec();
    assert_eq!(trades(&evs)[0].quantity, Qty(4));
    assert!(evs.iter().any(
        |e| matches!(e, EngineEvent::OrderCanceled { remaining, .. } if *remaining == Qty(6))
    ));
    assert!(eng.best_bid().is_none()); // nothing rests
}

#[test]
fn fok_fully_fills_or_rejects_atomically() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 4));
    // not enough liquidity for 10 -> reject, no trades, maker untouched
    let evs = eng
        .submit(ord(
            2,
            2,
            Side::Buy,
            OrderType::FillOrKill,
            Some(100),
            None,
            10,
        ))
        .to_vec();
    assert!(rejected_with(&evs, RejectReason::FillOrKillUnfillable));
    assert!(trades(&evs).is_empty());
    assert_eq!(eng.best_ask(), Some(Price(100)));
    assert_eq!(eng.snapshot(0).asks[0].quantity, Qty(4));

    // enough liquidity -> fully fills
    eng.submit(limit(3, 1, Side::Sell, 100, 6));
    let evs = eng
        .submit(ord(
            4,
            2,
            Side::Buy,
            OrderType::FillOrKill,
            Some(100),
            None,
            10,
        ))
        .to_vec();
    assert_eq!(
        trades(&evs).iter().map(|t| t.quantity.get()).sum::<u64>(),
        10
    );
}

#[test]
fn post_only_rejected_when_it_would_cross() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 5));
    let evs = eng
        .submit(ord(
            2,
            2,
            Side::Buy,
            OrderType::PostOnly,
            Some(100),
            None,
            5,
        ))
        .to_vec();
    assert!(rejected_with(&evs, RejectReason::PostOnlyWouldCross));
    assert!(trades(&evs).is_empty());
}

#[test]
fn post_only_rests_when_it_would_not_cross() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 101, 5));
    let evs = eng
        .submit(ord(
            2,
            2,
            Side::Buy,
            OrderType::PostOnly,
            Some(100),
            None,
            5,
        ))
        .to_vec();
    assert!(trades(&evs).is_empty());
    assert_eq!(eng.best_bid(), Some(Price(100)));
}

#[test]
fn stop_market_triggers_on_last_trade_cross() {
    let mut eng = MatchingEngine::default();
    // resting liquidity for the triggered stop to hit
    eng.submit(limit(1, 1, Side::Sell, 105, 10));
    // dormant buy stop with trigger 102
    let evs = eng
        .submit(ord(2, 2, Side::Buy, OrderType::Stop, None, Some(102), 5))
        .to_vec();
    assert!(trades(&evs).is_empty()); // dormant, nothing yet

    // a trade at 103 sets last price >= 102 -> stop activates and sweeps the 105 ask
    eng.submit(limit(3, 3, Side::Sell, 103, 1));
    let evs = eng.submit(limit(4, 4, Side::Buy, 103, 1)).to_vec();
    // last trade now 103; the stop should have fired during this submit's cascade
    let t = trades(&evs);
    assert!(t
        .iter()
        .any(|x| x.taker_order == OrderId(2) && x.price == Price(105)));
}

// ---- self-trade prevention ------------------------------------------------------------------

#[test]
fn stp_cancel_newest_aborts_aggressor() {
    let mut eng = MatchingEngine::with_capacity(64, engine_core::StpPolicy::CancelNewest);
    eng.submit(limit(1, 7, Side::Sell, 100, 5)); // account 7 resting
    let evs = eng.submit(limit(2, 7, Side::Buy, 100, 5)).to_vec(); // account 7 aggresses self
    assert!(trades(&evs).is_empty());
    assert_eq!(eng.best_ask(), Some(Price(100))); // resting maker untouched
    assert!(eng.best_bid().is_none()); // aggressor remainder not rested
}

#[test]
fn stp_cancel_oldest_removes_resting_then_continues() {
    let mut eng = MatchingEngine::with_capacity(64, engine_core::StpPolicy::CancelOldest);
    eng.submit(limit(1, 7, Side::Sell, 100, 5)); // own resting
    eng.submit(limit(2, 8, Side::Sell, 100, 5)); // other account, same level, later
    let evs = eng.submit(limit(3, 7, Side::Buy, 100, 5)).to_vec();
    // own maker (id 1) canceled; then trades against id 2
    assert!(evs
        .iter()
        .any(|e| matches!(e, EngineEvent::OrderCanceled { id, .. } if *id == OrderId(1))));
    let t = trades(&evs);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].maker_order, OrderId(2));
}

#[test]
fn stp_decrement_both_reduces_without_trading() {
    let mut eng = MatchingEngine::with_capacity(64, engine_core::StpPolicy::DecrementBoth);
    eng.submit(limit(1, 7, Side::Sell, 100, 5));
    let evs = eng.submit(limit(2, 7, Side::Buy, 100, 3)).to_vec();
    assert!(trades(&evs).is_empty());
    // maker reduced by 3 -> 2 remain
    assert_eq!(eng.snapshot(0).asks[0].quantity, Qty(2));
}

// ---- cancel & amend -------------------------------------------------------------------------

#[test]
fn cancel_nonexistent_order_errors() {
    let mut eng = MatchingEngine::default();
    let res = eng.cancel(OrderId(999));
    assert!(res.is_err());
}

#[test]
fn cancel_removes_resting_order() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Buy, 100, 5));
    let evs = eng.cancel(OrderId(1)).unwrap().to_vec();
    assert!(evs
        .iter()
        .any(|e| matches!(e, EngineEvent::OrderCanceled { id, remaining } if *id == OrderId(1) && *remaining == Qty(5))));
    assert!(eng.best_bid().is_none());
}

#[test]
fn amend_quantity_down_keeps_time_priority() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 10)); // earliest at level
    eng.submit(limit(2, 1, Side::Sell, 100, 10)); // later
    let evs = eng.amend(OrderId(1), Qty(5), None).unwrap().to_vec();
    assert!(evs
        .iter()
        .any(|e| matches!(e, EngineEvent::OrderAmended { repriced, .. } if !*repriced)));
    // id 1 still fills first (kept priority)
    let evs = eng.submit(limit(3, 2, Side::Buy, 100, 5)).to_vec();
    assert_eq!(trades(&evs)[0].maker_order, OrderId(1));
}

#[test]
fn amend_reprice_loses_time_priority() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Sell, 100, 10)); // earliest
    eng.submit(limit(2, 1, Side::Sell, 100, 10)); // later
                                                  // reprice id 1 to 100 again but via reprice path (change then back not allowed) — move to 101 then it is behind
    let evs = eng
        .amend(OrderId(1), Qty(10), Some(Price(101)))
        .unwrap()
        .to_vec();
    assert!(evs
        .iter()
        .any(|e| matches!(e, EngineEvent::OrderAmended { repriced, .. } if *repriced)));
    // now best ask is id 2 at 100; a buy at 100 hits id 2, not the repriced id 1
    let evs = eng.submit(limit(3, 2, Side::Buy, 100, 10)).to_vec();
    assert_eq!(trades(&evs)[0].maker_order, OrderId(2));
}

#[test]
fn amend_zero_quantity_errors() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Buy, 100, 5));
    assert!(eng.amend(OrderId(1), Qty(0), None).is_err());
}

// ---- validation -----------------------------------------------------------------------------

#[test]
fn zero_quantity_is_rejected() {
    let mut eng = MatchingEngine::default();
    let evs = eng.submit(limit(1, 1, Side::Buy, 100, 0)).to_vec();
    assert!(rejected_with(&evs, RejectReason::ZeroQuantity));
}

#[test]
fn duplicate_order_id_is_rejected() {
    let mut eng = MatchingEngine::default();
    eng.submit(limit(1, 1, Side::Buy, 100, 5));
    let evs = eng.submit(limit(1, 1, Side::Buy, 100, 5)).to_vec();
    assert!(rejected_with(&evs, RejectReason::DuplicateOrderId));
}
