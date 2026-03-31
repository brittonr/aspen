//! Iroh-based write forwarding implementation.
//!
//! Forwards write requests from follower nodes to the Raft leader via iroh
//! QUIC using the client RPC protocol (CLIENT_ALPN).
//!
//! Connection reuse: a single QUIC connection to the leader is cached and
//! reused across writes. Multiple writes are multiplexed as separate
//! bidirectional streams on that connection — no per-write handshake.
//! The cached connection is evicted when the leader changes or when
//! `open_bi()` fails (dead connection).

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::messages::AuthenticatedRequest;
use aspen_client_api::messages::BatchWriteOperation;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::ErrorResponse;
use aspen_client_api::messages::MAX_CLIENT_MESSAGE_SIZE;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::Connection;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

use crate::types::NodeId;
use crate::write_forwarder::WriteForwarder;

/// Timeout for forwarded write operations (per-stream, not per-connection).
const FORWARD_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for establishing a new QUIC connection to the leader.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// CLIENT_ALPN for connecting to the leader's client protocol handler.
const CLIENT_ALPN: &[u8] = aspen_constants::network::CLIENT_ALPN;

/// Convert a `WriteCommand` to the corresponding `ClientRpcRequest`.
///
/// Most write commands map directly to a client RPC variant. The few that
/// don't (SetMulti, SetMultiWithTTL, DeleteMulti) are converted to batch
/// operations.
fn write_command_to_rpc_request(
    command: &WriteCommand,
    leader_id: NodeId,
) -> Result<ClientRpcRequest, KeyValueStoreError> {
    match command {
        WriteCommand::Set { key, value } => Ok(ClientRpcRequest::WriteKey {
            key: key.clone(),
            value: value.as_bytes().to_vec(),
        }),
        WriteCommand::Delete { key } => Ok(ClientRpcRequest::DeleteKey { key: key.clone() }),
        WriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => Ok(ClientRpcRequest::CompareAndSwapKey {
            key: key.clone(),
            expected: expected.as_ref().map(|v| v.as_bytes().to_vec()),
            new_value: new_value.as_bytes().to_vec(),
        }),
        WriteCommand::CompareAndDelete { key, expected } => Ok(ClientRpcRequest::CompareAndDeleteKey {
            key: key.clone(),
            expected: expected.as_bytes().to_vec(),
        }),
        WriteCommand::SetWithTTL { key, value, .. } => {
            // TTL forwarding: the leader applies the TTL. We forward as a
            // plain write — the TTL semantics are re-applied by the leader's
            // KV handler since it processes the full WriteKey path.
            // NOTE: This loses TTL info. If TTL forwarding becomes critical,
            // add a WriteKeyWithTTL variant to ClientRpcRequest.
            Ok(ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.as_bytes().to_vec(),
            })
        }
        WriteCommand::SetMulti { pairs } => {
            let operations = pairs
                .iter()
                .map(|(k, v)| BatchWriteOperation::Set {
                    key: k.clone(),
                    value: v.as_bytes().to_vec(),
                })
                .collect();
            Ok(ClientRpcRequest::BatchWrite { operations })
        }
        WriteCommand::SetMultiWithTTL { pairs, .. } => {
            // Same TTL note as SetWithTTL above.
            let operations = pairs
                .iter()
                .map(|(k, v)| BatchWriteOperation::Set {
                    key: k.clone(),
                    value: v.as_bytes().to_vec(),
                })
                .collect();
            Ok(ClientRpcRequest::BatchWrite { operations })
        }
        WriteCommand::DeleteMulti { keys } => {
            let operations = keys.iter().map(|k| BatchWriteOperation::Delete { key: k.clone() }).collect();
            Ok(ClientRpcRequest::BatchWrite { operations })
        }
        WriteCommand::Batch { operations } => {
            let rpc_ops = operations
                .iter()
                .map(|op| match op {
                    aspen_kv_types::BatchOperation::Set { key, value } => BatchWriteOperation::Set {
                        key: key.clone(),
                        value: value.as_bytes().to_vec(),
                    },
                    aspen_kv_types::BatchOperation::Delete { key } => BatchWriteOperation::Delete { key: key.clone() },
                })
                .collect();
            Ok(ClientRpcRequest::BatchWrite { operations: rpc_ops })
        }
        other => {
            // ConditionalBatch and any future complex operations.
            debug!(
                leader_id = leader_id.0,
                command = ?other,
                "cannot forward complex write command, returning NotLeader"
            );
            Err(KeyValueStoreError::NotLeader {
                leader: Some(leader_id.0),
                reason: "complex write commands cannot be forwarded".to_string(),
            })
        }
    }
}

