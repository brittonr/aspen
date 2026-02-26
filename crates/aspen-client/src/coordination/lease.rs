//! Lease client for time-limited key ownership.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

use super::CoordinationRpc;

/// Client for lease operations.
///
/// Provides a high-level API for creating, managing, and querying leases
/// over the client RPC protocol. Leases are similar to etcd's lease model:
///
/// - Leases have a TTL (time-to-live)
/// - Keys can be attached to leases
/// - When a lease expires or is revoked, all attached keys are deleted
/// - Leases can be refreshed via keepalive
///
/// ## Usage
///
/// ```ignore
/// use aspen::client::coordination::LeaseClient;
///
/// // Create a lease client
/// let leases = LeaseClient::new(rpc_client);
///
/// // Grant a new lease with 60 second TTL
/// let lease = leases.grant(60).await?;
/// println!("Lease ID: {}", lease.lease_id);
///
/// // Write keys attached to the lease
/// leases.put_with_lease("mykey", b"myvalue", lease.lease_id).await?;
///
/// // Refresh the lease (reset TTL)
/// leases.keepalive(lease.lease_id).await?;
///
/// // Query lease info
/// let info = leases.time_to_live(lease.lease_id, true).await?;
/// println!("TTL remaining: {}s, keys: {:?}", info.remaining_ttl_seconds, info.keys);
///
/// // Revoke the lease (deletes all attached keys)
/// leases.revoke(lease.lease_id).await?;
/// ```
pub struct LeaseClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> LeaseClient<C> {
    /// Create a new lease client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Grant a new lease with the specified TTL.
    ///
    /// # Arguments
    /// * `ttl_seconds` - Time-to-live in seconds
    ///
    /// # Returns
    /// A `LeaseGrantResult` containing the lease ID and granted TTL.
    pub async fn grant(&self, ttl_seconds: u32) -> Result<LeaseGrantResult> {
        self.grant_with_id(ttl_seconds, None).await
    }

    /// Grant a new lease with a specific ID and TTL.
    ///
    /// # Arguments
    /// * `ttl_seconds` - Time-to-live in seconds
    /// * `lease_id` - Optional specific lease ID (None = auto-generate)
    ///
    /// # Returns
    /// A `LeaseGrantResult` containing the lease ID and granted TTL.
    pub async fn grant_with_id(&self, ttl_seconds: u32, lease_id: Option<u64>) -> Result<LeaseGrantResult> {
        use aspen_client_api::LeaseGrantResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LeaseGrant { ttl_seconds, lease_id })
            .await?;

