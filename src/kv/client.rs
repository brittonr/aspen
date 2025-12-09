//! Key-value client that forwards operations to the Raft actor.
//!
//! This module provides a focused client for key-value operations that implements
//! the `KeyValueStore` trait by delegating to a `RaftActor` through ractor's RPC
//! mechanism. The client ensures linearizable consistency by going through the Raft
//! consensus protocol.
//!
//! ## Design
//!
//! The client does not run as a separate actor; instead, it wraps an `ActorRef` and
//! uses ractor's `call_t!` macro for request-response messaging. This keeps the design
//! simple while providing clean separation between cluster control operations
//! (handled by `RaftControlClient`) and data plane operations (handled by `KvClient`).
//!
//! ## Usage
//!
//! ```ignore
//! let kv_client = KvClient::new(raft_actor_ref);
//! let write_req = WriteRequest {
//!     command: WriteCommand::Set {
//!         key: "foo".into(),
//!         value: "bar".into(),
//!     },
//! };
//! kv_client.write(write_req).await?;
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use ractor::{ActorRef, call_t};
use tracing::instrument;

use crate::api::{
    DeleteRequest, DeleteResult, KeyValueStore, KeyValueStoreError, ReadRequest, ReadResult,
    WriteRequest, WriteResult,
};
use crate::raft::RaftActorMessage;

/// Client for key-value operations via Raft consensus.
///
/// All operations are forwarded to a `RaftActor` which ensures linearizable
/// consistency through the Raft protocol. Writes go through consensus and are
/// replicated to a quorum before returning. Reads use ReadIndex to ensure
/// linearizability without going through the log.
#[derive(Clone)]
pub struct KvClient {
    raft_actor: ActorRef<RaftActorMessage>,
    timeout_ms: u64,
}

impl KvClient {
    /// Create a new KV client that forwards operations to the given Raft actor.
    ///
    /// Uses a default timeout of 500ms for operations. For custom timeout behavior,
    /// use `with_timeout()`.
    pub fn new(raft_actor: ActorRef<RaftActorMessage>) -> Self {
        Self::with_timeout(raft_actor, 500)
    }

    /// Create a KV client with a custom timeout in milliseconds.
    ///
    /// The timeout applies to each individual operation (write or read). If the
    /// operation does not complete within this time, an error is returned.
    pub fn with_timeout(raft_actor: ActorRef<RaftActorMessage>, timeout_ms: u64) -> Self {
        Self {
            raft_actor,
            timeout_ms,
        }
    }

    /// Wrap this client in an Arc for sharing across tasks.
    ///
    /// This is a convenience method since the HTTP layer and other components
    /// typically need `Arc<dyn KeyValueStore>`.
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

#[async_trait]
impl KeyValueStore for KvClient {
    #[instrument(skip(self, request), fields(command = ?request.command))]
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        call_t!(
            self.raft_actor,
            RaftActorMessage::Write,
            self.timeout_ms,
            request
        )
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?
    }

    #[instrument(skip(self), fields(key = %request.key))]
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        call_t!(
            self.raft_actor,
            RaftActorMessage::Read,
            self.timeout_ms,
            request
        )
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?
    }

    #[instrument(skip(self), fields(key = %request.key))]
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        call_t!(
            self.raft_actor,
            RaftActorMessage::Delete,
            self.timeout_ms,
            request
        )
        .map_err(|err| KeyValueStoreError::Failed {
            reason: err.to_string(),
        })?
    }
}
