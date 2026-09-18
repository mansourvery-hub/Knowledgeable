//! v2 generation-protocol stream registry (T12a).
//!
//! The client's resumable transport (`useResumableSSE`) splits a turn into
//! two steps: `POST /api/agents/chat/:endpoint` returns a JSON start ticket
//! (`streamId`), then `GET /api/agents/chat/stream/:stream_id` attaches to
//! the live SSE. This registry holds in-flight turns between those calls.
//!
//! Correctness notes:
//! - Frames are `serde_json::Value` payloads (the `data:` JSON); transport
//!   framing stays in the route handlers.
//! - `push` maintains a small snapshot (created / latest delta / annotations
//!   / final) so resumes and racing subscribers converge without replaying
//!   the whole turn.
//! - Entries are pruned after [`ENTRY_TTL_SECS`] so abandoned turns cannot
//!   leak memory. Single-user local mode needs no cross-instance sharing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub const STREAM_CHANNEL_CAPACITY: usize = 128;
const ENTRY_TTL_SECS: u64 = 600;

pub type StreamRegistry = Arc<Mutex<HashMap<String, Arc<StreamEntry>>>>;

pub fn new_registry() -> StreamRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Creates an entry, registers it, and arms its TTL pruner. Returns the entry
/// for the turn driver to push frames into.
pub fn new_registry_entry(
    registry: StreamRegistry,
    stream_id: String,
    conversation_id: uuid::Uuid,
    assistant_id: uuid::Uuid,
) -> Arc<StreamEntry> {
    let entry = Arc::new(StreamEntry::new(stream_id.clone(), conversation_id, assistant_id));
    if let Ok(mut map) = registry.lock() {
        map.insert(stream_id.clone(), entry.clone());
    }
    StreamEntry::spawn_pruner(registry, stream_id);
    entry
}

#[derive(Debug, Clone, Default)]
struct Snapshot {
    created: Option<serde_json::Value>,
    last_delta: Option<serde_json::Value>,
    annotations: Option<serde_json::Value>,
    final_payload: Option<serde_json::Value>,
    done: bool,
}

pub struct StreamEntry {
    pub stream_id: String,
    pub conversation_id: uuid::Uuid,
    pub assistant_id: uuid::Uuid,
    pub generation_created_at: i64,
    tx: broadcast::Sender<serde_json::Value>,
    snapshot: Mutex<Snapshot>,
}

impl StreamEntry {
    pub fn new(stream_id: String, conversation_id: uuid::Uuid, assistant_id: uuid::Uuid) -> Self {
        let (tx, _) = broadcast::channel(STREAM_CHANNEL_CAPACITY);
        Self {
            stream_id,
            conversation_id,
            assistant_id,
            generation_created_at: chrono::Utc::now().timestamp_millis(),
            tx,
            snapshot: Mutex::new(Snapshot::default()),
        }
    }

    /// Record a frame and fan it out to live subscribers. Never blocks or
    /// panics when nobody listens (`send` errors only mean zero receivers).
    /// A frame carrying `final` closes the entry.
    pub fn push(&self, frame: serde_json::Value) {
        if let Ok(mut snap) = self.snapshot.lock() {
            if frame.get("final").is_some() {
                snap.final_payload = Some(frame.clone());
                snap.done = true;
            } else if frame.get("created").is_some() {
                snap.created = Some(frame.clone());
            } else if frame.get("concept_annotations").is_some() {
                snap.annotations = Some(frame.clone());
            } else if frame.get("text").is_some() {
                snap.last_delta = Some(frame.clone());
            }
        }
        let _ = self.tx.send(frame);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<serde_json::Value> {
        self.tx.subscribe()
    }

    pub fn is_done(&self) -> bool {
        self.snapshot.lock().map(|s| s.done).unwrap_or(false)
    }

    /// Frames a (re)attaching consumer needs first, in stream order. Includes
    /// `final` when the turn already completed so resumes terminate.
    pub fn snapshot_frames(&self) -> Vec<serde_json::Value> {
        match self.snapshot.lock() {
            Ok(snap) => [
                snap.created.clone(),
                snap.last_delta.clone(),
                snap.annotations.clone(),
                snap.final_payload.clone(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Drop the entry after its TTL so finished or orphaned turns (client
    /// navigated away, server never reaped) cannot accumulate.
    pub fn spawn_pruner(registry: StreamRegistry, stream_id: String) {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(ENTRY_TTL_SECS)).await;
            if let Ok(mut map) = registry.lock() {
                map.remove(&stream_id);
            }
        });
    }
}

/// Latest tracked turn for a conversation (max epoch), if any. Used by the
/// stream-status endpoint the client polls to authorize terminal teardown.
pub fn latest_for_conversation(
    registry: &StreamRegistry,
    conversation_id: uuid::Uuid,
) -> Option<Arc<StreamEntry>> {
    registry
        .lock()
        .ok()?
        .values()
        .filter(|e| e.conversation_id == conversation_id)
        .max_by_key(|e| e.generation_created_at)
        .cloned()
}