/// Check if a response indicates a write success, regardless of the specific
/// response variant. Returns Ok for success, Err for failures.
fn interpret_write_response(response: ClientRpcResponse, leader_id: NodeId) -> Result<WriteResult, KeyValueStoreError> {
    match response {
        ClientRpcResponse::WriteResult(resp) => {
            if resp.is_success {
                Ok(WriteResult::default())
            } else {
                let reason = resp.error.unwrap_or_else(|| "write failed".to_string());
                check_not_leader_error(reason, leader_id)
            }
        }
        ClientRpcResponse::DeleteResult(resp) => {
            if resp.was_deleted {
                Ok(WriteResult::default())
            } else if let Some(err) = resp.error {
                check_not_leader_error(err, leader_id)
            } else {
                // Key didn't exist — still a successful operation
                Ok(WriteResult::default())
            }
        }
        ClientRpcResponse::CompareAndSwapResult(resp) => {
            if resp.is_success {
                Ok(WriteResult::default())
            } else if let Some(err) = resp.error {
                check_not_leader_error(err, leader_id)
            } else {
                // CAS condition failed (actual value didn't match expected).
                // This is a legitimate CAS failure, not a forwarding error.
                Err(KeyValueStoreError::CompareAndSwapFailed {
                    key: String::new(), // Key info not in CAS response
                    expected: None,
                    actual: resp.actual_value.map(|v| String::from_utf8_lossy(&v).to_string()),
                })
            }
        }
        ClientRpcResponse::BatchWriteResult(resp) => {
            if resp.is_success {
                Ok(WriteResult::default())
            } else {
                let reason = resp.error.unwrap_or_else(|| "batch write failed".to_string());
                check_not_leader_error(reason, leader_id)
            }
        }
        ClientRpcResponse::ConditionalBatchWriteResult(resp) => {
            if resp.is_success {
                Ok(WriteResult::default())
            } else {
                let reason = resp.error.unwrap_or_else(|| "conditional batch failed".to_string());
                check_not_leader_error(reason, leader_id)
            }
        }
        ClientRpcResponse::Error(ErrorResponse { code, message }) => {
            check_not_leader_error(format!("{code}: {message}"), leader_id)
        }
        other => {
            warn!(
                leader_id = leader_id.0,
                response_type = ?std::mem::discriminant(&other),
                "unexpected response type from forwarded write"
            );
            Err(KeyValueStoreError::Failed {
                reason: "unexpected response from leader".to_string(),
            })
        }
    }
}

/// If an error message suggests the target is also not the leader, return
/// NotLeader so the caller can retry. Otherwise return Failed.
fn check_not_leader_error(reason: String, leader_id: NodeId) -> Result<WriteResult, KeyValueStoreError> {
    if reason.contains("forward") || reason.contains("not the leader") || reason.contains("ForwardToLeader") {
        Err(KeyValueStoreError::NotLeader {
            leader: None,
            reason: format!("forwarding target {} is also not the leader", leader_id.0),
        })
    } else {
        Err(KeyValueStoreError::Failed { reason })
    }
}

/// Cached QUIC connection to the current leader.
///
/// Evicted when the leader changes (different `NodeId`) or when the
/// connection is dead (`open_bi()` fails).
struct CachedLeaderConnection {
    leader_id: NodeId,
    connection: Connection,
}

/// Forwards writes to the Raft leader via iroh QUIC client protocol.
///
/// Maintains a cached QUIC connection to the leader for stream multiplexing.
/// A single connection handles thousands of concurrent writes as separate
/// bidirectional streams — no per-write QUIC handshake.
pub struct IrohWriteForwarder {
    endpoint: Arc<Endpoint>,
    cached: AsyncMutex<Option<CachedLeaderConnection>>,
}

impl IrohWriteForwarder {
    /// Create a new write forwarder using the given iroh endpoint.
    pub fn new(endpoint: Arc<Endpoint>) -> Self {
        Self {
            endpoint,
            cached: AsyncMutex::new(None),
        }
    }

