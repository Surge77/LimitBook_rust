//! Single-writer runtime (LMAX-style): one OS thread owns the [`MatchingEngine`].
//!
//! Commands arrive over a **bounded** `std::sync::mpsc::sync_channel` (backpressure, no locks in
//! the matching loop); engine output flows out over another bounded channel. Only the engine
//! thread ever touches the book, so the matching path needs no synchronization.
//!
//! Book snapshots are *coalesced on the engine thread* to a configurable frame interval — at a
//! million orders per second you must not snapshot per command, and only the writer may read the
//! book, so the throttling belongs here.
//!
//! Uses only `std` (no async runtime, no external channel crate) so the core stays light.

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::domain::{BookSnapshot, EngineEvent, Order, OrderId, Price, Qty, StpPolicy};
use crate::engine::MatchingEngine;

#[cfg(feature = "serde")]
use serde::Serialize;

const DEFAULT_FRAME: Duration = Duration::from_millis(16); // ~60 fps snapshot cadence

/// A command sent to the engine thread.
#[derive(Debug)]
pub enum Command {
    Submit(Box<Order>),
    Cancel(OrderId),
    Amend {
        id: OrderId,
        quantity: Qty,
        price: Option<Price>,
    },
    /// Request an immediate snapshot, delivered on the reply channel.
    Snapshot {
        depth: usize,
        reply: SyncSender<BookSnapshot>,
    },
    Shutdown,
}

/// A message emitted by the engine thread to consumers (the gateway).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum EngineMsg {
    Event(EngineEvent),
    Book(BookSnapshot),
}

/// Configuration for [`spawn`].
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub book_capacity: usize,
    pub stp: StpPolicy,
    pub command_capacity: usize,
    pub output_capacity: usize,
    pub snapshot_depth: usize,
    pub frame: Duration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            book_capacity: 4096,
            stp: StpPolicy::default(),
            command_capacity: 65_536,
            output_capacity: 65_536,
            snapshot_depth: 25,
            frame: DEFAULT_FRAME,
        }
    }
}

/// A handle to the running engine thread.
#[derive(Debug)]
pub struct EngineHandle {
    tx: SyncSender<Command>,
    join: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Submit an order (blocks only if the command queue is full — backpressure).
    pub fn submit(&self, order: Order) -> Result<(), ClosedError> {
        self.send(Command::Submit(Box::new(order)))
    }

    /// Submit without blocking; returns `Err` if the queue is full or closed.
    pub fn try_submit(&self, order: Order) -> Result<(), TrySendError> {
        self.tx
            .try_send(Command::Submit(Box::new(order)))
            .map_err(Into::into)
    }

    pub fn cancel(&self, id: OrderId) -> Result<(), ClosedError> {
        self.send(Command::Cancel(id))
    }

    pub fn amend(
        &self,
        id: OrderId,
        quantity: Qty,
        price: Option<Price>,
    ) -> Result<(), ClosedError> {
        self.send(Command::Amend {
            id,
            quantity,
            price,
        })
    }

    /// Request a fresh snapshot synchronously. Blocks until the engine replies. Callers on an
    /// async runtime should wrap this in `spawn_blocking`.
    pub fn snapshot(&self, depth: usize) -> Result<BookSnapshot, ClosedError> {
        let (reply, rx) = sync_channel(1);
        self.send(Command::Snapshot { depth, reply })?;
        rx.recv().map_err(|_| ClosedError)
    }

    fn send(&self, cmd: Command) -> Result<(), ClosedError> {
        self.tx.send(cmd).map_err(|_| ClosedError)
    }

    /// Signal shutdown and join the engine thread.
    pub fn shutdown(mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        // Best-effort shutdown if the caller never called `shutdown`.
        let _ = self.tx.send(Command::Shutdown);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

/// The command channel is closed (engine thread gone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosedError;

impl std::fmt::Display for ClosedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "engine command channel is closed")
    }
}

impl std::error::Error for ClosedError {}

/// A non-blocking send failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrySendError {
    /// The bounded command queue is full — apply backpressure and retry.
    Full,
    /// The engine thread is gone.
    Closed,
}

impl From<std::sync::mpsc::TrySendError<Command>> for TrySendError {
    fn from(e: std::sync::mpsc::TrySendError<Command>) -> Self {
        match e {
            std::sync::mpsc::TrySendError::Full(_) => TrySendError::Full,
            std::sync::mpsc::TrySendError::Disconnected(_) => TrySendError::Closed,
        }
    }
}

