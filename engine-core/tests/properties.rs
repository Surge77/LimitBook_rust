//! Property-based invariants for the matching engine (`proptest`).
//!
//! These assert the laws that must hold for *any* input sequence:
//!   1. the book never crosses (best bid < best ask),
//!   2. quantity is conserved across matching (fills never exceed what was offered),
//!   3. no trade ever has zero quantity,
//!   4. matching is deterministic (identical input → identical event stream).

use engine_core::domain::{
    AccountId, EngineEvent, Order, OrderId, OrderType, Price, Qty, Seq, Side,
};
use engine_core::MatchingEngine;
use proptest::prelude::*;

/// A generated command against the engine.
#[derive(Debug, Clone)]
enum Cmd {
    Submit { side: Side, price: u64, qty: u64 },
    Cancel { which: usize },
}

fn cmd_strategy() -> impl Strategy<Value = Cmd> {
    prop_oneof![
        4 => (any::<bool>(), 95u64..=105, 1u64..=20).prop_map(|(b, price, qty)| Cmd::Submit {
            side: if b { Side::Buy } else { Side::Sell },
            price,
            qty,
        }),
        1 => (0usize..32).prop_map(|which| Cmd::Cancel { which }),
    ]
}

fn limit(id: u64, side: Side, price: u64, qty: u64) -> Order {
    limit_acct(id, 1, side, price, qty)
}

fn limit_acct(id: u64, acct: u64, side: Side, price: u64, qty: u64) -> Order {
    Order {
        id: OrderId(id),
        account: AccountId(acct),
        side,
        order_type: OrderType::Limit,
        limit_price: Some(Price(price)),
        stop_price: None,
        quantity: Qty(qty),
        seq: Seq(0),
    }
}

/// Drive a fresh engine through a command list, returning the full event stream.
fn run(cmds: &[Cmd]) -> Vec<EngineEvent> {
    let mut eng = MatchingEngine::default();
    let mut all = Vec::new();
    let mut accepted_ids: Vec<u64> = Vec::new();
    let mut next_id = 1u64;
    for cmd in cmds {
        match *cmd {
            Cmd::Submit { side, price, qty } => {
                let id = next_id;
                next_id += 1;
                for e in eng.submit(limit(id, side, price, qty)) {
                    all.push(e.clone());
                }
                accepted_ids.push(id);
            }
            Cmd::Cancel { which } => {
                if let Some(&id) = accepted_ids.get(which % accepted_ids.len().max(1)) {
                    if let Ok(evs) = eng.cancel(OrderId(id)) {
                        all.extend(evs.iter().cloned());
                    }
                }
            }
        }
    }
    all
}

proptest! {
    #[test]
    fn book_never_crosses(cmds in prop::collection::vec(cmd_strategy(), 0..60)) {
        let mut eng = MatchingEngine::default();
        let mut next_id = 1u64;
        let mut ids: Vec<u64> = Vec::new();
        for cmd in &cmds {
            match *cmd {
                Cmd::Submit { side, price, qty } => {
                    let id = next_id; next_id += 1;
                    eng.submit(limit(id, side, price, qty));
                    ids.push(id);
                }
                Cmd::Cancel { which } => {
                    if !ids.is_empty() {
                        let _ = eng.cancel(OrderId(ids[which % ids.len()]));
                    }
                }
            }
            if let (Some(bid), Some(ask)) = (eng.best_bid(), eng.best_ask()) {
                prop_assert!(bid < ask, "book crossed: bid {bid} >= ask {ask}");
            }
        }
    }

    #[test]
    fn no_trade_has_zero_quantity(cmds in prop::collection::vec(cmd_strategy(), 0..60)) {
        for e in run(&cmds) {
            if let EngineEvent::Trade(t) = e {
                prop_assert!(t.quantity.get() > 0);
            }
        }
    }

    #[test]
    fn aggressor_fills_never_exceed_submitted(
        maker_qty in 1u64..50,
        taker_qty in 1u64..50,
    ) {
        // One resting sell, one aggressing buy that crosses. Fills must conserve quantity.
        let mut eng = MatchingEngine::default();
        eng.submit(limit_acct(1, 1, Side::Sell, 100, maker_qty));
        let evs: Vec<_> = eng.submit(limit_acct(2, 2, Side::Buy, 100, taker_qty)).to_vec();
        let filled: u64 = evs.iter().filter_map(|e| match e {
            EngineEvent::Trade(t) => Some(t.quantity.get()),
            _ => None,
        }).sum();
        prop_assert_eq!(filled, maker_qty.min(taker_qty));
        prop_assert!(filled <= taker_qty);
        // Whatever didn't fill rests on exactly one side.
        let resting_bid = eng.snapshot(0).bids.iter().map(|l| l.quantity.get()).sum::<u64>();
        let resting_ask = eng.snapshot(0).asks.iter().map(|l| l.quantity.get()).sum::<u64>();
        prop_assert_eq!(resting_bid + resting_ask, taker_qty.max(maker_qty) - filled);
    }

    #[test]
    fn matching_is_deterministic(cmds in prop::collection::vec(cmd_strategy(), 0..60)) {
        let a = run(&cmds);
        let b = run(&cmds);
        prop_assert_eq!(a, b);
    }
}
