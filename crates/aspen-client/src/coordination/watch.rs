//! Watch client for real-time key change notifications.

use std::sync::Arc;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for watch operations via the client RPC protocol.
///
/// NOTE: Watch operations require a streaming connection via LOG_SUBSCRIBER_ALPN
/// for actual real-time event delivery. The WatchClient provides methods that
/// will return an error directing users to use the streaming protocol.
///
/// For actual watch functionality, use the log subscription protocol directly:
/// - Connect to the node via LOG_SUBSCRIBER_ALPN ("aspen-logs")
/// - Send SubscribeRequest with key_prefix filter
/// - Receive streaming LogEntryMessage events
///
/// The log subscriber protocol (in `log_subscriber.rs`) provides:
/// - Prefix-based filtering
/// - Historical replay from any log index
/// - Keepalive messages for connection health
/// - Graceful handling of lag when subscribers fall behind
pub struct WatchClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> WatchClient<C> {
    /// Create a new watch client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Attempt to create a watch (redirects to streaming protocol).
    ///
    /// This method always returns an error with instructions to use the
    /// streaming protocol for actual watch functionality.
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to watch (empty string watches all keys)
    /// * `start_index` - Starting log index (0 = from beginning, u64::MAX = latest only)
    /// * `include_prev_value` - Include previous value in events
    pub async fn create(&self, prefix: String, start_index: u64, include_prev_value: bool) -> Result<WatchHandle> {
        use aspen_client_api::WatchCreateResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::WatchCreate {
                prefix,
                start_index,
                include_prev_value,
            })
            .await?;

        match response {
            ClientRpcResponse::WatchCreateResult(WatchCreateResultResponse {
                success,
                watch_id,
                current_index,
                error,
            }) => {
                if success {
                    Ok(WatchHandle {
                        watch_id: watch_id.unwrap_or(0),
                        current_index: current_index.unwrap_or(0),
                    })
                } else {
                    bail!("watch create failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for WatchCreate"),
        }
    }

    /// Cancel an active watch.
    ///
    /// This method always returns an error with instructions to use the
    /// streaming protocol for actual watch functionality.
    pub async fn cancel(&self, watch_id: u64) -> Result<()> {
        use aspen_client_api::WatchCancelResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::WatchCancel { watch_id }).await?;

        match response {
            ClientRpcResponse::WatchCancelResult(WatchCancelResultResponse { success, error, .. }) => {
                if success {
                    Ok(())
                } else {
                    bail!("watch cancel failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for WatchCancel"),
        }
    }

    /// Get status of active watches.
    ///
    /// This method always returns an error with instructions to use the
    /// streaming protocol for actual watch functionality.
    pub async fn status(&self, watch_id: Option<u64>) -> Result<Vec<WatchInfoLocal>> {
        use aspen_client_api::WatchStatusResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::WatchStatus { watch_id }).await?;

        match response {
            ClientRpcResponse::WatchStatusResult(WatchStatusResultResponse {
                success,
                watches,
                error,
            }) => {
                if success {
                    Ok(watches
                        .unwrap_or_default()
                        .into_iter()
                        .map(|w| WatchInfoLocal {
                            watch_id: w.watch_id,
                            prefix: w.prefix,
                            last_sent_index: w.last_sent_index,
                            events_sent: w.events_sent,
                            created_at_ms: w.created_at_ms,
                            include_prev_value: w.include_prev_value,
                        })
                        .collect())
                } else {
                    bail!("watch status failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for WatchStatus"),
        }
    }
}

/// Handle returned from watch creation.
#[derive(Debug, Clone)]
pub struct WatchHandle {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Current log index at watch creation time.
    pub current_index: u64,
}

/// Local representation of watch info.
#[derive(Debug, Clone)]
pub struct WatchInfoLocal {
    /// Unique watch ID.
    pub watch_id: u64,
    /// Key prefix being watched.
    pub prefix: String,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Number of events sent.
    pub events_sent: u64,
    /// Watch creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Whether the watch includes previous values.
    pub include_prev_value: bool,
}