    /// Get or establish a QUIC connection to the leader.
    ///
    /// Returns the cached connection if it's for the same leader, otherwise
    /// connects fresh and caches the new connection.
    async fn get_connection(
        &self,
        leader_id: NodeId,
        leader_addr: &EndpointAddr,
    ) -> Result<Connection, KeyValueStoreError> {
        // Belt-and-suspenders: iroh rejects self-connections, catch it early
        // with a clear error instead of a cryptic QUIC failure.
        if leader_addr.id == self.endpoint.id() {
            return Err(KeyValueStoreError::NotLeader {
                leader: Some(leader_id.0),
                reason: "cannot forward to self (leader is this node)".to_string(),
            });
        }

        let mut cached = self.cached.lock().await;

        // Reuse if same leader and connection is still open
        if let Some(ref entry) = *cached {
            if entry.leader_id == leader_id {
                // Clone is cheap — Connection is Arc-wrapped internally
                return Ok(entry.connection.clone());
            }
            // Leader changed — drop old connection
            debug!(
                old_leader = entry.leader_id.0,
                new_leader = leader_id.0,
                "leader changed, dropping cached connection"
            );
        }

        // Establish new connection with a dedicated timeout
        let connection = timeout(CONNECT_TIMEOUT, self.endpoint.connect(leader_addr.clone(), CLIENT_ALPN))
            .await
            .map_err(|_| KeyValueStoreError::Timeout {
                duration_ms: CONNECT_TIMEOUT.as_millis() as u64,
            })?
            .map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to connect to leader {}: {e}", leader_id.0),
            })?;

        debug!(leader_id = leader_id.0, "cached new leader connection");
        *cached = Some(CachedLeaderConnection {
            leader_id,
            connection: connection.clone(),
        });

        Ok(connection)
    }

    /// Evict the cached connection (called when `open_bi` fails — dead connection).
    async fn evict_connection(&self, leader_id: NodeId) {
        let mut cached = self.cached.lock().await;
        if let Some(ref entry) = *cached
            && entry.leader_id == leader_id
        {
            debug!(leader_id = leader_id.0, "evicting dead leader connection");
            *cached = None;
        }
    }

    // execute_on_stream replaced by send_rpc_on_stream + forward_rpc
}

impl IrohWriteForwarder {
    /// Send an RPC request to the leader with automatic reconnect on dead connections.
    async fn forward_rpc(
        &self,
        leader_id: NodeId,
        leader_addr: &EndpointAddr,
        rpc_request: ClientRpcRequest,
    ) -> Result<ClientRpcResponse, KeyValueStoreError> {
        let result = timeout(FORWARD_TIMEOUT, async {
            let connection = self.get_connection(leader_id, leader_addr).await?;

            match Self::send_rpc_on_stream(&connection, &rpc_request, leader_id).await {
                Ok(resp) => Ok(resp),
                Err(KeyValueStoreError::Failed { ref reason })
                    if reason.contains("failed to open stream")
                        || reason.contains("truncated stream")
                        || reason.contains("failed to deserialize forwarded response")
                        || reason.contains("empty response") =>
                {
                    // Connection or stream is broken — evict and retry once on a fresh connection.
                    self.evict_connection(leader_id).await;
                    let fresh = self.get_connection(leader_id, leader_addr).await?;
                    Self::send_rpc_on_stream(&fresh, &rpc_request, leader_id).await
                }
                Err(e) => Err(e),
            }
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_timeout) => {
                warn!(
                    leader_id = leader_id.0,
                    timeout_secs = FORWARD_TIMEOUT.as_secs(),
                    "RPC forwarding to leader timed out"
                );
                Err(KeyValueStoreError::Timeout {
                    duration_ms: FORWARD_TIMEOUT.as_millis() as u64,
                })
            }
        }
    }

