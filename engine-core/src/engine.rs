//! The matching engine: validation, the price-time matching loop, order-type semantics,
//! self-trade prevention, and stop-order triggering.
//!
//! This type is **single-threaded and side-effect-free** (no I/O, no logging, no allocation on
//! the per-fill path). The M4 single-writer loop owns one instance on a dedicated thread; the
//! gateway never touches it directly.

use crate::book::OrderBook;
use crate::domain::{
    BookSnapshot, EngineError, EngineEvent, Order, OrderType, Price, Qty, RejectReason, Seq, Side,
    StpPolicy, Trade,
};
use crate::matcher::{price_crosses, stop_triggered};

/// Owns the book and all matching state. Events from each command are written to a reused
/// internal buffer and returned as a slice.
#[derive(Debug)]
pub struct MatchingEngine {
    book: OrderBook,
    seq: Seq,
    last_trade_price: Option<Price>,
    stp: StpPolicy,
    pending_stops: Vec<Order>,
    events: Vec<EngineEvent>,
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::with_capacity(1024, StpPolicy::default())
    }
}

impl MatchingEngine {
    pub fn with_capacity(capacity: usize, stp: StpPolicy) -> Self {
        MatchingEngine {
            book: OrderBook::with_capacity(capacity),
            seq: Seq(1),
            last_trade_price: None,
            stp,
            pending_stops: Vec::new(),
            events: Vec::with_capacity(64),
        }
    }

    #[inline]
    fn next_seq(&mut self) -> Seq {
        let s = self.seq;
        self.seq = self.seq.next();
        s
    }

    pub fn last_trade_price(&self) -> Option<Price> {
        self.last_trade_price
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.book.best_bid()
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.book.best_ask()
    }

    pub fn snapshot(&self, depth: usize) -> BookSnapshot {
        self.book.snapshot(depth, self.seq)
    }

    /// Submit a new order. Returns the events it produced (accept/reject, trades, cancels,
    /// book-updated, plus any cascaded stop activations).
    pub fn submit(&mut self, mut order: Order) -> &[EngineEvent] {
        self.events.clear();
        if let Some(reason) = validate(&order) {
            self.events.push(EngineEvent::OrderRejected {
                id: order.id,
                reason,
            });
            return &self.events;
        }
        if self.book.contains(order.id) {
            self.events.push(EngineEvent::OrderRejected {
                id: order.id,
                reason: RejectReason::DuplicateOrderId,
            });
            return &self.events;
        }
        order.seq = self.next_seq();
        self.events
            .push(EngineEvent::OrderAccepted { id: order.id });

        if order.order_type.is_stop() {
            self.handle_stop_submission(order);
        } else {
            self.process_active(order);
        }
        self.trigger_stops();
        &self.events
    }

    /// Cancel a resting (or dormant stop) order.
    pub fn cancel(&mut self, id: crate::domain::OrderId) -> Result<&[EngineEvent], EngineError> {
        self.events.clear();
        if let Some(pos) = self.pending_stops.iter().position(|o| o.id == id) {
            let stop = self.pending_stops.remove(pos);
            self.events.push(EngineEvent::OrderCanceled {
                id,
                remaining: stop.quantity,
            });
            return Ok(&self.events);
        }
        let remaining = self.book.cancel(id)?;
        self.events
            .push(EngineEvent::OrderCanceled { id, remaining });
        self.events.push(EngineEvent::BookUpdated);
        Ok(&self.events)
    }

