//! Distributed counting semaphore client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for distributed semaphore operations.
///
/// Provides a high-level API for limiting concurrent access to a shared resource.
/// Each permit holder has a TTL for automatic release on crash recovery.
///
/// ## Usage
///
/// ```ignore
/// use aspen::client::coordination::SemaphoreClient;
///
/// // Create a semaphore client
/// let sems = SemaphoreClient::new(rpc_client);
///
/// // Acquire 2 of 5 permits with 60s TTL
/// let result = sems.acquire("my-semaphore", "holder-1", 2, 5, Duration::from_secs(60), None).await?;
/// println!("Acquired: {}, available: {}", result.permits_acquired, result.available);
///
/// // ... do work ...
///
/// // Release permits
/// let available = sems.release("my-semaphore", "holder-1", 2).await?;
/// println!("Available after release: {}", available);
/// ```
pub struct SemaphoreClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> SemaphoreClient<C> {
    /// Create a new semaphore client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Acquire permits, blocking until available or timeout.
    ///
    /// # Arguments
    /// * `name` - Semaphore name (unique identifier)
    /// * `holder_id` - Unique identifier for this holder
    /// * `permits` - Number of permits to acquire
    /// * `capacity` - Maximum permits (creates semaphore if not exists)
    /// * `ttl` - Time-to-live for automatic release
    /// * `timeout` - Optional timeout for waiting
    ///
    /// # Returns
    /// A `SemaphoreAcquireResult` on success.
    pub async fn acquire(
        &self,
        name: &str,
        holder_id: &str,
        permits: u32,
        capacity: u32,
        ttl: Duration,
        timeout: Option<Duration>,
    ) -> Result<SemaphoreAcquireResult> {
        use aspen_client_api::SemaphoreResultResponse;

        let ttl_ms = ttl.as_millis() as u64;
        let timeout_ms = timeout.map(|t| t.as_millis() as u64).unwrap_or(0);

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SemaphoreAcquire {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                permits,
                capacity,
                ttl_ms,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
                success,
                permits_acquired,
                available,
                error,
                ..
            }) => {
                if success {
                    Ok(SemaphoreAcquireResult {
                        permits_acquired: permits_acquired.unwrap_or(permits),
                        available: available.unwrap_or(0),
                    })
                } else {
                    bail!("semaphore acquire failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for SemaphoreAcquire"),
        }
    }

    /// Try to acquire permits without blocking.
    ///
    /// # Arguments
    /// * `name` - Semaphore name
    /// * `holder_id` - Unique identifier for this holder
    /// * `permits` - Number of permits to acquire
    /// * `capacity` - Maximum permits (creates semaphore if not exists)
    /// * `ttl` - Time-to-live for automatic release
    ///
    /// # Returns
    /// Some(result) if permits acquired, None if not available.
    pub async fn try_acquire(
        &self,
        name: &str,
        holder_id: &str,
        permits: u32,
        capacity: u32,
        ttl: Duration,
    ) -> Result<Option<SemaphoreAcquireResult>> {
        use aspen_client_api::SemaphoreResultResponse;

        let ttl_ms = ttl.as_millis() as u64;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SemaphoreTryAcquire {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                permits,
                capacity,
                ttl_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::SemaphoreTryAcquireResult(SemaphoreResultResponse {
                success,
                permits_acquired,
                available,
                error,
                ..
            }) => {
                if success {
                    Ok(Some(SemaphoreAcquireResult {
                        permits_acquired: permits_acquired.unwrap_or(permits),
                        available: available.unwrap_or(0),
                    }))
                } else if error.is_some() {
                    bail!("semaphore try_acquire failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                } else {
                    // Not enough permits available
                    Ok(None)
                }
            }
            _ => bail!("unexpected response type for SemaphoreTryAcquire"),
        }
    }

    /// Release permits back to the semaphore.
    ///
    /// # Arguments
    /// * `name` - Semaphore name
    /// * `holder_id` - Unique identifier for this holder
    /// * `permits` - Number of permits to release (0 = release all)
    ///
    /// # Returns
    /// The number of available permits after release.
    pub async fn release(&self, name: &str, holder_id: &str, permits: u32) -> Result<u32> {
        use aspen_client_api::SemaphoreResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SemaphoreRelease {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                permits,
            })
            .await?;

        match response {
            ClientRpcResponse::SemaphoreReleaseResult(SemaphoreResultResponse {
                success,
                available,
                error,
                ..
            }) => {
                if success {
                    Ok(available.unwrap_or(0))
                } else {
                    bail!("semaphore release failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for SemaphoreRelease"),
        }
    }

    /// Get semaphore status.
    ///
    /// # Arguments
    /// * `name` - Semaphore name
    ///
    /// # Returns
    /// A `SemaphoreStatusResult` with current state.
    pub async fn status(&self, name: &str) -> Result<SemaphoreStatusResult> {
        use aspen_client_api::SemaphoreResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SemaphoreStatus { name: name.to_string() })
            .await?;

        match response {
            ClientRpcResponse::SemaphoreStatusResult(SemaphoreResultResponse {
                success,
                available,
                capacity,
                error,
                ..
            }) => {
                if success {
                    Ok(SemaphoreStatusResult {
                        available: available.unwrap_or(0),
                        capacity: capacity.unwrap_or(0),
                    })
                } else {
                    bail!("semaphore status failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for SemaphoreStatus"),
        }
    }
}

/// Result of acquiring semaphore permits.
#[derive(Debug, Clone)]
pub struct SemaphoreAcquireResult {
    /// Number of permits acquired.
    pub permits_acquired: u32,
    /// Permits available after acquisition.
    pub available: u32,
}

/// Result of querying semaphore status.
#[derive(Debug, Clone)]
pub struct SemaphoreStatusResult {
    /// Available permits.
    pub available: u32,
    /// Total capacity.
    pub capacity: u32,
}
