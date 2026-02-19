//! P2P Automerge document synchronization protocol.
//!
//! This module implements an ALPN-based protocol handler for synchronizing
//! Automerge documents between peers over Iroh QUIC connections.
//!
//! # Protocol
//!
//! The sync protocol uses Automerge's built-in sync mechanism which is based
//! on efficient bloom filter-based set reconciliation. Most syncs complete in
//! a single round trip.
//!
//! ## Wire Format
//!
//! Messages are postcard-serialized with length prefix:
//! ```text
//! [4 bytes: message length][N bytes: postcard message]
//! ```
//!
//! ## Message Flow
//!
//! 1. Initiator sends `SyncRequest { document_id }`
//! 2. Responder sends `SyncResponse { accepted: true }` or rejects
//! 3. Both peers exchange `SyncMessage` until sync is complete
//! 4. Either peer sends `SyncComplete` to signal done
//!
//! # Tiger Style
//!
//! - Bounded connection count via semaphore
//! - Bounded message sizes
//! - Explicit resource cleanup on shutdown

use std::sync::Arc;

use automerge::AutoCommit;
use automerge::sync;
use automerge::sync::SyncDoc;
use iroh::endpoint::Connection;
use iroh::endpoint::RecvStream;
use iroh::endpoint::SendStream;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::warn;

use crate::DocumentId;
use crate::DocumentStore;
use crate::constants::MAX_SYNC_MESSAGE_SIZE;

/// ALPN identifier for Automerge sync protocol.
pub const AUTOMERGE_SYNC_ALPN: &[u8] = b"automerge-sync/1";

/// Maximum concurrent sync connections.
const MAX_SYNC_CONNECTIONS: usize = 32;

/// Maximum concurrent streams per connection.
const MAX_STREAMS_PER_CONNECTION: usize = 4;

/// Maximum sync rounds before giving up (prevents infinite loops).
const MAX_SYNC_ROUNDS: usize = 100;

// ============================================================================
// Protocol Messages
// ============================================================================

/// Messages exchanged during document synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncProtocolMessage {
    /// Initial request to sync a specific document.
    SyncRequest {
        /// Document ID to synchronize.
        document_id: String,
        /// Optional signed capability token for authorization.
        capability: Option<Vec<u8>>,
    },

    /// Response to sync request.
    SyncResponse {
        /// Whether the sync request was accepted.
        accepted: bool,
        /// Error message if rejected.
        error: Option<String>,
    },

    /// Automerge sync message containing changes.
    SyncMessage {
        /// Raw sync message bytes from Automerge.
        data: Vec<u8>,
    },

    /// Signal that sync is complete from this peer's perspective.
    SyncComplete,

    /// Error during sync.
    SyncError {
        /// Error message.
        message: String,
    },
}

impl SyncProtocolMessage {
    /// Serialize message with length prefix.
    pub fn encode(&self) -> Result<Vec<u8>, postcard::Error> {
        let data = postcard::to_stdvec(self)?;
        let len = (data.len() as u32).to_be_bytes();
        let mut result = Vec::with_capacity(4 + data.len());
        result.extend_from_slice(&len);
        result.extend_from_slice(&data);
        Ok(result)
    }

    /// Read and deserialize a length-prefixed message from stream.
    pub async fn read_from(recv: &mut RecvStream) -> Result<Self, SyncError> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf).await.map_err(|e| SyncError::Io(e.to_string()))?;

        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_SYNC_MESSAGE_SIZE {
            return Err(SyncError::MessageTooLarge { size: len });
        }

        // Read message data
        let mut data = vec![0u8; len];
        recv.read_exact(&mut data).await.map_err(|e| SyncError::Io(e.to_string()))?;

        // Deserialize
        postcard::from_bytes(&data).map_err(|e| SyncError::Deserialize(e.to_string()))
    }

    /// Write message to stream.
    pub async fn write_to(&self, send: &mut SendStream) -> Result<(), SyncError> {
        let data = self.encode().map_err(|e| SyncError::Serialize(e.to_string()))?;
        send.write_all(&data).await.map_err(|e| SyncError::Io(e.to_string()))?;
        Ok(())
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors during sync protocol.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("deserialization error: {0}")]
    Deserialize(String),

    #[error("message too large: {size} bytes (max {MAX_SYNC_MESSAGE_SIZE})")]
    MessageTooLarge { size: usize },

    #[error("document not found: {0}")]
    DocumentNotFound(String),

    #[error("sync rejected: {0}")]
    Rejected(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("automerge error: {0}")]
    Automerge(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("max sync rounds exceeded")]
    MaxRoundsExceeded,
}