    /// Amend (cancel-replace) a resting order. A pure quantity *decrease* at the same price keeps
    /// time priority; a reprice or quantity *increase* loses priority and re-enters as an
    /// aggressor (so the book can never end up crossed).
    pub fn amend(
        &mut self,
        id: crate::domain::OrderId,
        new_qty: Qty,
        new_price: Option<Price>,
    ) -> Result<&[EngineEvent], EngineError> {
        self.events.clear();
        if new_qty.is_zero() {
            return Err(EngineError::InvalidAmendQuantity);
        }
        let (side, price, account, old_qty) =
            self.book.get(id).ok_or(EngineError::UnknownOrder(id))?;
        let target_price = new_price.unwrap_or(price);
        let repriced = target_price != price;
        let qty_increases = new_qty > old_qty;

        if !repriced && !qty_increases {
            self.book.reduce_in_place(id, new_qty)?;
            self.events.push(EngineEvent::OrderAmended {
                id,
                new_quantity: new_qty,
                new_price: Some(target_price),
                repriced: false,
            });
            self.events.push(EngineEvent::BookUpdated);
        } else {
            self.book.cancel(id)?;
            self.events.push(EngineEvent::OrderAmended {
                id,
                new_quantity: new_qty,
                new_price: Some(target_price),
                repriced: true,
            });
            let mut replacement = Order::limit(id, account, side, target_price, new_qty);
            replacement.seq = self.next_seq();
            self.process_active(replacement);
            self.trigger_stops();
        }
        Ok(&self.events)
    }

    fn handle_stop_submission(&mut self, order: Order) {
        let triggered = match (self.last_trade_price, order.stop_price) {
            (Some(last), Some(stop)) => stop_triggered(order.side, stop, last),
            _ => false,
        };
        if triggered {
            self.activate_stop(order);
        } else {
            self.pending_stops.push(order);
        }
    }

    fn activate_stop(&mut self, order: Order) {
        let active = match order.order_type {
            OrderType::Stop => Order {
                order_type: OrderType::Market,
                limit_price: None,
                ..order
            },
            OrderType::StopLimit => Order {
                order_type: OrderType::Limit,
                ..order
            },
            _ => order,
        };
        self.process_active(active);
    }

    /// Re-scan dormant stops after the last trade price moves; activate any now-triggered,
    /// cascading until none remain.
    fn trigger_stops(&mut self) {
        loop {
            let Some(last) = self.last_trade_price else {
                return;
            };
            let idx = self.pending_stops.iter().position(|o| {
                o.stop_price
                    .map(|sp| stop_triggered(o.side, sp, last))
                    .unwrap_or(false)
            });
            let Some(i) = idx else { break };
            let stop = self.pending_stops.remove(i);
            self.activate_stop(stop);
        }
    }

    fn would_cross(&self, side: Side, limit: Option<Price>) -> bool {
        self.book
            .best_maker(side.opposite())
            .map(|m| price_crosses(side, limit, m.price))
            .unwrap_or(false)
    }

    /// Dispatch an active (non-stop) order through its type-specific matching rules.
    fn process_active(&mut self, order: Order) {
        let limit = order.limit_price;
        match order.order_type {
            OrderType::PostOnly => {
                if self.would_cross(order.side, limit) {
                    self.reject(order.id, RejectReason::PostOnlyWouldCross);
                    return;
                }
                self.rest(&order, order.quantity);
                self.events.push(EngineEvent::BookUpdated);
            }
            OrderType::FillOrKill => {
                if self.book.available_against(order.side, limit) < order.quantity {
                    self.reject(order.id, RejectReason::FillOrKillUnfillable);
                    return;
                }
                let (remaining, _) = self.match_aggressor(&order, limit, order.quantity);
                if !remaining.is_zero() {
                    self.events.push(EngineEvent::OrderCanceled {
                        id: order.id,
                        remaining,
                    });
                }
                self.events.push(EngineEvent::BookUpdated);
            }
            OrderType::Market => {
                let (remaining, _) = self.match_aggressor(&order, None, order.quantity);
                if remaining == order.quantity {
                    self.reject(order.id, RejectReason::NoLiquidity);
                    return;
                }
                if !remaining.is_zero() {
                    self.events.push(EngineEvent::OrderCanceled {
                        id: order.id,
                        remaining,
                    });
                }
                self.events.push(EngineEvent::BookUpdated);
            }
            OrderType::ImmediateOrCancel => {
                let (remaining, _) = self.match_aggressor(&order, limit, order.quantity);
                if !remaining.is_zero() {
                    self.events.push(EngineEvent::OrderCanceled {
                        id: order.id,
                        remaining,
                    });
                }
                self.events.push(EngineEvent::BookUpdated);
            }
            OrderType::Limit => {
                let (remaining, stp_aborted) = self.match_aggressor(&order, limit, order.quantity);
                if !remaining.is_zero() {
                    if stp_aborted {
                        // Self-trade prevention (cancel-newest) canceled the aggressor: the
                        // remainder must NOT rest, or the account's own orders would cross.
                        self.events.push(EngineEvent::OrderCanceled {
                            id: order.id,
                            remaining,
                        });
                    } else {
                        self.rest(&order, remaining);
                    }
                }
                self.events.push(EngineEvent::BookUpdated);
            }
            OrderType::Stop | OrderType::StopLimit => {
                // Unreachable: stops are converted by `activate_stop` before reaching here.
            }
        }
    }

