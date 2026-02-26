//! Distributed lock client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

/// Client for distributed lock operations.
///
/// Provides a high-level API for acquiring, releasing, and renewing
/// distributed locks over the client RPC protocol.
pub struct LockClient<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
}

impl<C: CoordinationRpc> LockClient<C> {
    /// Create a new lock client for a specific key.
    pub fn new(client: Arc<C>, key: impl Into<String>) -> Self {
        Self {
            client,
            key: key.into(),
        }
    }

    /// Acquire the lock with blocking wait.
    ///
    /// # Arguments
    /// * `holder_id` - Unique identifier for this lock holder
    /// * `ttl` - How long the lock should be held before automatic expiry
    /// * `timeout` - How long to wait for the lock
    ///
    /// # Returns
    /// A lock guard with fencing token, or error if acquisition failed.
    pub async fn acquire(
        &self,
        holder_id: impl Into<String>,
        ttl: Duration,
        timeout: Duration,
    ) -> Result<RemoteLockGuard<C>> {
        let holder_id = holder_id.into();
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockAcquire {
                key: self.key.clone(),
                holder_id: holder_id.clone(),
                ttl_ms: ttl.as_millis() as u64,
                timeout_ms: timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::LockResult(result) => {
                if result.is_success {
                    Ok(RemoteLockGuard {
                        client: self.client.clone(),
                        key: self.key.clone(),
                        holder_id,
                        fencing_token: result.fencing_token.unwrap_or(0),
                        deadline_ms: result.deadline_ms.unwrap_or(0),
                    })
                } else {
                    bail!("lock acquisition failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LockAcquire"),
        }
    }

    /// Try to acquire the lock without blocking.
    ///
    /// # Arguments
    /// * `holder_id` - Unique identifier for this lock holder
    /// * `ttl` - How long the lock should be held before automatic expiry
    ///
    /// # Returns
    /// Some(guard) if lock was acquired, None if already held.
    pub async fn try_acquire(&self, holder_id: impl Into<String>, ttl: Duration) -> Result<Option<RemoteLockGuard<C>>> {
        let holder_id = holder_id.into();
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockTryAcquire {
                key: self.key.clone(),
                holder_id: holder_id.clone(),
                ttl_ms: ttl.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::LockResult(result) => {
                if result.is_success {
                    Ok(Some(RemoteLockGuard {
                        client: self.client.clone(),
                        key: self.key.clone(),
                        holder_id,
                        fencing_token: result.fencing_token.unwrap_or(0),
                        deadline_ms: result.deadline_ms.unwrap_or(0),
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => bail!("unexpected response type for LockTryAcquire"),
        }
    }
}

/// Guard for a remotely-held distributed lock.
///
/// Contains the fencing token for use in protected operations.
/// The lock can be released explicitly with `release()`.
pub struct RemoteLockGuard<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
    holder_id: String,
    fencing_token: u64,
    deadline_ms: u64,
}

impl<C: CoordinationRpc> RemoteLockGuard<C> {
    /// Get the fencing token for this lock.
    ///
    /// Include this in operations protected by the lock to detect
    /// stale lock holders.
    pub fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    /// Get the lock expiration deadline in Unix milliseconds.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Get the lock key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Release the lock explicitly.
    pub async fn release(self) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockRelease {
                key: self.key.clone(),
                holder_id: self.holder_id.clone(),
                fencing_token: self.fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::LockResult(result) => {
                if result.is_success {
                    Ok(())
                } else {
                    bail!("lock release failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LockRelease"),
        }
    }

    /// Renew the lock TTL.
    pub async fn renew(&mut self, ttl: Duration) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockRenew {
                key: self.key.clone(),
                holder_id: self.holder_id.clone(),
                fencing_token: self.fencing_token,
                ttl_ms: ttl.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::LockResult(result) => {
                if result.is_success {
                    if let Some(deadline) = result.deadline_ms {
                        self.deadline_ms = deadline;
                    }
                    Ok(())
                } else {
                    bail!("lock renewal failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for LockRenew"),
        }
    }
}
