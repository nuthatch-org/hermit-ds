//! Hermit Data Service indexer.
//!
//! Drives the pipeline: `Substrate` (here a synthetic mock) -> `Handler` (decode) ->
//! `Sink` (here stdout). Swap `MockSubstrate` for a real one (gRPC/firehose/RPC stream)
//! and `StdoutSink` for a Postgres sink, then put a `horizon-core` gateway in front of
//! the query layer for TAP payments. See README.md.

use anyhow::Result;
use futures::StreamExt;
use hermit_core::{
    ChangeSet, Cursor, EntityChange, Handler, Sink, Step, Substrate, SubstrateEvent, Value,
};

// ── Runtime loop ────────────────────────────────────────────────────────────────

async fn run<S, H, K>(substrate: S, handler: H, sink: K, from: Option<Cursor>) -> Result<()>
where
    S: Substrate,
    H: Handler,
    K: Sink,
{
    let mut stream = std::pin::pin!(substrate.stream(from));
    while let Some(result) = stream.next().await {
        let event = result?;
        let changeset = handler.handle(&event);
        if !changeset.is_empty() {
            sink.apply(&changeset).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermit_indexer=info".into()),
        )
        .init();

    tracing::info!("Hermit Data Service indexer starting (mock substrate -> stdout sink)");
    run(MockSubstrate::new(16), ExampleHandler, StdoutSink, None).await
}

// ── Example handler (replace with your decode logic) ──────────────────────────────

/// Decodes a raw event payload into entity changes. Pure and deterministic — no I/O.
struct ExampleHandler;

impl Handler for ExampleHandler {
    fn handle(&self, event: &SubstrateEvent) -> ChangeSet {
        // Demo: treat the payload as a little-endian u64 "amount".
        let amount = {
            let mut buf = [0u8; 8];
            let n = event.payload.len().min(8);
            buf[..n].copy_from_slice(&event.payload[..n]);
            u64::from_le_bytes(buf)
        };
        ChangeSet::empty(event.block, event.id.clone(), event.step, event.cursor.clone()).push(
            EntityChange::Upsert {
                entity_type: "Event",
                id: hex(&event.id),
                fields: vec![
                    ("block", Value::from(event.block)),
                    ("amount", Value::from(amount)),
                ],
            },
        )
    }
}

// ── Mock substrate (replace with gRPC/firehose/RPC) ───────────────────────────────

struct MockSubstrate {
    count: u64,
}

impl MockSubstrate {
    fn new(count: u64) -> Self {
        Self { count }
    }
}

impl Substrate for MockSubstrate {
    fn stream(
        &self,
        _from: Option<Cursor>,
    ) -> impl futures::Stream<Item = Result<SubstrateEvent>> + Send + '_ {
        let count = self.count;
        async_stream::stream! {
            for block in 0..count {
                let id = block.to_be_bytes().to_vec();
                yield Ok(SubstrateEvent {
                    block,
                    id,
                    step: Step::New,
                    cursor: Cursor(block.to_le_bytes().to_vec()),
                    payload: (block * 1000).to_le_bytes().to_vec(),
                });
            }
        }
    }
}

// ── Stdout sink (dev; replace with Postgres) ──────────────────────────────────────

struct StdoutSink;

impl Sink for StdoutSink {
    async fn apply(&self, cs: &ChangeSet) -> Result<()> {
        let step = match cs.step {
            Step::New => "NEW  ",
            Step::Undo => "UNDO ",
            Step::Irreversible => "FINAL",
        };
        for change in &cs.changes {
            if let EntityChange::Upsert { entity_type, id, fields } = change {
                tracing::info!("[block {:>6}] [{step}] {entity_type} {id} {:?}", cs.block,
                    fields.iter().map(|(k, _)| *k).collect::<Vec<_>>());
            }
        }
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