        match response {
            ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                is_success,
                lease_id,
                ttl_seconds,
                error,
            }) => {
                if is_success {
                    Ok(LeaseGrantResult {
                        lease_id: lease_id.unwrap_or(0),
                        ttl_seconds: ttl_seconds.unwrap_or(0),
                    })
                } else {
                    bail!("lease grant failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LeaseGrant"),
        }
    }

    /// Revoke a lease and delete all attached keys.
    ///
    /// # Arguments
    /// * `lease_id` - The lease ID to revoke
    ///
    /// # Returns
    /// A `LeaseRevokeResult` containing the number of keys deleted.
    pub async fn revoke(&self, lease_id: u64) -> Result<LeaseRevokeResult> {
        use aspen_client_api::LeaseRevokeResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseRevoke { lease_id }).await?;

        match response {
            ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                is_success,
                keys_deleted,
                error,
            }) => {
                if is_success {
                    Ok(LeaseRevokeResult {
                        keys_deleted: keys_deleted.unwrap_or(0),
                    })
                } else {
                    bail!("lease revoke failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LeaseRevoke"),
        }
    }

    /// Refresh a lease's TTL (keepalive).
    ///
    /// This resets the lease's deadline to TTL from now. Should be called
    /// periodically to prevent the lease from expiring.
    ///
    /// # Arguments
    /// * `lease_id` - The lease ID to refresh
    ///
    /// # Returns
    /// A `LeaseKeepaliveResult` containing the remaining TTL.
    pub async fn keepalive(&self, lease_id: u64) -> Result<LeaseKeepaliveResult> {
        use aspen_client_api::LeaseKeepaliveResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseKeepalive { lease_id }).await?;

        match response {
            ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                is_success,
                lease_id: returned_id,
                ttl_seconds,
                error,
            }) => {
                if is_success {
                    Ok(LeaseKeepaliveResult {
                        lease_id: returned_id.unwrap_or(lease_id),
                        ttl_seconds: ttl_seconds.unwrap_or(0),
                    })
                } else {
                    bail!("lease keepalive failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LeaseKeepalive"),
        }
    }

    /// Query lease information including TTL and optionally attached keys.
    ///
    /// # Arguments
    /// * `lease_id` - The lease ID to query
    /// * `include_keys` - Whether to include the list of attached keys
    ///
    /// # Returns
    /// A `LeaseTimeToLiveResult` with lease metadata.
    pub async fn time_to_live(&self, lease_id: u64, include_keys: bool) -> Result<LeaseTimeToLiveResult> {
        use aspen_client_api::LeaseTimeToLiveResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LeaseTimeToLive {
                lease_id,
                should_include_keys: include_keys,
            })
            .await?;

        match response {
            ClientRpcResponse::LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse {
                is_success,
                lease_id: returned_id,
                granted_ttl_seconds,
                remaining_ttl_seconds,
                keys,
                error,
            }) => {
                if is_success {
                    Ok(LeaseTimeToLiveResult {
                        lease_id: returned_id.unwrap_or(lease_id),
                        granted_ttl_seconds: granted_ttl_seconds.unwrap_or(0),
                        remaining_ttl_seconds: remaining_ttl_seconds.unwrap_or(0),
                        keys: keys.unwrap_or_default(),
                    })
                } else {
                    bail!("lease time-to-live query failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LeaseTimeToLive"),
        }
    }

    /// List all active leases.
    ///
    /// # Returns
    /// A vector of `LeaseInfoLocal` for each active lease.
    pub async fn list(&self) -> Result<Vec<LeaseInfoLocal>> {
        use aspen_client_api::LeaseListResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseList).await?;

        match response {
            ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
                is_success,
                leases,
                error,
            }) => {
                if is_success {
                    Ok(leases
                        .unwrap_or_default()
                        .into_iter()
                        .map(|info| LeaseInfoLocal {
                            lease_id: info.lease_id,
                            granted_ttl_seconds: info.granted_ttl_seconds,
                            remaining_ttl_seconds: info.remaining_ttl_seconds,
                        })
                        .collect())
                } else {
                    bail!("lease list failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LeaseList"),
        }
    }

    /// Write a key-value pair attached to a lease.
    ///
    /// When the lease expires or is revoked, the key will be automatically deleted.
    ///
    /// # Arguments
    /// * `key` - The key to write
    /// * `value` - The value to write
    /// * `lease_id` - The lease ID to attach the key to
    pub async fn put_with_lease(&self, key: &str, value: &[u8], lease_id: u64) -> Result<()> {
        use aspen_client_api::WriteResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::WriteKeyWithLease {
                key: key.to_string(),
                value: value.to_vec(),
                lease_id,
            })
            .await?;

        match response {
            ClientRpcResponse::WriteResult(WriteResultResponse { is_success, error }) => {
                if is_success {
                    Ok(())
                } else {
                    bail!("write with lease failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for WriteKeyWithLease"),
        }
    }

    /// Start automatic keepalive for a lease.
    ///
    /// Spawns a background task that periodically refreshes the lease's TTL.
    /// The keepalive runs at the specified interval (typically TTL/3 is recommended).
    ///
    /// # Arguments
    /// * `lease_id` - The lease ID to keep alive
    /// * `interval` - How often to send keepalive requests
    ///
    /// # Returns
    /// A `LeaseKeepaliveHandle` that can be used to stop the keepalive task.
    ///
    /// # Example
    /// ```ignore
    /// let result = lease_client.grant(60).await?; // 60 second TTL
    /// let handle = lease_client.start_keepalive(
    ///     result.lease_id,
    ///     Duration::from_secs(20), // keepalive every 20 seconds
    /// );
    ///
    /// // ... do work with the lease ...
    ///
    /// handle.stop(); // Stop keepalive when done
    /// ```
    pub fn start_keepalive(&self, lease_id: u64, interval: Duration) -> LeaseKeepaliveHandle
    where C: 'static {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            run_keepalive_loop(client, lease_id, interval, cancel_clone).await;
        });

        LeaseKeepaliveHandle { cancel, lease_id }
    }
}

