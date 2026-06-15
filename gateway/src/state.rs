//! Shared application state and the engine-output drain bridge.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use engine_core::{EngineHandle, EngineMsg};
use metrics_exporter_prometheus::PrometheusHandle;
use tokio::sync::broadcast;

use crate::dto::ServerMessage;

const RECENT_TRADES_CAP: usize = 200;
const BROADCAST_CAP: usize = 1024;

/// Cloneable handle to all shared gateway state.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<EngineHandle>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    pub latest_book: Arc<RwLock<Option<ServerMessage>>>,
    pub recent_trades: Arc<RwLock<VecDeque<ServerMessage>>>,
    pub next_id: Arc<AtomicU64>,
    pub sim_running: Arc<AtomicBool>,
    pub sim_rate: Arc<AtomicU64>,
    pub prometheus: PrometheusHandle,
}

impl AppState {
    pub fn new(engine: EngineHandle, prometheus: PrometheusHandle) -> Self {
        let (broadcast, _) = broadcast::channel(BROADCAST_CAP);
        AppState {
            engine: Arc::new(engine),
            broadcast,
            latest_book: Arc::new(RwLock::new(None)),
            recent_trades: Arc::new(RwLock::new(VecDeque::with_capacity(RECENT_TRADES_CAP))),
            next_id: Arc::new(AtomicU64::new(1)),
            sim_running: Arc::new(AtomicBool::new(false)),
            sim_rate: Arc::new(AtomicU64::new(200)),
            prometheus,
        }
    }

    /// Allocate the next server-side order id.
    pub fn alloc_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// Spawn the std-thread bridge: drains the engine's blocking output channel, updates cached
/// state + Prometheus metrics, and fans messages out to WebSocket subscribers. Runs on its own
/// thread so it never blocks the tokio runtime.
pub fn spawn_drain(rx: std::sync::mpsc::Receiver<EngineMsg>, state: AppState) {
    std::thread::Builder::new()
        .name("limitbook-drain".to_string())
        .spawn(move || {
            for msg in rx {
                match msg {
                    EngineMsg::Book(snap) => {
                        update_book_metrics(&snap);
                        let book = ServerMessage::from_snapshot(&snap);
                        if let Ok(mut guard) = state.latest_book.write() {
                            *guard = Some(book.clone());
                        }
                        let _ = state.broadcast.send(book);
                    }
                    EngineMsg::Event(ev) => {
                        record_event_metrics(&ev);
                        if let Some(server_msg) = ServerMessage::from_event(&ev) {
                            if matches!(server_msg, ServerMessage::Trade { .. }) {
                                push_recent_trade(&state, server_msg.clone());
                            }
                            let _ = state.broadcast.send(server_msg);
                        }
                    }
                }
            }
        })
        .expect("failed to spawn drain thread");
}

fn push_recent_trade(state: &AppState, trade: ServerMessage) {
    if let Ok(mut trades) = state.recent_trades.write() {
        if trades.len() >= RECENT_TRADES_CAP {
            trades.pop_front();
        }
        trades.push_back(trade);
    }
}

fn record_event_metrics(ev: &engine_core::EngineEvent) {
    use engine_core::EngineEvent::*;
    match ev {
        Trade(t) => {
            metrics::counter!("limitbook_trades_total").increment(1);
            metrics::counter!("limitbook_traded_quantity_total").increment(t.quantity.get());
        }
        OrderAccepted { .. } => metrics::counter!("limitbook_orders_accepted_total").increment(1),
        OrderRejected { .. } => metrics::counter!("limitbook_orders_rejected_total").increment(1),
        OrderCanceled { .. } => metrics::counter!("limitbook_orders_canceled_total").increment(1),
        _ => {}
    }
}

fn update_book_metrics(snap: &engine_core::BookSnapshot) {
    let bid_depth: u64 = snap.bids.iter().map(|l| l.quantity.get()).sum();
    let ask_depth: u64 = snap.asks.iter().map(|l| l.quantity.get()).sum();
    metrics::gauge!("limitbook_bid_depth").set(bid_depth as f64);
    metrics::gauge!("limitbook_ask_depth").set(ask_depth as f64);
    if let Some(spread) = snap.spread() {
        metrics::gauge!("limitbook_spread_ticks").set(spread as f64);
    }
    if let Some(bid) = snap.best_bid() {
        metrics::gauge!("limitbook_best_bid").set(bid.get() as f64);
    }
    if let Some(ask) = snap.best_ask() {
        metrics::gauge!("limitbook_best_ask").set(ask.get() as f64);
    }
}
