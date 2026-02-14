//! Distributed read-write lock client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for distributed read-write lock operations.
///
/// Provides a high-level API for shared read access or exclusive write access.
/// Multiple readers can hold the lock simultaneously, but writers have exclusive access.
///
/// ## Features
///
/// - **Writer-preference fairness**: Prevents writer starvation
/// - **TTL-based crash recovery**: Locks automatically expire
/// - **Fencing tokens**: For split-brain prevention on write locks
/// - **Downgrade support**: Convert write lock to read lock
///
/// ## Usage
///
/// ```ignore
/// use aspen::client::coordination::RWLockClient;
///
/// // Create a RWLock client
/// let rwlock = RWLockClient::new(rpc_client);
///
/// // Acquire read lock
/// let read_result = rwlock.acquire_read("my-lock", "reader-1", Duration::from_secs(30), None).await?;
///
/// // Acquire write lock
/// let write_result = rwlock.acquire_write("my-lock", "writer-1", Duration::from_secs(30), None).await?;
/// println!("Fencing token: {}", write_result.fencing_token);
///
/// // Downgrade write to read
/// let downgrade_result = rwlock.downgrade("my-lock", "writer-1", write_result.fencing_token, Duration::from_secs(30)).await?;
///
/// // Release locks
/// rwlock.release_read("my-lock", "reader-1").await?;
/// rwlock.release_write("my-lock", "writer-1", write_result.fencing_token).await?;
/// ```
pub struct RWLockClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> RWLockClient<C> {
    /// Create a new RWLock client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Acquire a read lock, blocking until available or timeout.
    ///
    /// Multiple readers can hold the lock simultaneously.
    /// Blocked if a writer holds the lock or if writers are waiting (writer-preference).
    pub async fn acquire_read(
        &self,
        name: &str,
        holder_id: &str,
        ttl: Duration,
        timeout: Option<Duration>,
    ) -> Result<RWLockReadResult> {
        use aspen_client_api::RWLockResultResponse;

        let ttl_ms = ttl.as_millis() as u64;
        let timeout_ms = timeout.map(|t| t.as_millis() as u64).unwrap_or(0);

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockAcquireRead {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                ttl_ms,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
                success,
                fencing_token,
                deadline_ms,
                reader_count,
                error,
                ..
            }) => {
                if success {
                    Ok(RWLockReadResult {
                        fencing_token: fencing_token.unwrap_or(0),
                        deadline_ms: deadline_ms.unwrap_or(0),
                        reader_count: reader_count.unwrap_or(1),
                    })
                } else {
                    bail!("read lock acquire failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockAcquireRead"),
        }
    }

    /// Try to acquire a read lock without blocking.
    ///
    /// Returns Some(result) if acquired, None if not available.
    pub async fn try_acquire_read(
        &self,
        name: &str,
        holder_id: &str,
        ttl: Duration,
    ) -> Result<Option<RWLockReadResult>> {
        use aspen_client_api::RWLockResultResponse;

        let ttl_ms = ttl.as_millis() as u64;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockTryAcquireRead {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                ttl_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
                success,
                fencing_token,
                deadline_ms,
                reader_count,
                error,
                ..
            }) => {
                if success {
                    Ok(Some(RWLockReadResult {
                        fencing_token: fencing_token.unwrap_or(0),
                        deadline_ms: deadline_ms.unwrap_or(0),
                        reader_count: reader_count.unwrap_or(1),
                    }))
                } else if error.as_ref().map(|e| e.contains("not available")).unwrap_or(false) {
                    Ok(None)
                } else {
                    bail!("try_acquire_read failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockTryAcquireRead"),
        }
    }

    /// Acquire a write lock, blocking until available or timeout.
    ///
    /// Write locks are exclusive - no other readers or writers can hold the lock.
    /// Returns a fencing token for correctness validation.
    pub async fn acquire_write(
        &self,
        name: &str,
        holder_id: &str,
        ttl: Duration,
        timeout: Option<Duration>,
    ) -> Result<RWLockWriteResult> {
        use aspen_client_api::RWLockResultResponse;

        let ttl_ms = ttl.as_millis() as u64;
        let timeout_ms = timeout.map(|t| t.as_millis() as u64).unwrap_or(0);

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockAcquireWrite {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                ttl_ms,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
                success,
                fencing_token,
                deadline_ms,
                error,
                ..
            }) => {
                if success {
                    Ok(RWLockWriteResult {
                        fencing_token: fencing_token.unwrap_or(0),
                        deadline_ms: deadline_ms.unwrap_or(0),
                    })
                } else {
                    bail!("write lock acquire failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockAcquireWrite"),
        }
    }

