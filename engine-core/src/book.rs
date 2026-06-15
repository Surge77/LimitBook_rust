//! The order book: two price-ordered sides, each a map of price level → FIFO queue.
//!
//! ## Why these structures
//!
//! - **`BTreeMap<Price, Level>` per side.** Best bid is the largest key
//!   (`bids.keys().next_back()`), best ask the smallest (`asks.keys().next()`) — O(log n) on
//!   either end, and iteration walks levels in price order for sweeps.
//! - **Intrusive doubly-linked list in a [`slab::Slab`] arena.** Every resting order is a node
//!   in a shared arena; level FIFOs link nodes by arena index (`prev`/`next`), not pointers.
//!   Cancel/amend is O(1): find the node via the id index, rewire two links, free the slot.
//! - **`HashMap<OrderId, usize>` index.** O(1) location of any resting order for cancel/amend.
//!
//! The slab reuses freed slots, so steady-state inserts/matches do **not** allocate. The only
//! allocations are amortized `BTreeMap`/`HashMap`/slab growth when a brand-new price level or a
//! new capacity high-water-mark appears — never on the per-fill path.

use std::collections::{BTreeMap, HashMap};

use slab::Slab;

use crate::domain::{
    AccountId, BookLevel, BookSnapshot, EngineError, OrderId, Price, Qty, Seq, Side,
};

/// A resting order stored in the arena, with intrusive FIFO links within its price level.
#[derive(Debug, Clone, Copy)]
struct Node {
    id: OrderId,
    account: AccountId,
    price: Price,
    side: Side,
    quantity: Qty,
    seq: Seq,
    prev: Option<usize>,
    next: Option<usize>,
}

/// One price level: a FIFO of nodes plus O(1) aggregates for snapshotting.
#[derive(Debug, Clone, Copy)]
struct Level {
    head: usize,
    tail: usize,
    total: Qty,
    count: u32,
}

/// A copy-out reference to the front (oldest) resting order on a side at the best price — what
/// an aggressor matches against. Returned by value so the matcher holds no borrow on the book.
#[derive(Debug, Clone, Copy)]
pub struct MakerRef {
    pub node: usize,
    pub price: Price,
    pub id: OrderId,
    pub account: AccountId,
    pub quantity: Qty,
    pub seq: Seq,
}

/// The two-sided limit order book.
#[derive(Debug)]
pub struct OrderBook {
    nodes: Slab<Node>,
    bids: BTreeMap<Price, Level>,
    asks: BTreeMap<Price, Level>,
    index: HashMap<OrderId, usize>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::with_capacity(1024)
    }
}

impl OrderBook {
    /// Pre-size the arena so that the common case never reallocates node storage.
    pub fn with_capacity(capacity: usize) -> Self {
        OrderBook {
            nodes: Slab::with_capacity(capacity),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: HashMap::with_capacity(capacity),
        }
    }