// ============================================================================
// Protocol Handler
// ============================================================================

/// Protocol handler for Automerge document synchronization.
///
/// Accepts incoming sync connections and handles the document sync protocol.
/// If `verifier` is set, every sync request must include a valid
/// [`CapabilityToken`].
pub struct AutomergeSyncHandler<S: DocumentStore> {
    /// Document store for loading/saving documents.
    store: Arc<S>,
    /// If set, incoming sync requests must present a valid CapabilityToken.
    verifier: Option<Arc<aspen_auth::TokenVerifier>>,
    /// Semaphore for bounding concurrent connections.
    connection_semaphore: Arc<Semaphore>,
}

impl<S: DocumentStore> std::fmt::Debug for AutomergeSyncHandler<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutomergeSyncHandler").field("max_connections", &MAX_SYNC_CONNECTIONS).finish()
    }
}

impl<S: DocumentStore + 'static> AutomergeSyncHandler<S> {
    /// Create a new sync handler that accepts all connections (no auth).
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            verifier: None,
            connection_semaphore: Arc::new(Semaphore::new(MAX_SYNC_CONNECTIONS)),
        }
    }

    /// Create a sync handler that requires capability tokens signed by this key.
    pub fn with_capability_auth(store: Arc<S>, issuer_public_key: iroh::PublicKey) -> Self {
        let verifier = aspen_auth::TokenVerifier::new().with_trusted_root(issuer_public_key);
        Self {
            store,
            verifier: Some(Arc::new(verifier)),
            connection_semaphore: Arc::new(Semaphore::new(MAX_SYNC_CONNECTIONS)),
        }
    }

    /// Handle a sync connection.
    async fn handle_connection(&self, connection: Connection) -> Result<(), SyncError> {
        let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION));

        loop {
            // Accept bidirectional stream
            let stream = match connection.accept_bi().await {
                Ok(stream) => stream,
                Err(e) => {
                    // Connection closed
                    debug!(error = %e, "sync connection closed");
                    break;
                }
            };

            // Try to acquire stream permit
            let permit = match stream_semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("max sync streams reached, dropping stream");
                    continue;
                }
            };

            // Spawn task to handle this stream
            let store = self.store.clone();
            let verifier = self.verifier.clone();
            let remote_id = connection.remote_id();
            let (send, recv) = stream;
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = handle_sync_stream(store, recv, send, verifier, remote_id).await {
                    error!(error = %e, "sync stream error");
                }
            });
        }

        Ok(())
    }
}

impl<S: DocumentStore + 'static> ProtocolHandler for AutomergeSyncHandler<S> {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        // Try to acquire connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                return Err(AcceptError::from_err(std::io::Error::other("max sync connections reached")));
            }
        };

        // Handle the connection
        let result = self.handle_connection(connection).await;

        // Release permit
        drop(permit);

        result.map_err(|e| AcceptError::from_err(std::io::Error::other(e.to_string())))
    }

    async fn shutdown(&self) {
        self.connection_semaphore.close();
    }
}

// ============================================================================
// Stream Handling
// ============================================================================