    /// Try to acquire a write lock without blocking.
    ///
    /// Returns Some(result) if acquired, None if not available.
    pub async fn try_acquire_write(
        &self,
        name: &str,
        holder_id: &str,
        ttl: Duration,
    ) -> Result<Option<RWLockWriteResult>> {
        use aspen_client_api::RWLockResultResponse;

        let ttl_ms = ttl.as_millis() as u64;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockTryAcquireWrite {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                ttl_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
                success,
                fencing_token,
                deadline_ms,
                error,
                ..
            }) => {
                if success {
                    Ok(Some(RWLockWriteResult {
                        fencing_token: fencing_token.unwrap_or(0),
                        deadline_ms: deadline_ms.unwrap_or(0),
                    }))
                } else if error.as_ref().map(|e| e.contains("not available")).unwrap_or(false) {
                    Ok(None)
                } else {
                    bail!("try_acquire_write failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockTryAcquireWrite"),
        }
    }

    /// Release a read lock.
    pub async fn release_read(&self, name: &str, holder_id: &str) -> Result<()> {
        use aspen_client_api::RWLockResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockReleaseRead {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse { success, error, .. }) => {
                if success {
                    Ok(())
                } else {
                    bail!("release_read failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockReleaseRead"),
        }
    }

    /// Release a write lock.
    ///
    /// Requires the fencing token from acquisition for verification.
    pub async fn release_write(&self, name: &str, holder_id: &str, fencing_token: u64) -> Result<()> {
        use aspen_client_api::RWLockResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockReleaseWrite {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse { success, error, .. }) => {
                if success {
                    Ok(())
                } else {
                    bail!("release_write failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockReleaseWrite"),
        }
    }

    /// Downgrade a write lock to a read lock.
    ///
    /// This is atomic - you maintain a lock throughout the transition.
    /// Returns a read lock result with the same fencing token.
    pub async fn downgrade(
        &self,
        name: &str,
        holder_id: &str,
        fencing_token: u64,
        ttl: Duration,
    ) -> Result<RWLockReadResult> {
        use aspen_client_api::RWLockResultResponse;

        let ttl_ms = ttl.as_millis() as u64;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockDowngrade {
                name: name.to_string(),
                holder_id: holder_id.to_string(),
                fencing_token,
                ttl_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
                success,
                fencing_token: new_token,
                deadline_ms,
                reader_count,
                error,
                ..
            }) => {
                if success {
                    Ok(RWLockReadResult {
                        fencing_token: new_token.unwrap_or(fencing_token),
                        deadline_ms: deadline_ms.unwrap_or(0),
                        reader_count: reader_count.unwrap_or(1),
                    })
                } else {
                    bail!("downgrade failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockDowngrade"),
        }
    }

    /// Get lock status.
    pub async fn status(&self, name: &str) -> Result<RWLockStatusResult> {
        use aspen_client_api::RWLockResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RWLockStatus { name: name.to_string() })
            .await?;

        match response {
            ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                success,
                mode,
                fencing_token,
                reader_count,
                writer_holder,
                error,
                ..
            }) => {
                if success {
                    Ok(RWLockStatusResult {
                        mode: mode.unwrap_or_else(|| "free".to_string()),
                        fencing_token: fencing_token.unwrap_or(0),
                        reader_count: reader_count.unwrap_or(0),
                        writer_holder,
                    })
                } else {
                    bail!("status failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RWLockStatus"),
        }
    }
}

/// Result of acquiring a read lock.
#[derive(Debug, Clone)]
pub struct RWLockReadResult {
    /// Global fencing token (for reference, not required for read release).
    pub fencing_token: u64,
    /// Lock deadline in milliseconds since epoch.
    pub deadline_ms: u64,
    /// Number of active readers.
    pub reader_count: u32,
}

/// Result of acquiring a write lock.
#[derive(Debug, Clone)]
pub struct RWLockWriteResult {
    /// Fencing token (required for release and downgrade).
    pub fencing_token: u64,
    /// Lock deadline in milliseconds since epoch.
    pub deadline_ms: u64,
}

/// Result of querying lock status.
#[derive(Debug, Clone)]
pub struct RWLockStatusResult {
    /// Current lock mode: "free", "read", or "write".
    pub mode: String,
    /// Global fencing token.
    pub fencing_token: u64,
    /// Number of active readers.
    pub reader_count: u32,
    /// Writer holder ID (if mode == "write").
    pub writer_holder: Option<String>,
}