    /// The price-time matching loop. Fills the aggressor against the best opposing orders in
    /// FIFO order, emitting a [`Trade`] per fill and applying self-trade prevention. Returns the
    /// aggressor's unfilled remainder.
    fn match_aggressor(
        &mut self,
        taker: &Order,
        limit: Option<Price>,
        mut remaining: Qty,
    ) -> (Qty, bool) {
        let mut stp_aborted = false;
        while !remaining.is_zero() {
            let Some(maker) = self.book.best_maker(taker.side.opposite()) else {
                break;
            };
            if !price_crosses(taker.side, limit, maker.price) {
                break;
            }
            if maker.account == taker.account {
                match self.stp {
                    StpPolicy::CancelNewest => {
                        stp_aborted = true;
                        break;
                    }
                    StpPolicy::CancelOldest => {
                        let rem = maker.quantity;
                        let _ = self.book.cancel(maker.id);
                        self.events.push(EngineEvent::OrderCanceled {
                            id: maker.id,
                            remaining: rem,
                        });
                        continue;
                    }
                    StpPolicy::DecrementBoth => {
                        let dec = remaining.min(maker.quantity);
                        let maker_left = maker.quantity.checked_sub(dec).unwrap_or(Qty::ZERO);
                        self.book.reduce_maker(maker.node, dec);
                        if maker_left.is_zero() {
                            self.events.push(EngineEvent::OrderCanceled {
                                id: maker.id,
                                remaining: Qty::ZERO,
                            });
                        }
                        remaining = remaining.checked_sub(dec).unwrap_or(Qty::ZERO);
                        continue;
                    }
                }
            }
            let fill = remaining.min(maker.quantity);
            let seq = self.next_seq();
            self.events.push(EngineEvent::Trade(Trade {
                seq,
                taker_order: taker.id,
                maker_order: maker.id,
                price: maker.price,
                quantity: fill,
                taker_side: taker.side,
            }));
            self.book.reduce_maker(maker.node, fill);
            self.last_trade_price = Some(maker.price);
            remaining = remaining.checked_sub(fill).unwrap_or(Qty::ZERO);
        }
        (remaining, stp_aborted)
    }

    fn rest(&mut self, order: &Order, qty: Qty) {
        if let Some(price) = order.limit_price {
            let _ = self
                .book
                .insert(order.id, order.account, order.side, price, qty, order.seq);
        }
    }

    fn reject(&mut self, id: crate::domain::OrderId, reason: RejectReason) {
        self.events.push(EngineEvent::OrderRejected { id, reason });
    }
}

/// Stateless structural validation. The gateway validates first; this is the engine's own guard
/// so the library is safe to call directly.
fn validate(order: &Order) -> Option<RejectReason> {
    if order.quantity.is_zero() {
        return Some(RejectReason::ZeroQuantity);
    }
    match order.order_type {
        OrderType::Limit | OrderType::PostOnly => {
            if order.limit_price.is_none() {
                return Some(RejectReason::MissingLimitPrice);
            }
        }
        OrderType::StopLimit => {
            if order.limit_price.is_none() {
                return Some(RejectReason::MissingLimitPrice);
            }
            if order.stop_price.is_none() {
                return Some(RejectReason::MissingStopPrice);
            }
        }
        OrderType::Stop => {
            if order.stop_price.is_none() {
                return Some(RejectReason::MissingStopPrice);
            }
        }
        OrderType::Market | OrderType::ImmediateOrCancel | OrderType::FillOrKill => {}
    }
    None
}