/// Handle a single sync stream (responder side).
async fn handle_sync_stream<S: DocumentStore>(
    store: Arc<S>,
    mut recv: RecvStream,
    mut send: SendStream,
    verifier: Option<Arc<aspen_auth::TokenVerifier>>,
    remote_id: iroh::PublicKey,
) -> Result<(), SyncError> {
    // Read sync request
    let request = SyncProtocolMessage::read_from(&mut recv).await?;

    let (document_id, capability_bytes) = match request {
        SyncProtocolMessage::SyncRequest {
            document_id,
            capability,
        } => (document_id, capability),
        _ => {
            let error = SyncProtocolMessage::SyncError {
                message: "expected SyncRequest".into(),
            };
            error.write_to(&mut send).await?;
            return Err(SyncError::Protocol("expected SyncRequest".into()));
        }
    };

    debug!(document_id = %document_id, "received sync request");

    // Parse document ID
    let doc_id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            let response = SyncProtocolMessage::SyncResponse {
                accepted: false,
                error: Some(format!("invalid document ID: {}", e)),
            };
            response.write_to(&mut send).await?;
            return Err(SyncError::Protocol(format!("invalid document ID: {}", e)));
        }
    };

    // Verify capability if auth is required
    if let Some(ref verifier) = verifier {
        let cap_bytes = match capability_bytes {
            Some(bytes) => bytes,
            None => {
                let response = SyncProtocolMessage::SyncResponse {
                    accepted: false,
                    error: Some("capability token required".into()),
                };
                response.write_to(&mut send).await?;
                return Err(SyncError::Rejected("capability token required".into()));
            }
        };
        let token = match aspen_auth::CapabilityToken::decode(&cap_bytes) {
            Ok(t) => t,
            Err(e) => {
                let response = SyncProtocolMessage::SyncResponse {
                    accepted: false,
                    error: Some(format!("invalid token: {}", e)),
                };
                response.write_to(&mut send).await?;
                return Err(SyncError::Rejected(format!("invalid token: {}", e)));
            }
        };
        // Automerge documents are stored at "automerge:{doc_id}" keys in the KV store.
        // Sync is bidirectional so we require Write permission (which subsumes Read
        // for the Full capability variant).
        let operation = aspen_auth::Operation::Write {
            key: format!("automerge:{}", document_id),
            value: vec![],
        };
        if let Err(e) = verifier.authorize(&token, &operation, Some(&remote_id)) {
            let response = SyncProtocolMessage::SyncResponse {
                accepted: false,
                error: Some(format!("unauthorized: {}", e)),
            };
            response.write_to(&mut send).await?;
            return Err(SyncError::Rejected(format!("unauthorized: {}", e)));
        }
        debug!(document_id = %document_id, remote = %remote_id, "capability verified");
    }

    // Load document
    let mut doc = match store.get(&doc_id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            // Auto-create document when not found
            debug!(document_id = %document_id, "auto-creating document");
            if let Err(e) = store.create(Some(doc_id.clone()), None).await {
                let response = SyncProtocolMessage::SyncResponse {
                    accepted: false,
                    error: Some(format!("failed to create document: {}", e)),
                };
                response.write_to(&mut send).await?;
                return Err(SyncError::Store(e.to_string()));
            }
            // Load the freshly created document
            match store.get(&doc_id).await {
                Ok(Some(doc)) => doc,
                Ok(None) => {
                    let response = SyncProtocolMessage::SyncResponse {
                        accepted: false,
                        error: Some("failed to load newly created document".into()),
                    };
                    response.write_to(&mut send).await?;
                    return Err(SyncError::Store("failed to load newly created document".into()));
                }
                Err(e) => {
                    let response = SyncProtocolMessage::SyncResponse {
                        accepted: false,
                        error: Some(format!("failed to load document: {}", e)),
                    };
                    response.write_to(&mut send).await?;
                    return Err(SyncError::Store(e.to_string()));
                }
            }
        }
        Err(e) => {
            let response = SyncProtocolMessage::SyncResponse {
                accepted: false,
                error: Some(format!("failed to load document: {}", e)),
            };
            response.write_to(&mut send).await?;
            return Err(SyncError::Store(e.to_string()));
        }
    };

    // Accept the sync
    let response = SyncProtocolMessage::SyncResponse {
        accepted: true,
        error: None,
    };
    response.write_to(&mut send).await?;

    // Run sync loop
    run_sync_loop(&store, &doc_id, &mut doc, &mut recv, &mut send).await?;

    // Save document after sync
    if let Err(e) = store.save(&doc_id, &mut doc).await {
        error!(error = %e, "failed to save document after sync");
        return Err(SyncError::Store(e.to_string()));
    }

    debug!(document_id = %document_id, "sync complete");
    Ok(())
}

