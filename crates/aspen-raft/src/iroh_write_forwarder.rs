//! Iroh-based write forwarding implementation.
//!
//! Forwards write requests from follower nodes to the Raft leader via iroh
//! QUIC using the client RPC protocol (CLIENT_ALPN).

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::messages::AuthenticatedRequest;
use aspen_client_api::messages::BatchWriteOperation;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::MAX_CLIENT_MESSAGE_SIZE;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

use crate::types::NodeId;
use crate::write_forwarder::WriteForwarder;

/// Timeout for forwarded write operations.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Forwards writes to the Raft leader via iroh QUIC client protocol.
pub struct IrohWriteForwarder {
    endpoint: Arc<Endpoint>,
}

impl IrohWriteForwarder {
    /// Create a new write forwarder using the given iroh endpoint.
    pub fn new(endpoint: Arc<Endpoint>) -> Self {
        Self { endpoint }
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

        let result = timeout(FORWARD_TIMEOUT, async {
            let connection = self.endpoint.connect(leader_addr.clone(), CLIENT_ALPN).await.map_err(|e| {
                KeyValueStoreError::Failed {
                    reason: format!("failed to connect to leader {}: {e}", leader_id.0),
                }
            })?;

            let (mut send, mut recv) = connection.open_bi().await.map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to open stream to leader {}: {e}", leader_id.0),
            })?;

            let authenticated = AuthenticatedRequest::unauthenticated(rpc_request);
            let request_bytes = postcard::to_stdvec(&authenticated).map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to serialize forwarded write: {e}"),
            })?;

            send.write_all(&request_bytes).await.map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to send forwarded write: {e}"),
            })?;

            send.finish().map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to finish send stream: {e}"),
            })?;

            let response_bytes =
                recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.map_err(|e| KeyValueStoreError::Failed {
                    reason: format!("failed to read forwarded write response: {e}"),
                })?;

            let response: ClientRpcResponse =
                postcard::from_bytes(&response_bytes).map_err(|e| KeyValueStoreError::Failed {
                    reason: format!("failed to deserialize forwarded write response: {e}"),
                })?;

            interpret_write_response(response, leader_id)
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_timeout) => {
                warn!(
                    leader_id = leader_id.0,
                    timeout_secs = FORWARD_TIMEOUT.as_secs(),
                    "write forwarding to leader timed out"
                );
                Err(KeyValueStoreError::Timeout {
                    duration_ms: FORWARD_TIMEOUT.as_millis() as u64,
                })
            }
        }
    }
}
