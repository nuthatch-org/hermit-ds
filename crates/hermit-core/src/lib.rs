//! Core pipeline abstractions for Hermit Data Service.
//!
//! The runtime is three composable pieces, exactly as in seahorn:
//!   `Substrate` streams events -> `Handler` purely transforms them to a `ChangeSet`
//!   -> `Sink` applies the `ChangeSet` to storage. Handlers do NO I/O and are
//!   deterministic — the same event always yields the same `ChangeSet`. This is what
//!   makes the pipeline testable and a future PoI possible.

use anyhow::Result;
use futures::Stream;

/// Position in the fork-resolution lifecycle of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// New event on the current fork.
    New,
    /// Roll back a previously emitted `New` event (reorg).
    Undo,
    /// Permanently canonical — safe to promote in the sink.
    Irreversible,
}

/// Opaque substrate cursor for resuming. Consumers never inspect the bytes.
#[derive(Debug, Clone, Default)]
pub struct Cursor(pub Vec<u8>);

/// A typed value for the entity store.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    U64(u64),
    I64(i64),
    Bool(bool),
    Bytes(Vec<u8>),
    Null,
}

impl From<&str> for Value {
    fn from(s: &str) -> Self { Value::String(s.to_owned()) }
}
impl From<String> for Value {
    fn from(s: String) -> Self { Value::String(s) }
}
impl From<u64> for Value {
    fn from(n: u64) -> Self { Value::U64(n) }
}
impl From<i64> for Value {
    fn from(n: i64) -> Self { Value::I64(n) }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self { Value::Bool(b) }
}

/// A single mutation to the entity store.
#[derive(Debug, Clone)]
pub enum EntityChange {
    Upsert {
        entity_type: &'static str,
        id: String,
        fields: Vec<(&'static str, Value)>,
    },
    Delete {
        entity_type: &'static str,
        id: String,
    },
}

/// The output of a pure handler — a *description* of what should change. Zero I/O.
#[derive(Debug, Clone)]
pub struct ChangeSet {
    pub block: u64,
    pub id: Vec<u8>,
    pub step: Step,
    pub cursor: Cursor,
    pub changes: Vec<EntityChange>,
}

impl ChangeSet {
    pub fn empty(block: u64, id: Vec<u8>, step: Step, cursor: Cursor) -> Self {
        Self { block, id, step, cursor, changes: Vec::new() }
    }
    pub fn push(mut self, change: EntityChange) -> Self {
        self.changes.push(change);
        self
    }
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// One unit of work from the substrate: a block/tx with its raw payload to decode.
#[derive(Debug, Clone)]
pub struct SubstrateEvent {
    pub block: u64,
    pub id: Vec<u8>,
    pub step: Step,
    pub cursor: Cursor,
    /// Raw bytes the handler decodes (a tx, a log, a protobuf frame — chain-specific).
    pub payload: Vec<u8>,
}

/// A source of substrate events. The runtime drives `stream()` to completion.
pub trait Substrate {
    fn stream(&self, from: Option<Cursor>) -> impl Stream<Item = Result<SubstrateEvent>> + Send + '_;
}

/// A pure, deterministic, I/O-free transform from an event to a `ChangeSet`.
pub trait Handler: Send + Sync {
    fn handle(&self, event: &SubstrateEvent) -> ChangeSet;
}

impl Handler for Box<dyn Handler> {
    fn handle(&self, event: &SubstrateEvent) -> ChangeSet { (**self).handle(event) }
}
impl<H: Handler> Handler for &H {
    fn handle(&self, event: &SubstrateEvent) -> ChangeSet { (*self).handle(event) }
}

/// Runs multiple handlers against the same event and merges their ChangeSets.
pub struct MultiHandler {
    handlers: Vec<Box<dyn Handler>>,
}

impl MultiHandler {
    pub fn new(handlers: Vec<Box<dyn Handler>>) -> Self { Self { handlers } }
}

impl Handler for MultiHandler {
    fn handle(&self, event: &SubstrateEvent) -> ChangeSet {
        let mut cs = ChangeSet::empty(event.block, event.id.clone(), event.step, event.cursor.clone());
        for h in &self.handlers {
            cs.changes.extend(h.handle(event).changes);
        }
        cs
    }
}

/// A sink that applies a `ChangeSet` to storage. Owns all I/O.
pub trait Sink: Send + Sync {
    fn apply(&self, changeset: &ChangeSet) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl<K: Sink> Sink for &K {
    fn apply(&self, changeset: &ChangeSet) -> impl std::future::Future<Output = Result<()>> + Send {
        (*self).apply(changeset)
    }
}