    #[inline]
    fn side_map(&self, side: Side) -> &BTreeMap<Price, Level> {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    #[inline]
    fn side_map_mut(&mut self, side: Side) -> &mut BTreeMap<Price, Level> {
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }

    /// Best (highest) bid price.
    #[inline]
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    /// Best (lowest) ask price.
    #[inline]
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    /// Best resting price on `side` (highest for buys, lowest for sells).
    #[inline]
    fn best_price(&self, side: Side) -> Option<Price> {
        match side {
            Side::Buy => self.best_bid(),
            Side::Sell => self.best_ask(),
        }
    }

    /// Number of resting orders.
    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    #[inline]
    pub fn contains(&self, id: OrderId) -> bool {
        self.index.contains_key(&id)
    }

    /// The oldest resting order on `side` at its best price — the next maker to fill.
    pub fn best_maker(&self, side: Side) -> Option<MakerRef> {
        let price = self.best_price(side)?;
        let level = self.side_map(side).get(&price)?;
        let node = &self.nodes[level.head];
        Some(MakerRef {
            node: level.head,
            price,
            id: node.id,
            account: node.account,
            quantity: node.quantity,
            seq: node.seq,
        })
    }

    /// Reduce a maker node by `fill`. If it reaches zero it is unlinked and freed, and its level
    /// is removed when empty. `fill` must not exceed the node's quantity.
    pub fn reduce_maker(&mut self, node_key: usize, fill: Qty) {
        let (side, price, became_empty) = {
            let node = &mut self.nodes[node_key];
            // Caller guarantees fill <= node.quantity; checked_sub upholds it defensively.
            node.quantity = node.quantity.checked_sub(fill).unwrap_or(Qty::ZERO);
            (node.side, node.price, node.quantity.is_zero())
        };
        if let Some(level) = self.side_map_mut(side).get_mut(&price) {
            level.total = level.total.checked_sub(fill).unwrap_or(Qty::ZERO);
        }
        if became_empty {
            self.remove_node(node_key);
        }
    }

    /// Rest an order at the tail of its price level (FIFO). The order's `seq` must already be set.
    pub fn insert(
        &mut self,
        id: OrderId,
        account: AccountId,
        side: Side,
        price: Price,
        quantity: Qty,
        seq: Seq,
    ) -> Result<(), EngineError> {
        if self.index.contains_key(&id) {
            return Err(EngineError::CapacityExhausted); // id reuse is guarded earlier; defensive
        }
        let node_key = self.nodes.insert(Node {
            id,
            account,
            price,
            side,
            quantity,
            seq,
            prev: None,
            next: None,
        });
        self.index.insert(id, node_key);

        // Borrow the map and the arena in separate, sequential scopes to keep the borrow
        // checker happy (they are disjoint fields but a method borrow spans all of `self`).
        let existing_tail = self.side_map(side).get(&price).map(|l| l.tail);
        match existing_tail {
            Some(old_tail) => {
                self.nodes[old_tail].next = Some(node_key);
                self.nodes[node_key].prev = Some(old_tail);
                if let Some(level) = self.side_map_mut(side).get_mut(&price) {
                    level.tail = node_key;
                    level.total = level.total.checked_add(quantity).unwrap_or(level.total);
                    level.count += 1;
                }
            }
            None => {
                self.side_map_mut(side).insert(
                    price,
                    Level {
                        head: node_key,
                        tail: node_key,
                        total: quantity,
                        count: 1,
                    },
                );
            }
        }
        Ok(())
    }

    /// Cancel a resting order, returning its remaining quantity.
    pub fn cancel(&mut self, id: OrderId) -> Result<Qty, EngineError> {
        let node_key = self
            .index
            .get(&id)
            .copied()
            .ok_or(EngineError::UnknownOrder(id))?;
        let remaining = self.nodes[node_key].quantity;
        self.remove_node(node_key);
        Ok(remaining)
    }

    /// Look up a resting order's side, price, account, and remaining quantity.
    pub fn get(&self, id: OrderId) -> Option<(Side, Price, AccountId, Qty)> {
        let node_key = *self.index.get(&id)?;
        let n = &self.nodes[node_key];
        Some((n.side, n.price, n.account, n.quantity))
    }

    /// Reduce a resting order's quantity in place without changing time priority.
    pub fn reduce_in_place(&mut self, id: OrderId, new_qty: Qty) -> Result<(), EngineError> {
        let node_key = self
            .index
            .get(&id)
            .copied()
            .ok_or(EngineError::UnknownOrder(id))?;
        let (side, price, old_qty) = {
            let n = &self.nodes[node_key];
            (n.side, n.price, n.quantity)
        };
        self.nodes[node_key].quantity = new_qty;
        if let Some(level) = self.side_map_mut(side).get_mut(&price) {
            let delta = old_qty.checked_sub(new_qty).unwrap_or(Qty::ZERO);
            level.total = level.total.checked_sub(delta).unwrap_or(Qty::ZERO);
        }
        Ok(())
    }

    /// Unlink a node from its level's FIFO and free its arena slot; drops the level if empty.
    fn remove_node(&mut self, node_key: usize) {
        let (id, side, price, qty, prev, next) = {
            let n = &self.nodes[node_key];
            (n.id, n.side, n.price, n.quantity, n.prev, n.next)
        };
        if let Some(p) = prev {
            self.nodes[p].next = next;
        }
        if let Some(nx) = next {
            self.nodes[nx].prev = prev;
        }
        if let Some(level) = self.side_map_mut(side).get_mut(&price) {
            if level.head == node_key {
                if let Some(nx) = next {
                    level.head = nx;
                }
            }
            if level.tail == node_key {
                if let Some(p) = prev {
                    level.tail = p;
                }
            }
            level.total = level.total.checked_sub(qty).unwrap_or(Qty::ZERO);
            level.count = level.count.saturating_sub(1);
            if level.count == 0 {
                self.side_map_mut(side).remove(&price);
            }
        }
        self.index.remove(&id);
        self.nodes.remove(node_key);
    }

    /// Total resting quantity an aggressor on `taker_side` could match against, given an optional
    /// `limit` price (`None` = market, all opposing liquidity). Used for fill-or-kill feasibility.
    ///
    /// Ignores self-trade prevention — a FOK from an account that owns part of the opposing book
    /// may report as fillable but then be STP-aborted. This is documented and accepted.
    pub fn available_against(&self, taker_side: Side, limit: Option<Price>) -> Qty {
        let mut total = Qty::ZERO;
        match taker_side {
            Side::Buy => {
                for (price, level) in self.asks.iter() {
                    if let Some(l) = limit {
                        if *price > l {
                            break;
                        }
                    }
                    total = total.checked_add(level.total).unwrap_or(total);
                }
            }
            Side::Sell => {
                for (price, level) in self.bids.iter().rev() {
                    if let Some(l) = limit {
                        if *price < l {
                            break;
                        }
                    }
                    total = total.checked_add(level.total).unwrap_or(total);
                }
            }
        }
        total
    }

    /// Build an aggregated L2 snapshot, capped at `depth` levels per side (0 = all levels).
    pub fn snapshot(&self, depth: usize, seq: Seq) -> BookSnapshot {
        let take = |it: &mut dyn Iterator<Item = (&Price, &Level)>| -> Vec<BookLevel> {
            let mut out = Vec::new();
            for (price, level) in it {
                out.push(BookLevel {
                    price: *price,
                    quantity: level.total,
                });
                if depth != 0 && out.len() >= depth {
                    break;
                }
            }
            out
        };
        BookSnapshot {
            bids: take(&mut self.bids.iter().rev()),
            asks: take(&mut self.asks.iter()),
            seq,
        }
    }
}