/// Run the sync message exchange loop.
async fn run_sync_loop<S: DocumentStore>(
    store: &Arc<S>,
    doc_id: &DocumentId,
    doc: &mut AutoCommit,
    recv: &mut RecvStream,
    send: &mut SendStream,
) -> Result<(), SyncError> {
    let mut sync_state = sync::State::new();
    let mut rounds = 0;
    let mut peer_complete = false;

    loop {
        rounds += 1;
        if rounds > MAX_SYNC_ROUNDS {
            return Err(SyncError::MaxRoundsExceeded);
        }

        // Generate our sync message
        let our_message = doc.sync().generate_sync_message(&mut sync_state);

        // Send our message if we have one
        if let Some(msg) = our_message {
            let sync_msg = SyncProtocolMessage::SyncMessage { data: msg.encode() };
            sync_msg.write_to(send).await?;
        } else if peer_complete {
            // Both sides are done
            let complete = SyncProtocolMessage::SyncComplete;
            complete.write_to(send).await?;
            break;
        } else {
            // We're done but peer might not be, signal completion
            let complete = SyncProtocolMessage::SyncComplete;
            complete.write_to(send).await?;
        }

        // Read peer's message
        let peer_msg = SyncProtocolMessage::read_from(recv).await?;

        match peer_msg {
            SyncProtocolMessage::SyncMessage { data } => {
                // Decode and apply
                let msg = sync::Message::decode(&data)
                    .map_err(|e| SyncError::Automerge(format!("invalid sync message: {}", e)))?;

                doc.sync()
                    .receive_sync_message(&mut sync_state, msg)
                    .map_err(|e| SyncError::Automerge(format!("sync failed: {}", e)))?;

                // Periodically save to avoid losing progress
                if rounds % 10 == 0
                    && let Err(e) = store.save(doc_id, doc).await
                {
                    warn!(error = %e, "failed to save document during sync");
                }
            }
            SyncProtocolMessage::SyncComplete => {
                peer_complete = true;
                // Check if we're also done
                if doc.sync().generate_sync_message(&mut sync_state).is_none() {
                    break;
                }
            }
            SyncProtocolMessage::SyncError { message } => {
                return Err(SyncError::Protocol(message));
            }
            _ => {
                return Err(SyncError::Protocol("unexpected message during sync".into()));
            }
        }
    }

    Ok(())
}

// ============================================================================
// Client-Side Sync
// ============================================================================

/// Sync a document with a remote peer.
///
/// This is the initiator side of the sync protocol.
/// If `capability` is provided, it's sent with the request for authorization.
pub async fn sync_with_peer<S: DocumentStore>(
    store: &S,
    document_id: &DocumentId,
    connection: &Connection,
) -> Result<(), SyncError> {
    sync_with_peer_cap(store, document_id, connection, None).await
}