    /// Low-level: serialize, send, receive, deserialize on a single stream.
    async fn send_rpc_on_stream(
        connection: &Connection,
        rpc_request: &ClientRpcRequest,
        leader_id: NodeId,
    ) -> Result<ClientRpcResponse, KeyValueStoreError> {
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to open stream to leader {}: {e}", leader_id.0),
        })?;

        let authenticated = AuthenticatedRequest::unauthenticated(rpc_request.clone());
        let request_bytes = postcard::to_stdvec(&authenticated).map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to serialize forwarded request: {e}"),
        })?;

        send.write_all(&request_bytes).await.map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to send forwarded request: {e}"),
        })?;

        send.finish().map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to finish send stream: {e}"),
        })?;

        let response_bytes =
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to read forwarded response: {e}"),
            })?;

        // Detect truncated/empty responses before attempting deserialization.
        // Under QUIC load (e.g., concurrent snapshot transfers), streams can
        // deliver 0 bytes when the leader's handler fails to write a response
        // or the connection is disrupted. Return a retryable error instead of
        // a confusing postcard "Hit the end of buffer" message.
        if response_bytes.is_empty() {
            return Err(KeyValueStoreError::Failed {
                reason: format!("empty response from leader {} (truncated stream under load)", leader_id.0),
            });
        }

        postcard::from_bytes(&response_bytes).map_err(|e| KeyValueStoreError::Failed {
            reason: format!("failed to deserialize forwarded response ({} bytes): {e}", response_bytes.len()),
        })
    }
}

#[async_trait]
impl WriteForwarder for IrohWriteForwarder {
    async fn forward_write(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: WriteRequest,
    ) -> Result<WriteResult, KeyValueStoreError> {
        let rpc_request = write_command_to_rpc_request(&request.command, leader_id)?;
        let response = self.forward_rpc(leader_id, &leader_addr, rpc_request).await?;
        interpret_write_response(response, leader_id)
    }

    async fn forward_read(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: ReadRequest,
    ) -> Result<ReadResult, KeyValueStoreError> {
        let rpc_request = ClientRpcRequest::ReadKey {
            key: request.key.clone(),
        };
        let response = self.forward_rpc(leader_id, &leader_addr, rpc_request).await?;

        match response {
            ClientRpcResponse::ReadResult(resp) => {
                if let Some(ref err) = resp.error {
                    if err.contains("not leader") || err.contains("ForwardToLeader") {
                        return Err(KeyValueStoreError::NotLeader {
                            leader: None,
                            reason: err.clone(),
                        });
                    }
                    return Err(KeyValueStoreError::Failed { reason: err.clone() });
                }
                if resp.was_found {
                    let value = resp.value.map(|v| String::from_utf8_lossy(&v).to_string()).unwrap_or_default();
                    Ok(ReadResult {
                        kv: Some(KeyValueWithRevision {
                            key: request.key,
                            value,
                            version: 1,
                            create_revision: 0,
                            mod_revision: 0,
                        }),
                    })
                } else {
                    Err(KeyValueStoreError::NotFound { key: request.key })
                }
            }
            ClientRpcResponse::Error(e) => Err(KeyValueStoreError::Failed { reason: e.message }),
            _ => Err(KeyValueStoreError::Failed {
                reason: "unexpected response type for forwarded read".to_string(),
            }),
        }
    }

    async fn forward_scan(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: ScanRequest,
    ) -> Result<ScanResult, KeyValueStoreError> {
        let rpc_request = ClientRpcRequest::ScanKeys {
            prefix: request.prefix.clone(),
            limit: request.limit_results,
            continuation_token: request.continuation_token.clone(),
        };
        let response = self.forward_rpc(leader_id, &leader_addr, rpc_request).await?;

        match response {
            ClientRpcResponse::ScanResult(resp) => {
                if let Some(ref err) = resp.error {
                    if err.contains("not leader") || err.contains("ForwardToLeader") {
                        return Err(KeyValueStoreError::NotLeader {
                            leader: None,
                            reason: err.clone(),
                        });
                    }
                    return Err(KeyValueStoreError::Failed { reason: err.clone() });
                }
                let entries = resp
                    .entries
                    .into_iter()
                    .map(|e| KeyValueWithRevision {
                        key: e.key,
                        value: e.value,
                        version: e.version,
                        create_revision: e.create_revision,
                        mod_revision: e.mod_revision,
                    })
                    .collect::<Vec<_>>();
                Ok(ScanResult {
                    result_count: resp.count,
                    is_truncated: resp.is_truncated,
                    continuation_token: resp.continuation_token,
                    entries,
                })
            }
            ClientRpcResponse::Error(e) => Err(KeyValueStoreError::Failed { reason: e.message }),
            _ => Err(KeyValueStoreError::Failed {
                reason: "unexpected response type for forwarded scan".to_string(),
            }),
        }
    }
}
