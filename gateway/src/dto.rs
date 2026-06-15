//! Wire types: REST request/response bodies, WebSocket server messages, and validation.
//!
//! All prices are integer **ticks** (1 tick = 0.01 quote units); the frontend divides by 100
//! for display. The gateway validates every inbound order here — the engine trusts only what
//! passes this boundary.

use engine_core::domain::{
    AccountId, BookSnapshot, EngineEvent, Order, OrderId, OrderType, Price, Qty, Seq, Side, Trade,
};
use serde::{Deserialize, Serialize};

/// Inbound order from `POST /orders`.
#[derive(Debug, Deserialize)]
pub struct NewOrderRequest {
    pub side: Side,
    #[serde(rename = "order_type")]
    pub order_type: OrderType,
    #[serde(default)]
    pub price: Option<u64>,
    #[serde(default)]
    pub stop_price: Option<u64>,
    pub quantity: u64,
    #[serde(default)]
    pub account: Option<u64>,
}

/// Inbound amend from `PUT /orders/{id}`.
#[derive(Debug, Deserialize)]
pub struct AmendRequest {
    pub quantity: u64,
    #[serde(default)]
    pub price: Option<u64>,
}

/// A single field-level validation error.
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: &'static str,
    pub message: &'static str,
}

impl NewOrderRequest {
    /// Validate and convert into an engine [`Order`] with the server-assigned `id`.
    pub fn into_order(self, id: u64) -> Result<Order, Vec<FieldError>> {
        let mut errors = Vec::new();
        if self.quantity == 0 {
            errors.push(FieldError {
                field: "quantity",
                message: "must be greater than zero",
            });
        }
        let needs_limit = matches!(
            self.order_type,
            OrderType::Limit | OrderType::PostOnly | OrderType::StopLimit
        );
        if needs_limit && self.price.is_none() {
            errors.push(FieldError {
                field: "price",
                message: "required for this order type",
            });
        }
        let needs_stop = matches!(self.order_type, OrderType::Stop | OrderType::StopLimit);
        if needs_stop && self.stop_price.is_none() {
            errors.push(FieldError {
                field: "stop_price",
                message: "required for stop orders",
            });
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Order {
            id: OrderId(id),
            account: AccountId(self.account.unwrap_or(0)),
            side: self.side,
            order_type: self.order_type,
            limit_price: self.price.map(Price),
            stop_price: self.stop_price.map(Price),
            quantity: Qty(self.quantity),
            seq: Seq(0),
        })
    }
}

/// `POST /orders` acknowledgement — the engine is asynchronous, so this confirms intake only;
/// the fill/reject outcome arrives over the WebSocket.
#[derive(Debug, Serialize)]
pub struct OrderAck {
    pub id: u64,
    pub accepted: bool,
}

/// A book level on the wire.
#[derive(Debug, Clone, Serialize)]
pub struct LevelDto {
    pub price: u64,
    pub quantity: u64,
}

/// Messages pushed to WebSocket clients (tagged by `type`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Book {
        bids: Vec<LevelDto>,
        asks: Vec<LevelDto>,
        seq: u64,
        best_bid: Option<u64>,
        best_ask: Option<u64>,
        spread: Option<u64>,
    },
    Trade {
        seq: u64,
        price: u64,
        quantity: u64,
        taker_side: Side,
        taker_order: u64,
        maker_order: u64,
    },
    OrderAccepted {
        id: u64,
    },
    OrderRejected {
        id: u64,
        reason: String,
    },
    OrderCanceled {
        id: u64,
        remaining: u64,
    },
    OrderAmended {
        id: u64,
        quantity: u64,
        price: Option<u64>,
        repriced: bool,
    },
}

impl ServerMessage {
    /// Build a `Book` message from an engine snapshot.
    pub fn from_snapshot(s: &BookSnapshot) -> Self {
        let to_levels = |levels: &[engine_core::domain::BookLevel]| {
            levels
                .iter()
                .map(|l| LevelDto {
                    price: l.price.get(),
                    quantity: l.quantity.get(),
                })
                .collect()
        };
        ServerMessage::Book {
            bids: to_levels(&s.bids),
            asks: to_levels(&s.asks),
            seq: s.seq.get(),
            best_bid: s.best_bid().map(|p| p.get()),
            best_ask: s.best_ask().map(|p| p.get()),
            spread: s.spread(),
        }
    }

    /// Convert an engine event into a server message, if it maps to one the UI cares about.
    pub fn from_event(ev: &EngineEvent) -> Option<Self> {
        match ev {
            EngineEvent::Trade(t) => Some(Self::trade(t)),
            EngineEvent::OrderAccepted { id } => {
                Some(ServerMessage::OrderAccepted { id: id.get() })
            }
            EngineEvent::OrderRejected { id, reason } => Some(ServerMessage::OrderRejected {
                id: id.get(),
                reason: format!("{reason:?}"),
            }),
            EngineEvent::OrderCanceled { id, remaining } => Some(ServerMessage::OrderCanceled {
                id: id.get(),
                remaining: remaining.get(),
            }),
            EngineEvent::OrderAmended {
                id,
                new_quantity,
                new_price,
                repriced,
            } => Some(ServerMessage::OrderAmended {
                id: id.get(),
                quantity: new_quantity.get(),
                price: new_price.map(|p| p.get()),
                repriced: *repriced,
            }),
            EngineEvent::BookUpdated => None, // book state is carried by snapshots
        }
    }

    fn trade(t: &Trade) -> Self {
        ServerMessage::Trade {
            seq: t.seq.get(),
            price: t.price.get(),
            quantity: t.quantity.get(),
            taker_side: t.taker_side,
            taker_order: t.taker_order.get(),
            maker_order: t.maker_order.get(),
        }
    }
}