/// Sync a document with a remote peer, presenting a capability token.
pub async fn sync_with_peer_cap<S: DocumentStore>(
    store: &S,
    document_id: &DocumentId,
    connection: &Connection,
    capability: Option<&aspen_auth::CapabilityToken>,
) -> Result<(), SyncError> {
    // Open bidirectional stream
    let (mut send, mut recv) =
        connection.open_bi().await.map_err(|e| SyncError::Io(format!("failed to open stream: {}", e)))?;

    // Send sync request with optional capability
    let request = SyncProtocolMessage::SyncRequest {
        document_id: document_id.to_string(),
        capability: capability.and_then(|c| c.encode().ok()),
    };
    request.write_to(&mut send).await?;

    // Read response
    let response = SyncProtocolMessage::read_from(&mut recv).await?;

    match response {
        SyncProtocolMessage::SyncResponse { accepted, error } => {
            if !accepted {
                return Err(SyncError::Rejected(error.unwrap_or_else(|| "unknown".into())));
            }
        }
        _ => {
            return Err(SyncError::Protocol("expected SyncResponse".into()));
        }
    }

    // Load document
    let mut doc = match store.get(document_id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(SyncError::DocumentNotFound(document_id.to_string())),
        Err(e) => return Err(SyncError::Store(e.to_string())),
    };

    // Run sync loop (same as responder)
    let store_arc = Arc::new(StoreBridge { store });
    run_sync_loop(&store_arc, document_id, &mut doc, &mut recv, &mut send).await?;

    // Save document
    store.save(document_id, &mut doc).await.map_err(|e| SyncError::Store(e.to_string()))?;

    // Finish stream
    send.finish().map_err(|e| SyncError::Io(e.to_string()))?;

    debug!(document_id = %document_id, "sync with peer complete");
    Ok(())
}

/// Bridge to use a &S as Arc<S> in sync loop.
struct StoreBridge<'a, S: DocumentStore> {
    store: &'a S,
}

#[async_trait::async_trait]
impl<'a, S: DocumentStore> DocumentStore for StoreBridge<'a, S> {
    async fn create(
        &self,
        id: Option<DocumentId>,
        metadata: Option<crate::DocumentMetadata>,
    ) -> crate::AutomergeResult<DocumentId> {
        self.store.create(id, metadata).await
    }

    async fn get(&self, id: &DocumentId) -> crate::AutomergeResult<Option<AutoCommit>> {
        self.store.get(id).await
    }

    async fn save(&self, id: &DocumentId, doc: &mut AutoCommit) -> crate::AutomergeResult<()> {
        self.store.save(id, doc).await
    }

    async fn delete(&self, id: &DocumentId) -> crate::AutomergeResult<bool> {
        self.store.delete(id).await
    }

    async fn apply_changes(
        &self,
        id: &DocumentId,
        changes: Vec<crate::DocumentChange>,
    ) -> crate::AutomergeResult<crate::ApplyResult> {
        self.store.apply_changes(id, changes).await
    }

    async fn merge(
        &self,
        target_id: &DocumentId,
        source_id: &DocumentId,
    ) -> crate::AutomergeResult<crate::ApplyResult> {
        self.store.merge(target_id, source_id).await
    }

    async fn list(&self, options: crate::ListOptions) -> crate::AutomergeResult<crate::ListResult> {
        self.store.list(options).await
    }

    async fn get_metadata(&self, id: &DocumentId) -> crate::AutomergeResult<Option<crate::DocumentMetadata>> {
        self.store.get_metadata(id).await
    }

    async fn exists(&self, id: &DocumentId) -> crate::AutomergeResult<bool> {
        self.store.exists(id).await
    }

    async fn get_heads(&self, id: &DocumentId) -> crate::AutomergeResult<Vec<String>> {
        self.store.get_heads(id).await
    }

    async fn list_ids(&self, namespace: Option<&str>, limit: u32) -> crate::AutomergeResult<Vec<DocumentId>> {
        self.store.list_ids(namespace, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_encode_decode() {
        let msg = SyncProtocolMessage::SyncRequest {
            document_id: "test-doc".to_string(),
            capability: None,
        };

        let encoded = msg.encode().unwrap();
        assert!(encoded.len() > 4); // Has length prefix

        // Verify length prefix is correct
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(len, encoded.len() - 4);
    }

    #[test]
    fn test_sync_complete_message() {
        let msg = SyncProtocolMessage::SyncComplete;
        let encoded = msg.encode().unwrap();
        assert!(encoded.len() > 4);
    }

    #[test]
    fn test_sync_error_message() {
        let msg = SyncProtocolMessage::SyncError {
            message: "test error".to_string(),
        };
        let encoded = msg.encode().unwrap();
        assert!(encoded.len() > 4);
    }
}