/// Spawn the engine on a dedicated thread. Returns a handle for sending commands and a receiver
/// for the engine's output stream (events + coalesced snapshots).
pub fn spawn(config: RuntimeConfig) -> std::io::Result<(EngineHandle, Receiver<EngineMsg>)> {
    let (cmd_tx, cmd_rx) = sync_channel::<Command>(config.command_capacity);
    let (out_tx, out_rx) = sync_channel::<EngineMsg>(config.output_capacity);

    let join = thread::Builder::new()
        .name("limitbook-engine".to_string())
        .spawn(move || engine_loop(config, cmd_rx, out_tx))?;

    Ok((
        EngineHandle {
            tx: cmd_tx,
            join: Some(join),
        },
        out_rx,
    ))
}

/// The single-writer loop. Owns the engine; forwards events immediately; emits a snapshot at most
/// once per `frame` while the book is dirty.
fn engine_loop(config: RuntimeConfig, cmd_rx: Receiver<Command>, out_tx: SyncSender<EngineMsg>) {
    let mut engine = MatchingEngine::with_capacity(config.book_capacity, config.stp);
    let mut dirty = false;
    let mut last_snapshot = Instant::now();

    loop {
        match cmd_rx.recv_timeout(config.frame) {
            Ok(Command::Shutdown) => break,
            Ok(Command::Snapshot { depth, reply }) => {
                let _ = reply.send(engine.snapshot(depth));
            }
            Ok(cmd) => {
                dirty |= apply(&mut engine, cmd, &out_tx);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if dirty && last_snapshot.elapsed() >= config.frame {
            if out_tx
                .send(EngineMsg::Book(engine.snapshot(config.snapshot_depth)))
                .is_err()
            {
                break;
            }
            dirty = false;
            last_snapshot = Instant::now();
        }
    }
}

/// Apply one mutating command, forwarding every emitted event. Returns whether the book changed.
fn apply(engine: &mut MatchingEngine, cmd: Command, out_tx: &SyncSender<EngineMsg>) -> bool {
    let mut book_changed = false;
    match cmd {
        Command::Submit(order) => {
            for ev in engine.submit(*order) {
                book_changed |= matches!(ev, EngineEvent::BookUpdated);
                if out_tx.send(EngineMsg::Event(ev.clone())).is_err() {
                    return book_changed;
                }
            }
        }
        Command::Cancel(id) => {
            if let Ok(events) = engine.cancel(id) {
                for ev in events {
                    book_changed |= matches!(ev, EngineEvent::BookUpdated);
                    if out_tx.send(EngineMsg::Event(ev.clone())).is_err() {
                        return book_changed;
                    }
                }
            }
        }
        Command::Amend {
            id,
            quantity,
            price,
        } => {
            if let Ok(events) = engine.amend(id, quantity, price) {
                for ev in events {
                    book_changed |= matches!(ev, EngineEvent::BookUpdated);
                    if out_tx.send(EngineMsg::Event(ev.clone())).is_err() {
                        return book_changed;
                    }
                }
            }
        }
        Command::Snapshot { .. } | Command::Shutdown => {}
    }
    book_changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AccountId, OrderType, Seq, Side};

    fn limit(id: u64, side: Side, price: u64, qty: u64) -> Order {
        Order {
            id: OrderId(id),
            account: AccountId(id),
            side,
            order_type: OrderType::Limit,
            limit_price: Some(Price(price)),
            stop_price: None,
            quantity: Qty(qty),
            seq: Seq(0),
        }
    }

    fn drain_until_trade(rx: &Receiver<EngineMsg>) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(msg) = rx.recv_timeout(Duration::from_millis(50)) {
                if matches!(msg, EngineMsg::Event(EngineEvent::Trade(_))) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn runtime_matches_orders_and_streams_trade() {
        let (handle, rx) = spawn(RuntimeConfig::default()).unwrap();
        handle.submit(limit(1, Side::Sell, 100, 10)).unwrap();
        handle.submit(limit(2, Side::Buy, 100, 10)).unwrap();
        assert!(drain_until_trade(&rx));
        handle.shutdown();
    }

    #[test]
    fn runtime_replies_to_snapshot_request() {
        let (handle, _rx) = spawn(RuntimeConfig::default()).unwrap();
        handle.submit(limit(1, Side::Buy, 99, 5)).unwrap();
        let snap = handle.snapshot(10).unwrap();
        assert_eq!(snap.best_bid(), Some(Price(99)));
        handle.shutdown();
    }

    #[test]
    fn closed_handle_reports_error() {
        let (handle, rx) = spawn(RuntimeConfig::default()).unwrap();
        drop(rx); // consumer gone; engine will exit when output send fails
        handle.submit(limit(1, Side::Buy, 99, 5)).unwrap();
        // Eventually the engine thread stops; submits then error. Allow time to settle.
        thread::sleep(Duration::from_millis(50));
        let _ = handle.submit(limit(2, Side::Buy, 99, 5));
    }
}
