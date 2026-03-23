//! Iroh-based write forwarding implementation.
//!
//! Forwards write requests from follower nodes to the Raft leader via iroh
//! QUIC using the client RPC protocol (CLIENT_ALPN).

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::messages::AuthenticatedRequest;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::MAX_CLIENT_MESSAGE_SIZE;
use aspen_client_api::messages::WriteResultResponse;
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
        // Convert WriteCommand to ClientRpcRequest
        let rpc_request = match &request.command {
            WriteCommand::Set { key, value } => ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.as_bytes().to_vec(),
            },
            WriteCommand::Delete { key } => ClientRpcRequest::DeleteKey { key: key.clone() },
            other => {
                // Complex operations (CAS, batch) can't be forwarded via simple
                // client RPC. Return NotLeader so the caller's retry loop handles it.
                debug!(
                    leader_id = leader_id.0,
                    command = ?other,
                    "cannot forward complex write command, returning NotLeader"
                );
                return Err(KeyValueStoreError::NotLeader {
                    leader: Some(leader_id.0),
                    reason: "complex write commands cannot be forwarded".to_string(),
                });
            }
        };

        // Connect to leader and send the request
        let result = timeout(FORWARD_TIMEOUT, async {
            let connection = self.endpoint.connect(leader_addr.clone(), CLIENT_ALPN).await.map_err(|e| {
                KeyValueStoreError::Failed {
                    reason: format!("failed to connect to leader {}: {e}", leader_id.0),
                }
            })?;

            let (mut send, mut recv) = connection.open_bi().await.map_err(|e| KeyValueStoreError::Failed {
                reason: format!("failed to open stream to leader {}: {e}", leader_id.0),
            })?;

            // Wrap as unauthenticated (internal forwarding, leader trusts cluster peers)
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

            match response {
                ClientRpcResponse::WriteResult(WriteResultResponse { is_success: true, .. }) => {
                    Ok(WriteResult::default())
                }
                ClientRpcResponse::WriteResult(WriteResultResponse {
                    is_success: false,
                    error,
                }) => {
                    let reason = error.unwrap_or_else(|| "unknown error".to_string());
                    // Check if the leader also returned NotLeader (stale hint)
                    if reason.contains("forward") || reason.contains("not the leader") {
                        Err(KeyValueStoreError::NotLeader {
                            leader: None,
                            reason: format!("forwarding target {leader_id:?} is also not the leader"),
                        })
                    } else {
                        Err(KeyValueStoreError::Failed { reason })
                    }
                }
                ClientRpcResponse::DeleteResult(resp) => {
                    if resp.was_deleted {
                        Ok(WriteResult::default())
                    } else {
                        Err(KeyValueStoreError::Failed {
                            reason: resp.error.unwrap_or_else(|| "delete failed".to_string()),
                        })
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