/// Run the keepalive loop until cancelled.
async fn run_keepalive_loop<C: CoordinationRpc>(
    client: Arc<C>,
    lease_id: u64,
    interval: Duration,
    cancel: CancellationToken,
) {
    use tokio::time::interval as tokio_interval;

    let mut ticker = tokio_interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    debug!(lease_id, interval_secs = interval.as_secs(), "Lease keepalive started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(lease_id, "Lease keepalive stopped by cancel");
                break;
            }
            _ = ticker.tick() => {
                match send_keepalive(&client, lease_id).await {
                    Ok(ttl) => {
                        debug!(lease_id, ttl_seconds = ttl, "Lease keepalive succeeded");
                    }
                    Err(e) => {
                        warn!(lease_id, error = %e, "Lease keepalive failed");
                        // Continue trying - the lease might still be valid
                    }
                }
            }
        }
    }
}

/// Send a single keepalive request.
async fn send_keepalive<C: CoordinationRpc>(client: &Arc<C>, lease_id: u64) -> Result<u32> {
    use aspen_client_api::LeaseKeepaliveResultResponse;

    let response = client.send_coordination_request(ClientRpcRequest::LeaseKeepalive { lease_id }).await?;

    match response {
        ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
            is_success,
            ttl_seconds,
            error,
            ..
        }) => {
            if is_success {
                Ok(ttl_seconds.unwrap_or(0))
            } else {
                bail!("keepalive failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        _ => bail!("unexpected response type"),
    }
}

/// Handle for controlling a lease keepalive background task.
///
/// When dropped, the keepalive task continues running. Call `stop()` to
/// cancel the keepalive task explicitly.
#[derive(Debug)]
pub struct LeaseKeepaliveHandle {
    cancel: CancellationToken,
    lease_id: u64,
}

impl LeaseKeepaliveHandle {
    /// Stop the keepalive task.
    ///
    /// After calling this, the lease will no longer be automatically refreshed
    /// and will expire after its TTL.
    pub fn stop(self) {
        self.cancel.cancel();
    }

    /// Get the lease ID being kept alive.
    pub fn lease_id(&self) -> u64 {
        self.lease_id
    }

    /// Check if the keepalive is still running.
    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
    }
}

/// Result of a lease grant operation.
#[derive(Debug, Clone)]
pub struct LeaseGrantResult {
    /// The unique lease ID (server-generated or client-provided).
    pub lease_id: u64,
    /// The granted TTL in seconds.
    pub ttl_seconds: u32,
}

/// Result of a lease revoke operation.
#[derive(Debug, Clone)]
pub struct LeaseRevokeResult {
    /// Number of keys deleted with the lease.
    pub keys_deleted: u32,
}

/// Result of a lease keepalive operation.
#[derive(Debug, Clone)]
pub struct LeaseKeepaliveResult {
    /// The lease ID that was refreshed.
    pub lease_id: u64,
    /// The new TTL in seconds after refresh.
    pub ttl_seconds: u32,
}

/// Result of a lease time-to-live query.
#[derive(Debug, Clone)]
pub struct LeaseTimeToLiveResult {
    /// The lease ID queried.
    pub lease_id: u64,
    /// The original TTL when the lease was granted.
    pub granted_ttl_seconds: u32,
    /// The remaining TTL in seconds.
    pub remaining_ttl_seconds: u32,
    /// Keys attached to the lease (if requested).
    pub keys: Vec<String>,
}

/// Local representation of lease info from list operation.
#[derive(Debug, Clone)]
pub struct LeaseInfoLocal {
    /// Unique lease ID.
    pub lease_id: u64,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: u32,
    /// Remaining TTL in seconds.
    pub remaining_ttl_seconds: u32,
}
