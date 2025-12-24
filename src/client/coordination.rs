//! Typed client wrappers for coordination primitives.
//!
//! Provides ergonomic APIs for distributed locks, counters, sequences,
//! and rate limiters over the client RPC protocol.
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::client::coordination::{LockClient, CounterClient, SequenceClient, RateLimiterClient};
//!
//! // Get a distributed lock
//! let lock = LockClient::new(rpc_client.clone(), "my-resource");
//! let guard = lock.acquire("holder-1", Duration::from_secs(30)).await?;
//! // ... protected work ...
//! guard.release().await?;
//!
//! // Use an atomic counter
//! let counter = CounterClient::new(rpc_client.clone(), "request-count");
//! let value = counter.increment().await?;
//!
//! // Generate unique IDs
//! let seq = SequenceClient::new(rpc_client.clone(), "order-ids");
//! let order_id = seq.next().await?;
//!
//! // Rate limit operations
//! let limiter = RateLimiterClient::new(rpc_client.clone(), "api-calls", 100.0, 50);
//! if limiter.try_acquire().await? {
//!     // proceed with rate-limited operation
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Trait for RPC clients that can send coordination requests.
///
/// Implemented by both single-node and multi-node clients.
pub trait CoordinationRpc: Send + Sync {
    /// Send a coordination RPC request.
    fn send_coordination_request(
        &self,
        request: ClientRpcRequest,
    ) -> impl std::future::Future<Output = Result<ClientRpcResponse>> + Send;
}

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
                if result.success {
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
                if result.success {
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
                if result.success {
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
                if result.success {
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

/// Client for atomic counter operations.
pub struct CounterClient<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
}

impl<C: CoordinationRpc> CounterClient<C> {
    /// Create a new counter client for a specific key.
    pub fn new(client: Arc<C>, key: impl Into<String>) -> Self {
        Self {
            client,
            key: key.into(),
        }
    }

    /// Get the current counter value.
    pub async fn get(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterGet { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    /// Increment the counter by 1 and return the new value.
    pub async fn increment(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterIncrement { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    /// Decrement the counter by 1 and return the new value.
    pub async fn decrement(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterDecrement { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    /// Add amount to counter and return the new value.
    pub async fn add(&self, amount: u64) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterAdd {
                key: self.key.clone(),
                amount,
            })
            .await?;

        Self::extract_value(response)
    }

    /// Subtract amount from counter and return the new value.
    pub async fn subtract(&self, amount: u64) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterSubtract {
                key: self.key.clone(),
                amount,
            })
            .await?;

        Self::extract_value(response)
    }

    /// Set the counter to a specific value.
    pub async fn set(&self, value: u64) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterSet {
                key: self.key.clone(),
                value,
            })
            .await?;

        Self::extract_value(response)
    }

    /// Compare and set: set to new_value only if current value equals expected.
    ///
    /// # Returns
    /// The new value if CAS succeeded, or error if expected didn't match.
    pub async fn compare_and_set(&self, expected: u64, new_value: u64) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterCompareAndSet {
                key: self.key.clone(),
                expected,
                new_value,
            })
            .await?;

        Self::extract_value(response)
    }

    fn extract_value(response: ClientRpcResponse) -> Result<u64> {
        match response {
            ClientRpcResponse::CounterResult(result) => {
                if result.success {
                    Ok(result.value.unwrap_or(0))
                } else {
                    bail!("counter operation failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for counter operation"),
        }
    }
}

/// Client for signed counter operations.
pub struct SignedCounterClient<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
}

impl<C: CoordinationRpc> SignedCounterClient<C> {
    /// Create a new signed counter client for a specific key.
    pub fn new(client: Arc<C>, key: impl Into<String>) -> Self {
        Self {
            client,
            key: key.into(),
        }
    }

    /// Get the current counter value.
    pub async fn get(&self) -> Result<i64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SignedCounterGet { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    /// Add amount to counter (can be negative) and return the new value.
    pub async fn add(&self, amount: i64) -> Result<i64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SignedCounterAdd {
                key: self.key.clone(),
                amount,
            })
            .await?;

        Self::extract_value(response)
    }

    fn extract_value(response: ClientRpcResponse) -> Result<i64> {
        match response {
            ClientRpcResponse::SignedCounterResult(result) => {
                if result.success {
                    Ok(result.value.unwrap_or(0))
                } else {
                    bail!(
                        "signed counter operation failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
                }
            }
            _ => bail!("unexpected response type for signed counter operation"),
        }
    }
}

/// Client for sequence generator operations.
pub struct SequenceClient<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
}

impl<C: CoordinationRpc> SequenceClient<C> {
    /// Create a new sequence client for a specific key.
    pub fn new(client: Arc<C>, key: impl Into<String>) -> Self {
        Self {
            client,
            key: key.into(),
        }
    }

    /// Get the next sequence value.
    pub async fn next(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SequenceNext { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    /// Reserve a batch of sequence values.
    ///
    /// # Returns
    /// The start of the reserved range. The caller owns [start, start+count).
    pub async fn reserve(&self, count: u64) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SequenceReserve {
                key: self.key.clone(),
                count,
            })
            .await?;

        Self::extract_value(response)
    }

    /// Get the current sequence value without incrementing.
    pub async fn current(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::SequenceCurrent { key: self.key.clone() })
            .await?;

        Self::extract_value(response)
    }

    fn extract_value(response: ClientRpcResponse) -> Result<u64> {
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                if result.success {
                    Ok(result.value.unwrap_or(0))
                } else {
                    bail!("sequence operation failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for sequence operation"),
        }
    }
}

/// Client for rate limiter operations.
pub struct RateLimiterClient<C: CoordinationRpc> {
    client: Arc<C>,
    key: String,
    capacity: u64,
    refill_rate: f64,
}

impl<C: CoordinationRpc> RateLimiterClient<C> {
    /// Create a new rate limiter client.
    ///
    /// # Arguments
    /// * `client` - RPC client for communication
    /// * `key` - Unique key for this rate limiter
    /// * `refill_rate` - Tokens per second to refill
    /// * `capacity` - Maximum token capacity (burst size)
    pub fn new(client: Arc<C>, key: impl Into<String>, refill_rate: f64, capacity: u64) -> Self {
        Self {
            client,
            key: key.into(),
            capacity,
            refill_rate,
        }
    }

    /// Try to acquire one token without blocking.
    ///
    /// # Returns
    /// Ok(remaining) if token was acquired, Err with retry_after_ms if rate limited.
    pub async fn try_acquire(&self) -> Result<RateLimitResult> {
        self.try_acquire_n(1).await
    }

    /// Try to acquire multiple tokens without blocking.
    pub async fn try_acquire_n(&self, tokens: u64) -> Result<RateLimitResult> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RateLimiterTryAcquire {
                key: self.key.clone(),
                tokens,
                capacity: self.capacity,
                refill_rate: self.refill_rate,
            })
            .await?;

        Self::extract_result(response)
    }

    /// Acquire tokens with blocking wait.
    ///
    /// # Arguments
    /// * `tokens` - Number of tokens to acquire
    /// * `timeout` - Maximum time to wait for tokens
    pub async fn acquire(&self, tokens: u64, timeout: Duration) -> Result<RateLimitResult> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RateLimiterAcquire {
                key: self.key.clone(),
                tokens,
                capacity: self.capacity,
                refill_rate: self.refill_rate,
                timeout_ms: timeout.as_millis() as u64,
            })
            .await?;

        Self::extract_result(response)
    }

    /// Get the number of tokens currently available.
    pub async fn available(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RateLimiterAvailable {
                key: self.key.clone(),
                capacity: self.capacity,
                refill_rate: self.refill_rate,
            })
            .await?;

        match response {
            ClientRpcResponse::RateLimiterResult(result) => Ok(result.tokens_remaining.unwrap_or(0)),
            _ => bail!("unexpected response type for RateLimiterAvailable"),
        }
    }

    /// Reset the rate limiter to full capacity.
    pub async fn reset(&self) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::RateLimiterReset {
                key: self.key.clone(),
                capacity: self.capacity,
                refill_rate: self.refill_rate,
            })
            .await?;

        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    bail!("rate limiter reset failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for RateLimiterReset"),
        }
    }

    fn extract_result(response: ClientRpcResponse) -> Result<RateLimitResult> {
        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                if result.success {
                    Ok(RateLimitResult::Acquired {
                        tokens_remaining: result.tokens_remaining.unwrap_or(0),
                    })
                } else {
                    Ok(RateLimitResult::RateLimited {
                        retry_after_ms: result.retry_after_ms.unwrap_or(0),
                    })
                }
            }
            _ => bail!("unexpected response type for rate limiter operation"),
        }
    }
}

/// Result of a rate limit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Tokens were acquired successfully.
    Acquired {
        /// Remaining tokens after acquisition.
        tokens_remaining: u64,
    },
    /// Rate limit exceeded, try again later.
    RateLimited {
        /// Milliseconds to wait before retrying.
        retry_after_ms: u64,
    },
}

impl RateLimitResult {
    /// Check if tokens were acquired.
    pub fn is_acquired(&self) -> bool {
        matches!(self, RateLimitResult::Acquired { .. })
    }

    /// Check if rate limited.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, RateLimitResult::RateLimited { .. })
    }
}

// ============================================================================
// Batch Operations Client
// ============================================================================

/// Client for batch read/write operations.
///
/// Provides atomic multi-key operations for efficient bulk access.
pub struct BatchClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> BatchClient<C> {
    /// Create a new batch client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Read multiple keys atomically.
    ///
    /// Returns values for all keys in the same order as requested.
    /// Non-existent keys return None in their position.
    pub async fn read(&self, keys: Vec<String>) -> Result<Vec<Option<Vec<u8>>>> {
        use crate::client_rpc::BatchReadResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::BatchRead { keys }).await?;

        match response {
            ClientRpcResponse::BatchReadResult(BatchReadResultResponse { success, values, error }) => {
                if success {
                    Ok(values.unwrap_or_default())
                } else {
                    bail!("batch read failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BatchRead"),
        }
    }

    /// Write multiple key-value operations atomically.
    ///
    /// All operations in the batch are applied atomically - either all succeed
    /// or none are applied.
    pub async fn write(&self, operations: Vec<BatchWriteOp>) -> Result<u32> {
        use crate::client_rpc::BatchWriteOperation;
        use crate::client_rpc::BatchWriteResultResponse;

        let ops: Vec<BatchWriteOperation> = operations
            .into_iter()
            .map(|op| match op {
                BatchWriteOp::Set { key, value } => BatchWriteOperation::Set { key, value },
                BatchWriteOp::Delete { key } => BatchWriteOperation::Delete { key },
            })
            .collect();

        let response = self.client.send_coordination_request(ClientRpcRequest::BatchWrite { operations: ops }).await?;

        match response {
            ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                success,
                operations_applied,
                error,
            }) => {
                if success {
                    Ok(operations_applied.unwrap_or(0))
                } else {
                    bail!("batch write failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BatchWrite"),
        }
    }

    /// Write with conditions (etcd-style transaction).
    ///
    /// Checks all conditions first. If all pass, applies all operations atomically.
    /// If any condition fails, returns which condition failed without modifying data.
    pub async fn conditional_write(
        &self,
        conditions: Vec<BatchConditionOp>,
        operations: Vec<BatchWriteOp>,
    ) -> Result<ConditionalBatchResult> {
        use crate::client_rpc::BatchCondition;
        use crate::client_rpc::BatchWriteOperation;
        use crate::client_rpc::ConditionalBatchWriteResultResponse;

        let conds: Vec<BatchCondition> = conditions
            .into_iter()
            .map(|c| match c {
                BatchConditionOp::ValueEquals { key, expected } => BatchCondition::ValueEquals { key, expected },
                BatchConditionOp::KeyExists { key } => BatchCondition::KeyExists { key },
                BatchConditionOp::KeyNotExists { key } => BatchCondition::KeyNotExists { key },
            })
            .collect();

        let ops: Vec<BatchWriteOperation> = operations
            .into_iter()
            .map(|op| match op {
                BatchWriteOp::Set { key, value } => BatchWriteOperation::Set { key, value },
                BatchWriteOp::Delete { key } => BatchWriteOperation::Delete { key },
            })
            .collect();

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ConditionalBatchWrite {
                conditions: conds,
                operations: ops,
            })
            .await?;

        match response {
            ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
                success: _,
                conditions_met,
                operations_applied,
                failed_condition_index,
                failed_condition_reason,
                error,
            }) => {
                if let Some(err) = error {
                    bail!("conditional batch failed: {}", err);
                }

                if conditions_met {
                    Ok(ConditionalBatchResult::Applied {
                        operations_count: operations_applied.unwrap_or(0),
                    })
                } else {
                    Ok(ConditionalBatchResult::ConditionFailed {
                        index: failed_condition_index.unwrap_or(0),
                        reason: failed_condition_reason,
                    })
                }
            }
            _ => bail!("unexpected response type for ConditionalBatchWrite"),
        }
    }
}

/// A batch write operation.
#[derive(Debug, Clone)]
pub enum BatchWriteOp {
    /// Set a key to a value.
    Set {
        /// Key to set.
        key: String,
        /// Value to set.
        value: Vec<u8>,
    },
    /// Delete a key.
    Delete {
        /// Key to delete.
        key: String,
    },
}

impl BatchWriteOp {
    /// Create a Set operation.
    pub fn set(key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        BatchWriteOp::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Create a Delete operation.
    pub fn delete(key: impl Into<String>) -> Self {
        BatchWriteOp::Delete { key: key.into() }
    }
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone)]
pub enum BatchConditionOp {
    /// Key must have this exact value.
    ValueEquals {
        /// Key to check.
        key: String,
        /// Expected value.
        expected: Vec<u8>,
    },
    /// Key must exist.
    KeyExists {
        /// Key to check.
        key: String,
    },
    /// Key must not exist.
    KeyNotExists {
        /// Key to check.
        key: String,
    },
}

impl BatchConditionOp {
    /// Create a ValueEquals condition.
    pub fn value_equals(key: impl Into<String>, expected: impl Into<Vec<u8>>) -> Self {
        BatchConditionOp::ValueEquals {
            key: key.into(),
            expected: expected.into(),
        }
    }

    /// Create a KeyExists condition.
    pub fn key_exists(key: impl Into<String>) -> Self {
        BatchConditionOp::KeyExists { key: key.into() }
    }

    /// Create a KeyNotExists condition.
    pub fn key_not_exists(key: impl Into<String>) -> Self {
        BatchConditionOp::KeyNotExists { key: key.into() }
    }
}

/// Result of a conditional batch operation.
#[derive(Debug, Clone)]
pub enum ConditionalBatchResult {
    /// All conditions passed, operations were applied.
    Applied {
        /// Number of operations that were applied.
        operations_count: u32,
    },
    /// A condition failed, no operations were applied.
    ConditionFailed {
        /// Index of the first failed condition.
        index: u32,
        /// Optional reason for failure.
        reason: Option<String>,
    },
}

impl ConditionalBatchResult {
    /// Check if the batch was applied.
    pub fn is_applied(&self) -> bool {
        matches!(self, ConditionalBatchResult::Applied { .. })
    }

    /// Check if a condition failed.
    pub fn is_condition_failed(&self) -> bool {
        matches!(self, ConditionalBatchResult::ConditionFailed { .. })
    }
}

// =============================================================================
// Watch Client - Real-time key change notifications
// =============================================================================

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
        use crate::client_rpc::WatchCreateResultResponse;

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
        use crate::client_rpc::WatchCancelResultResponse;

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
        use crate::client_rpc::WatchStatusResultResponse;

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

// =============================================================================
// Lease Client
// =============================================================================

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
        use crate::client_rpc::LeaseGrantResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LeaseGrant { ttl_seconds, lease_id })
            .await?;

        match response {
            ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                success,
                lease_id,
                ttl_seconds,
                error,
            }) => {
                if success {
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
        use crate::client_rpc::LeaseRevokeResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseRevoke { lease_id }).await?;

        match response {
            ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                success,
                keys_deleted,
                error,
            }) => {
                if success {
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
        use crate::client_rpc::LeaseKeepaliveResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseKeepalive { lease_id }).await?;

        match response {
            ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                success,
                lease_id: returned_id,
                ttl_seconds,
                error,
            }) => {
                if success {
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
        use crate::client_rpc::LeaseTimeToLiveResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LeaseTimeToLive { lease_id, include_keys })
            .await?;

        match response {
            ClientRpcResponse::LeaseTimeToLiveResult(LeaseTimeToLiveResultResponse {
                success,
                lease_id: returned_id,
                granted_ttl_seconds,
                remaining_ttl_seconds,
                keys,
                error,
            }) => {
                if success {
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
        use crate::client_rpc::LeaseListResultResponse;

        let response = self.client.send_coordination_request(ClientRpcRequest::LeaseList).await?;

        match response {
            ClientRpcResponse::LeaseListResult(LeaseListResultResponse { success, leases, error }) => {
                if success {
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
        use crate::client_rpc::WriteResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::WriteKeyWithLease {
                key: key.to_string(),
                value: value.to_vec(),
                lease_id,
            })
            .await?;

        match response {
            ClientRpcResponse::WriteResult(WriteResultResponse { success, error }) => {
                if success {
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
    where
        C: 'static,
    {
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
    use crate::client_rpc::LeaseKeepaliveResultResponse;

    let response = client.send_coordination_request(ClientRpcRequest::LeaseKeepalive { lease_id }).await?;

    match response {
        ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
            success,
            ttl_seconds,
            error,
            ..
        }) => {
            if success {
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

// =============================================================================
// Barrier Client - Distributed synchronization barrier
// =============================================================================

/// Client for distributed barrier operations.
///
/// Provides a high-level API for coordinating multiple participants at
/// synchronization points. Implements a "double barrier" pattern:
///
/// 1. **Enter phase**: Participants register and wait until all arrive
/// 2. **Work phase**: All participants proceed with their work
/// 3. **Leave phase**: Participants deregister and wait until all leave
///
/// ## Usage
///
/// ```ignore
/// use aspen::client::coordination::BarrierClient;
///
/// // Create a barrier client
/// let barriers = BarrierClient::new(rpc_client);
///
/// // Enter barrier with 3 participants
/// let (count, phase) = barriers.enter("my-barrier", "participant-1", 3, None).await?;
/// println!("Arrived: {}/{}", count, 3);
///
/// // ... do synchronized work ...
///
/// // Leave barrier
/// let (remaining, phase) = barriers.leave("my-barrier", "participant-1", None).await?;
/// println!("Left: {} remaining", remaining);
/// ```
pub struct BarrierClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> BarrierClient<C> {
    /// Create a new barrier client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Enter a barrier, waiting until all participants arrive.
    ///
    /// # Arguments
    /// * `name` - Barrier name (unique identifier)
    /// * `participant_id` - Unique identifier for this participant
    /// * `required_count` - Number of participants required before proceeding
    /// * `timeout` - Optional timeout for waiting
    ///
    /// # Returns
    /// A tuple of (current_count, phase) on success.
    pub async fn enter(
        &self,
        name: &str,
        participant_id: &str,
        required_count: u32,
        timeout: Option<Duration>,
    ) -> Result<BarrierEnterResult> {
        use crate::client_rpc::BarrierResultResponse;

        let timeout_ms = timeout.map(|t| t.as_millis() as u64).unwrap_or(0);

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::BarrierEnter {
                name: name.to_string(),
                participant_id: participant_id.to_string(),
                required_count,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
                success,
                current_count,
                required_count: returned_required,
                phase,
                error,
            }) => {
                if success {
                    Ok(BarrierEnterResult {
                        current_count: current_count.unwrap_or(0),
                        required_count: returned_required.unwrap_or(required_count),
                        phase: phase.unwrap_or_else(|| "unknown".to_string()),
                    })
                } else {
                    bail!("barrier enter failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BarrierEnter"),
        }
    }

    /// Leave a barrier, waiting until all participants leave.
    ///
    /// # Arguments
    /// * `name` - Barrier name
    /// * `participant_id` - Unique identifier for this participant
    /// * `timeout` - Optional timeout for waiting
    ///
    /// # Returns
    /// A tuple of (remaining_count, phase) on success.
    pub async fn leave(
        &self,
        name: &str,
        participant_id: &str,
        timeout: Option<Duration>,
    ) -> Result<BarrierLeaveResult> {
        use crate::client_rpc::BarrierResultResponse;

        let timeout_ms = timeout.map(|t| t.as_millis() as u64).unwrap_or(0);

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::BarrierLeave {
                name: name.to_string(),
                participant_id: participant_id.to_string(),
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
                success,
                current_count,
                phase,
                error,
                ..
            }) => {
                if success {
                    Ok(BarrierLeaveResult {
                        remaining_count: current_count.unwrap_or(0),
                        phase: phase.unwrap_or_else(|| "unknown".to_string()),
                    })
                } else {
                    bail!("barrier leave failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BarrierLeave"),
        }
    }

    /// Get barrier status without modifying it.
    ///
    /// # Arguments
    /// * `name` - Barrier name
    ///
    /// # Returns
    /// A `BarrierStatusResult` with current state.
    pub async fn status(&self, name: &str) -> Result<BarrierStatusResult> {
        use crate::client_rpc::BarrierResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::BarrierStatus { name: name.to_string() })
            .await?;

        match response {
            ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
                success,
                current_count,
                required_count,
                phase,
                error,
            }) => {
                if success {
                    Ok(BarrierStatusResult {
                        current_count: current_count.unwrap_or(0),
                        required_count: required_count.unwrap_or(0),
                        phase: phase.unwrap_or_else(|| "none".to_string()),
                    })
                } else {
                    bail!("barrier status failed: {}", error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for BarrierStatus"),
        }
    }
}

/// Result of entering a barrier.
#[derive(Debug, Clone)]
pub struct BarrierEnterResult {
    /// Current number of participants.
    pub current_count: u32,
    /// Required number of participants.
    pub required_count: u32,
    /// Current barrier phase.
    pub phase: String,
}

/// Result of leaving a barrier.
#[derive(Debug, Clone)]
pub struct BarrierLeaveResult {
    /// Remaining participants.
    pub remaining_count: u32,
    /// Current barrier phase.
    pub phase: String,
}

/// Result of querying barrier status.
#[derive(Debug, Clone)]
pub struct BarrierStatusResult {
    /// Current number of participants.
    pub current_count: u32,
    /// Required number of participants.
    pub required_count: u32,
    /// Current barrier phase.
    pub phase: String,
}

// =============================================================================
// Semaphore Client - Distributed counting semaphore
// =============================================================================

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
        use crate::client_rpc::SemaphoreResultResponse;

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
        use crate::client_rpc::SemaphoreResultResponse;

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
        use crate::client_rpc::SemaphoreResultResponse;

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
        use crate::client_rpc::SemaphoreResultResponse;

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

// =============================================================================
// RWLock Client - Distributed read-write lock
// =============================================================================

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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
        use crate::client_rpc::RWLockResultResponse;

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

// ============================================================================
// Queue Client
// ============================================================================

/// Client for distributed queue operations.
///
/// Provides a high-level async API for queue operations including enqueue,
/// dequeue with visibility timeout, acknowledgment, and dead letter queue.
///
/// # Example
///
/// ```ignore
/// use aspen::client::coordination::QueueClient;
///
/// let queue = QueueClient::new(rpc_client, "my-queue");
///
/// // Enqueue an item
/// let item_id = queue.enqueue(b"hello".to_vec()).await?;
///
/// // Dequeue with visibility timeout
/// let items = queue.dequeue("consumer-1", 10, Duration::from_secs(30)).await?;
///
/// // Process and acknowledge
/// for item in items {
///     // Process item...
///     queue.ack(&item.receipt_handle).await?;
/// }
/// ```
pub struct QueueClient<C: CoordinationRpc> {
    client: Arc<C>,
    queue_name: String,
}

impl<C: CoordinationRpc> QueueClient<C> {
    /// Create a new queue client.
    pub fn new(client: Arc<C>, queue_name: impl Into<String>) -> Self {
        Self {
            client,
            queue_name: queue_name.into(),
        }
    }

    /// Create or configure a queue.
    ///
    /// Returns (created, success) where created is true if the queue was newly created.
    pub async fn create(&self, config: QueueCreateConfig) -> Result<bool> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueCreate {
                queue_name: self.queue_name.clone(),
                default_visibility_timeout_ms: config.default_visibility_timeout.map(|d| d.as_millis() as u64),
                default_ttl_ms: config.default_ttl.map(|d| d.as_millis() as u64),
                max_delivery_attempts: config.max_delivery_attempts,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueCreateResult(result) => {
                if result.success {
                    Ok(result.created)
                } else {
                    bail!("queue create failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueCreate"),
        }
    }

    /// Delete the queue and all its items.
    ///
    /// Returns the number of items deleted.
    pub async fn delete(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDelete {
                queue_name: self.queue_name.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDeleteResult(result) => {
                if result.success {
                    Ok(result.items_deleted.unwrap_or(0))
                } else {
                    bail!("queue delete failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDelete"),
        }
    }

    /// Enqueue an item.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue(&self, payload: Vec<u8>) -> Result<u64> {
        self.enqueue_with_options(payload, QueueEnqueueOptions::default()).await
    }

    /// Enqueue an item with options.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue_with_options(&self, payload: Vec<u8>, options: QueueEnqueueOptions) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueEnqueue {
                queue_name: self.queue_name.clone(),
                payload,
                ttl_ms: options.ttl.map(|d| d.as_millis() as u64),
                message_group_id: options.message_group_id,
                deduplication_id: options.deduplication_id,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueEnqueueResult(result) => {
                if result.success {
                    Ok(result.item_id.unwrap_or(0))
                } else {
                    bail!("enqueue failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueEnqueue"),
        }
    }

    /// Enqueue multiple items in a batch.
    ///
    /// Returns the list of item IDs on success.
    pub async fn enqueue_batch(&self, items: Vec<QueueEnqueueBatchItem>) -> Result<Vec<u64>> {
        use crate::client_rpc::QueueEnqueueItem;

        let rpc_items: Vec<QueueEnqueueItem> = items
            .into_iter()
            .map(|item| QueueEnqueueItem {
                payload: item.payload,
                ttl_ms: item.ttl.map(|d| d.as_millis() as u64),
                message_group_id: item.message_group_id,
                deduplication_id: item.deduplication_id,
            })
            .collect();

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueEnqueueBatch {
                queue_name: self.queue_name.clone(),
                items: rpc_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueEnqueueBatchResult(result) => {
                if result.success {
                    Ok(result.item_ids)
                } else {
                    bail!("enqueue batch failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueEnqueueBatch"),
        }
    }

    /// Dequeue items with visibility timeout (non-blocking).
    ///
    /// Returns up to `max_items` items. Each item is locked for `visibility_timeout`.
    /// Returns empty vec if queue is empty.
    pub async fn dequeue(
        &self,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout: Duration,
    ) -> Result<Vec<QueueDequeuedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDequeue {
                queue_name: self.queue_name.clone(),
                consumer_id: consumer_id.to_string(),
                max_items,
                visibility_timeout_ms: visibility_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDequeueResult(result) => {
                if result.success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDequeuedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            visibility_deadline: Duration::from_millis(item.visibility_deadline_ms),
                        })
                        .collect())
                } else {
                    bail!("dequeue failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDequeue"),
        }
    }

    /// Dequeue items with blocking wait.
    ///
    /// Polls until items are available or `wait_timeout` expires.
    pub async fn dequeue_wait(
        &self,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout: Duration,
        wait_timeout: Duration,
    ) -> Result<Vec<QueueDequeuedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueDequeueWait {
                queue_name: self.queue_name.clone(),
                consumer_id: consumer_id.to_string(),
                max_items,
                visibility_timeout_ms: visibility_timeout.as_millis() as u64,
                wait_timeout_ms: wait_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueDequeueResult(result) => {
                if result.success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDequeuedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            receipt_handle: item.receipt_handle,
                            delivery_attempts: item.delivery_attempts,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            visibility_deadline: Duration::from_millis(item.visibility_deadline_ms),
                        })
                        .collect())
                } else {
                    bail!("dequeue_wait failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueDequeueWait"),
        }
    }

    /// Peek at items without removing them.
    ///
    /// Returns up to `max_items` items from the front of the queue.
    pub async fn peek(&self, max_items: u32) -> Result<Vec<QueuePeekedItem>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueuePeek {
                queue_name: self.queue_name.clone(),
                max_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueuePeekResult(result) => {
                if result.success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueuePeekedItem {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            expires_at: if item.expires_at_ms > 0 {
                                Some(Duration::from_millis(item.expires_at_ms))
                            } else {
                                None
                            },
                            delivery_attempts: item.delivery_attempts,
                        })
                        .collect())
                } else {
                    bail!("peek failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueuePeek"),
        }
    }

    /// Acknowledge successful processing of an item.
    pub async fn ack(&self, receipt_handle: &str) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueAck {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueAckResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    bail!("ack failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueAck"),
        }
    }

    /// Negative acknowledge - return item to queue.
    pub async fn nack(&self, receipt_handle: &str) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueNack {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                move_to_dlq: false,
                error_message: None,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueNackResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    bail!("nack failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueNack"),
        }
    }

    /// Negative acknowledge and move to dead letter queue.
    pub async fn nack_to_dlq(&self, receipt_handle: &str, error_message: Option<String>) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueNack {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                move_to_dlq: true,
                error_message,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueNackResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    bail!("nack_to_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueNack"),
        }
    }

    /// Extend the visibility timeout for a pending item.
    ///
    /// Returns the new visibility deadline (Unix epoch millis).
    pub async fn extend_visibility(&self, receipt_handle: &str, additional_timeout: Duration) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueExtendVisibility {
                queue_name: self.queue_name.clone(),
                receipt_handle: receipt_handle.to_string(),
                additional_timeout_ms: additional_timeout.as_millis() as u64,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueExtendVisibilityResult(result) => {
                if result.success {
                    Ok(result.new_deadline_ms.unwrap_or(0))
                } else {
                    bail!("extend_visibility failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueExtendVisibility"),
        }
    }

    /// Get queue status.
    pub async fn status(&self) -> Result<QueueStatusInfo> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueStatus {
                queue_name: self.queue_name.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::QueueStatusResult(result) => {
                if result.success {
                    Ok(QueueStatusInfo {
                        exists: result.exists,
                        visible_count: result.visible_count.unwrap_or(0),
                        pending_count: result.pending_count.unwrap_or(0),
                        dlq_count: result.dlq_count.unwrap_or(0),
                        total_enqueued: result.total_enqueued.unwrap_or(0),
                        total_acked: result.total_acked.unwrap_or(0),
                    })
                } else {
                    bail!("status failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueStatus"),
        }
    }

    /// Get items from the dead letter queue.
    pub async fn get_dlq(&self, max_items: u32) -> Result<Vec<QueueDLQItemInfo>> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueGetDLQ {
                queue_name: self.queue_name.clone(),
                max_items,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueGetDLQResult(result) => {
                if result.success {
                    Ok(result
                        .items
                        .into_iter()
                        .map(|item| QueueDLQItemInfo {
                            item_id: item.item_id,
                            payload: item.payload,
                            enqueued_at: Duration::from_millis(item.enqueued_at_ms),
                            delivery_attempts: item.delivery_attempts,
                            reason: item.reason,
                            moved_at: Duration::from_millis(item.moved_at_ms),
                            last_error: item.last_error,
                        })
                        .collect())
                } else {
                    bail!("get_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueGetDLQ"),
        }
    }

    /// Move a DLQ item back to the main queue.
    pub async fn redrive_dlq(&self, item_id: u64) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::QueueRedriveDLQ {
                queue_name: self.queue_name.clone(),
                item_id,
            })
            .await?;

        match response {
            ClientRpcResponse::QueueRedriveDLQResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    bail!("redrive_dlq failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
                }
            }
            _ => bail!("unexpected response type for QueueRedriveDLQ"),
        }
    }
}

/// Configuration for creating a queue.
#[derive(Debug, Clone, Default)]
pub struct QueueCreateConfig {
    /// Default visibility timeout.
    pub default_visibility_timeout: Option<Duration>,
    /// Default item TTL.
    pub default_ttl: Option<Duration>,
    /// Maximum delivery attempts before moving to DLQ.
    pub max_delivery_attempts: Option<u32>,
}

/// Options for enqueuing an item.
#[derive(Debug, Clone, Default)]
pub struct QueueEnqueueOptions {
    /// Item TTL (overrides queue default).
    pub ttl: Option<Duration>,
    /// Message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Item to enqueue in a batch operation.
#[derive(Debug, Clone)]
pub struct QueueEnqueueBatchItem {
    /// Item payload.
    pub payload: Vec<u8>,
    /// Item TTL.
    pub ttl: Option<Duration>,
    /// Message group ID.
    pub message_group_id: Option<String>,
    /// Deduplication ID.
    pub deduplication_id: Option<String>,
}

/// A dequeued item with receipt handle for acknowledgment.
#[derive(Debug, Clone)]
pub struct QueueDequeuedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Visibility deadline (as Duration from Unix epoch).
    pub visibility_deadline: Duration,
}

/// A peeked item (not dequeued).
#[derive(Debug, Clone)]
pub struct QueuePeekedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Expiration time (as Duration from Unix epoch), None if no expiration.
    pub expires_at: Option<Duration>,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
}

/// Queue status information.
#[derive(Debug, Clone)]
pub struct QueueStatusInfo {
    /// Whether the queue exists.
    pub exists: bool,
    /// Approximate number of visible items.
    pub visible_count: u64,
    /// Approximate number of pending items.
    pub pending_count: u64,
    /// Approximate number of DLQ items.
    pub dlq_count: u64,
    /// Total items enqueued.
    pub total_enqueued: u64,
    /// Total items acked.
    pub total_acked: u64,
}

/// A dead letter queue item.
#[derive(Debug, Clone)]
pub struct QueueDLQItemInfo {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (as Duration from Unix epoch).
    pub enqueued_at: Duration,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: String,
    /// Time moved to DLQ (as Duration from Unix epoch).
    pub moved_at: Duration,
    /// Last error message.
    pub last_error: Option<String>,
}

// =============================================================================
// Service Registry Client
// =============================================================================

/// Client for service registry operations.
///
/// Provides a high-level API for service registration, discovery,
/// and health management.
pub struct ServiceClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> ServiceClient<C> {
    /// Create a new service registry client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Register a service instance.
    ///
    /// Returns a registration handle with fencing token and deadline.
    pub async fn register(
        &self,
        service_name: &str,
        instance_id: &str,
        address: &str,
        options: ServiceRegisterOptions,
    ) -> Result<ServiceRegistration<C>> {
        use crate::client_rpc::ServiceRegisterResultResponse;

        let tags_json = serde_json::to_string(&options.tags).unwrap_or_else(|_| "[]".to_string());
        let custom_json = serde_json::to_string(&options.custom_metadata).unwrap_or_else(|_| "{}".to_string());

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceRegister {
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
                address: address.to_string(),
                version: options.version.clone(),
                tags: tags_json,
                weight: options.weight,
                custom_metadata: custom_json,
                ttl_ms: options.ttl.map(|d| d.as_millis() as u64).unwrap_or(0),
                lease_id: options.lease_id,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
                success: true,
                fencing_token: Some(token),
                deadline_ms: Some(deadline),
                ..
            }) => Ok(ServiceRegistration {
                client: self.client.clone(),
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
                fencing_token: token,
                deadline_ms: deadline,
            }),
            ClientRpcResponse::ServiceRegisterResult(result) => {
                bail!("service register failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceRegister"),
        }
    }

    /// Discover service instances.
    ///
    /// Returns a list of matching instances.
    pub async fn discover(
        &self,
        service_name: &str,
        filter: Option<ServiceDiscoveryFilter>,
    ) -> Result<Vec<ServiceInstanceInfo>> {
        use crate::client_rpc::ServiceDiscoverResultResponse;

        let filter = filter.unwrap_or_default();
        let tags_json = serde_json::to_string(&filter.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceDiscover {
                service_name: service_name.to_string(),
                healthy_only: filter.healthy_only,
                tags: tags_json,
                version_prefix: filter.version_prefix,
                limit: filter.limit,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceDiscoverResult(ServiceDiscoverResultResponse {
                success: true,
                instances,
                ..
            }) => Ok(instances
                .into_iter()
                .map(|inst| ServiceInstanceInfo {
                    instance_id: inst.instance_id,
                    service_name: inst.service_name,
                    address: inst.address,
                    health_status: inst.health_status,
                    version: inst.version,
                    tags: inst.tags,
                    weight: inst.weight,
                    custom_metadata: serde_json::from_str(&inst.custom_metadata).unwrap_or_default(),
                    registered_at: Duration::from_millis(inst.registered_at_ms),
                    last_heartbeat: Duration::from_millis(inst.last_heartbeat_ms),
                    deadline: Duration::from_millis(inst.deadline_ms),
                    lease_id: inst.lease_id,
                    fencing_token: inst.fencing_token,
                })
                .collect()),
            ClientRpcResponse::ServiceDiscoverResult(result) => {
                bail!("service discover failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceDiscover"),
        }
    }

    /// List service names by prefix.
    pub async fn list_services(&self, prefix: &str, limit: Option<u32>) -> Result<Vec<String>> {
        use crate::client_rpc::ServiceListResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceList {
                prefix: prefix.to_string(),
                limit: limit.unwrap_or(100),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceListResult(ServiceListResultResponse {
                success: true,
                services,
                ..
            }) => Ok(services),
            ClientRpcResponse::ServiceListResult(result) => {
                bail!("service list failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceList"),
        }
    }

    /// Get a specific service instance.
    pub async fn get_instance(&self, service_name: &str, instance_id: &str) -> Result<Option<ServiceInstanceInfo>> {
        use crate::client_rpc::ServiceGetInstanceResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceGetInstance {
                service_name: service_name.to_string(),
                instance_id: instance_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                success: true,
                found: true,
                instance: Some(inst),
                ..
            }) => Ok(Some(ServiceInstanceInfo {
                instance_id: inst.instance_id,
                service_name: inst.service_name,
                address: inst.address,
                health_status: inst.health_status,
                version: inst.version,
                tags: inst.tags,
                weight: inst.weight,
                custom_metadata: serde_json::from_str(&inst.custom_metadata).unwrap_or_default(),
                registered_at: Duration::from_millis(inst.registered_at_ms),
                last_heartbeat: Duration::from_millis(inst.last_heartbeat_ms),
                deadline: Duration::from_millis(inst.deadline_ms),
                lease_id: inst.lease_id,
                fencing_token: inst.fencing_token,
            })),
            ClientRpcResponse::ServiceGetInstanceResult(ServiceGetInstanceResultResponse {
                success: true,
                found: false,
                ..
            }) => Ok(None),
            ClientRpcResponse::ServiceGetInstanceResult(result) => {
                bail!("service get instance failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceGetInstance"),
        }
    }
}

/// Handle for a registered service instance.
///
/// Provides methods for heartbeat, health updates, and deregistration.
pub struct ServiceRegistration<C: CoordinationRpc> {
    client: Arc<C>,
    service_name: String,
    instance_id: String,
    fencing_token: u64,
    deadline_ms: u64,
}

impl<C: CoordinationRpc + 'static> ServiceRegistration<C> {
    /// Get the fencing token for this registration.
    pub fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    /// Get the TTL deadline in milliseconds since Unix epoch.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Send a heartbeat to renew the TTL.
    ///
    /// Returns the new deadline and current health status.
    pub async fn heartbeat(&mut self) -> Result<(u64, String)> {
        use crate::client_rpc::ServiceHeartbeatResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceHeartbeat {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceHeartbeatResult(ServiceHeartbeatResultResponse {
                success: true,
                new_deadline_ms: Some(deadline),
                health_status: Some(status),
                ..
            }) => {
                self.deadline_ms = deadline;
                Ok((deadline, status))
            }
            ClientRpcResponse::ServiceHeartbeatResult(result) => {
                bail!("service heartbeat failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceHeartbeat"),
        }
    }

    /// Update the health status.
    pub async fn set_health(&self, status: &str) -> Result<()> {
        use crate::client_rpc::ServiceUpdateHealthResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceUpdateHealth {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
                status: status.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceUpdateHealthResult(ServiceUpdateHealthResultResponse {
                success: true, ..
            }) => Ok(()),
            ClientRpcResponse::ServiceUpdateHealthResult(result) => {
                bail!("service update health failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceUpdateHealth"),
        }
    }

    /// Update instance metadata.
    pub async fn update_metadata(&self, updates: ServiceMetadataUpdate) -> Result<()> {
        use crate::client_rpc::ServiceUpdateMetadataResultResponse;

        let tags_json = updates.tags.map(|t| serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()));
        let custom_json =
            updates.custom_metadata.map(|c| serde_json::to_string(&c).unwrap_or_else(|_| "{}".to_string()));

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceUpdateMetadata {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
                version: updates.version,
                tags: tags_json,
                weight: updates.weight,
                custom_metadata: custom_json,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceUpdateMetadataResult(ServiceUpdateMetadataResultResponse {
                success: true,
                ..
            }) => Ok(()),
            ClientRpcResponse::ServiceUpdateMetadataResult(result) => {
                bail!("service update metadata failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceUpdateMetadata"),
        }
    }

    /// Deregister this instance.
    ///
    /// Returns true if the instance was registered.
    pub async fn deregister(self) -> Result<bool> {
        use crate::client_rpc::ServiceDeregisterResultResponse;

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::ServiceDeregister {
                service_name: self.service_name.clone(),
                instance_id: self.instance_id.clone(),
                fencing_token: self.fencing_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ServiceDeregisterResult(ServiceDeregisterResultResponse {
                success: true,
                was_registered,
                ..
            }) => Ok(was_registered),
            ClientRpcResponse::ServiceDeregisterResult(result) => {
                bail!("service deregister failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
            _ => bail!("unexpected response type for ServiceDeregister"),
        }
    }

    /// Start a background task that sends heartbeats automatically.
    ///
    /// Returns a handle that can be used to stop the heartbeat task.
    pub fn start_heartbeat_task(self, interval: Duration) -> ServiceHeartbeatHandle<C> {
        let cancel_token = CancellationToken::new();
        let running_token = cancel_token.clone();

        let registration = Arc::new(tokio::sync::Mutex::new(self));
        let reg_clone = registration.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = running_token.cancelled() => {
                        debug!("service heartbeat task cancelled");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        let mut reg = reg_clone.lock().await;
                        match reg.heartbeat().await {
                            Ok((deadline, _status)) => {
                                debug!(
                                    service = %reg.service_name,
                                    instance = %reg.instance_id,
                                    new_deadline = deadline,
                                    "service heartbeat sent"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    service = %reg.service_name,
                                    instance = %reg.instance_id,
                                    error = %e,
                                    "service heartbeat failed"
                                );
                            }
                        }
                    }
                }
            }
        });

        ServiceHeartbeatHandle {
            cancel_token,
            task: Some(task),
            registration,
        }
    }
}

/// Handle for a background service heartbeat task.
pub struct ServiceHeartbeatHandle<C: CoordinationRpc> {
    cancel_token: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
    registration: Arc<tokio::sync::Mutex<ServiceRegistration<C>>>,
}

impl<C: CoordinationRpc + 'static> ServiceHeartbeatHandle<C> {
    /// Check if the heartbeat task is still running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().map(|t| !t.is_finished()).unwrap_or(false)
    }

    /// Stop the heartbeat task.
    pub fn stop(mut self) {
        self.cancel_token.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    /// Stop heartbeats and deregister the instance.
    pub async fn stop_and_deregister(mut self) -> Result<bool> {
        self.cancel_token.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }

        // Take ownership of the registration
        let reg = Arc::try_unwrap(self.registration)
            .map_err(|_| anyhow::anyhow!("registration still in use"))?
            .into_inner();
        reg.deregister().await
    }

    /// Get the service name.
    pub async fn service_name(&self) -> String {
        self.registration.lock().await.service_name.clone()
    }

    /// Get the instance ID.
    pub async fn instance_id(&self) -> String {
        self.registration.lock().await.instance_id.clone()
    }

    /// Get the fencing token.
    pub async fn fencing_token(&self) -> u64 {
        self.registration.lock().await.fencing_token
    }
}

/// Options for service registration.
#[derive(Debug, Clone, Default)]
pub struct ServiceRegisterOptions {
    /// Version string.
    pub version: String,
    /// Tags for filtering.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata.
    pub custom_metadata: std::collections::HashMap<String, String>,
    /// TTL for the registration (None = default).
    pub ttl: Option<Duration>,
    /// Optional lease ID to attach to.
    pub lease_id: Option<u64>,
}

/// Filter for service discovery.
#[derive(Debug, Clone, Default)]
pub struct ServiceDiscoveryFilter {
    /// Only return healthy instances.
    pub healthy_only: bool,
    /// Filter by tags (all must match).
    pub tags: Option<Vec<String>>,
    /// Filter by version prefix.
    pub version_prefix: Option<String>,
    /// Maximum instances to return.
    pub limit: Option<u32>,
}

/// Information about a discovered service instance.
#[derive(Debug, Clone)]
pub struct ServiceInstanceInfo {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Service name.
    pub service_name: String,
    /// Network address.
    pub address: String,
    /// Health status.
    pub health_status: String,
    /// Version string.
    pub version: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata.
    pub custom_metadata: std::collections::HashMap<String, String>,
    /// Registration time (as Duration from Unix epoch).
    pub registered_at: Duration,
    /// Last heartbeat time (as Duration from Unix epoch).
    pub last_heartbeat: Duration,
    /// TTL deadline (as Duration from Unix epoch).
    pub deadline: Duration,
    /// Associated lease ID.
    pub lease_id: Option<u64>,
    /// Fencing token.
    pub fencing_token: u64,
}

/// Updates for service instance metadata.
#[derive(Debug, Clone, Default)]
pub struct ServiceMetadataUpdate {
    /// New version (None = keep current).
    pub version: Option<String>,
    /// New tags (None = keep current).
    pub tags: Option<Vec<String>>,
    /// New weight (None = keep current).
    pub weight: Option<u32>,
    /// New custom metadata (None = keep current).
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::client_rpc::CounterResultResponse;
    use crate::client_rpc::LeaseGrantResultResponse;
    use crate::client_rpc::LeaseInfo;
    use crate::client_rpc::LeaseKeepaliveResultResponse;
    use crate::client_rpc::LeaseListResultResponse;
    use crate::client_rpc::LeaseRevokeResultResponse;
    use crate::client_rpc::LeaseTimeToLiveResultResponse;
    use crate::client_rpc::LockResultResponse;
    use crate::client_rpc::RateLimiterResultResponse;
    use crate::client_rpc::SequenceResultResponse;
    use crate::client_rpc::WriteResultResponse;

    /// Mock RPC client for testing.
    struct MockRpcClient {
        responses: Mutex<Vec<ClientRpcResponse>>,
    }

    impl MockRpcClient {
        fn new(responses: Vec<ClientRpcResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl CoordinationRpc for MockRpcClient {
        async fn send_coordination_request(&self, _request: ClientRpcRequest) -> Result<ClientRpcResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                bail!("no more mock responses");
            }
            Ok(responses.remove(0))
        }
    }

    #[tokio::test]
    async fn test_counter_client_increment() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(42),
            error: None,
        })]));

        let counter = CounterClient::new(client, "test");
        let value = counter.increment().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_sequence_client_next() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SequenceResult(SequenceResultResponse {
            success: true,
            value: Some(1001),
            error: None,
        })]));

        let seq = SequenceClient::new(client, "test");
        let value = seq.next().await.unwrap();
        assert_eq!(value, 1001);
    }

    #[tokio::test]
    async fn test_rate_limiter_acquired() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                success: true,
                tokens_remaining: Some(99),
                retry_after_ms: None,
                error: None,
            })]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_acquired());
        assert_eq!(result, RateLimitResult::Acquired { tokens_remaining: 99 });
    }

    #[tokio::test]
    async fn test_rate_limiter_rate_limited() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                success: false,
                tokens_remaining: Some(0),
                retry_after_ms: Some(500),
                error: None,
            })]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_rate_limited());
        assert_eq!(result, RateLimitResult::RateLimited { retry_after_ms: 500 });
    }

    #[tokio::test]
    async fn test_lock_client_acquire() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockResult(LockResultResponse {
            success: true,
            fencing_token: Some(42),
            holder_id: Some("holder-1".to_string()),
            deadline_ms: Some(1234567890),
            error: None,
        })]));

        let lock = LockClient::new(client, "test");
        let guard = lock.acquire("holder-1", Duration::from_secs(30), Duration::from_secs(5)).await.unwrap();

        assert_eq!(guard.fencing_token(), 42);
        assert_eq!(guard.deadline_ms(), 1234567890);
    }

    // ================================
    // LeaseClient tests
    // ================================

    #[tokio::test]
    async fn test_lease_grant_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                success: true,
                lease_id: Some(12345),
                ttl_seconds: Some(60),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant(60).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.ttl_seconds, 60);
    }

    #[tokio::test]
    async fn test_lease_grant_with_id_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                success: true,
                lease_id: Some(99999),
                ttl_seconds: Some(300),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant_with_id(300, Some(99999)).await.unwrap();

        assert_eq!(result.lease_id, 99999);
        assert_eq!(result.ttl_seconds, 300);
    }

    #[tokio::test]
    async fn test_lease_grant_failure() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
                success: false,
                lease_id: None,
                ttl_seconds: None,
                error: Some("Lease ID already exists".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.grant_with_id(60, Some(12345)).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease ID already exists"));
    }

    #[tokio::test]
    async fn test_lease_revoke_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                success: true,
                keys_deleted: Some(5),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.revoke(12345).await.unwrap();

        assert_eq!(result.keys_deleted, 5);
    }

    #[tokio::test]
    async fn test_lease_revoke_not_found() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
                success: false,
                keys_deleted: None,
                error: Some("Lease not found".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.revoke(99999).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease not found"));
    }

    #[tokio::test]
    async fn test_lease_keepalive_success() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                success: true,
                lease_id: Some(12345),
                ttl_seconds: Some(60),
                error: None,
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.keepalive(12345).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.ttl_seconds, 60);
    }

    #[tokio::test]
    async fn test_lease_keepalive_expired() {
        let client =
            Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                success: false,
                lease_id: None,
                ttl_seconds: None,
                error: Some("Lease expired or not found".to_string()),
            })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.keepalive(12345).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease expired or not found"));
    }

    #[tokio::test]
    async fn test_lease_time_to_live_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                success: true,
                lease_id: Some(12345),
                granted_ttl_seconds: Some(60),
                remaining_ttl_seconds: Some(45),
                keys: Some(vec!["key1".to_string(), "key2".to_string()]),
                error: None,
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(12345, true).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert_eq!(result.granted_ttl_seconds, 60);
        assert_eq!(result.remaining_ttl_seconds, 45);
        assert_eq!(result.keys, vec!["key1".to_string(), "key2".to_string()]);
    }

    #[tokio::test]
    async fn test_lease_time_to_live_without_keys() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                success: true,
                lease_id: Some(12345),
                granted_ttl_seconds: Some(60),
                remaining_ttl_seconds: Some(45),
                keys: None,
                error: None,
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(12345, false).await.unwrap();

        assert_eq!(result.lease_id, 12345);
        assert!(result.keys.is_empty());
    }

    #[tokio::test]
    async fn test_lease_time_to_live_not_found() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                success: false,
                lease_id: None,
                granted_ttl_seconds: None,
                remaining_ttl_seconds: None,
                keys: None,
                error: Some("Lease not found".to_string()),
            },
        )]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.time_to_live(99999, false).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Lease not found"));
    }

    #[tokio::test]
    async fn test_lease_list_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
            success: true,
            leases: Some(vec![
                LeaseInfo {
                    lease_id: 100,
                    granted_ttl_seconds: 60,
                    remaining_ttl_seconds: 50,
                    attached_keys: 2,
                },
                LeaseInfo {
                    lease_id: 200,
                    granted_ttl_seconds: 120,
                    remaining_ttl_seconds: 100,
                    attached_keys: 5,
                },
            ]),
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.list().await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].lease_id, 100);
        assert_eq!(result[1].lease_id, 200);
    }

    #[tokio::test]
    async fn test_lease_list_empty() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
            success: true,
            leases: Some(vec![]),
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.list().await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_lease_put_with_lease_success() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::WriteResult(WriteResultResponse {
            success: true,
            error: None,
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.put_with_lease("mykey", b"myvalue", 12345).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lease_put_with_lease_failure() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::WriteResult(WriteResultResponse {
            success: false,
            error: Some("Lease not found".to_string()),
        })]));

        let lease_client = LeaseClient::new(client);
        let result = lease_client.put_with_lease("mykey", b"myvalue", 12345).await;

        assert!(result.is_err());
    }

    // ================================
    // LeaseKeepaliveHandle tests
    // ================================

    #[tokio::test]
    async fn test_lease_keepalive_handle_stop() {
        // This test verifies that the handle can be stopped
        let client = Arc::new(MockRpcClient::new(vec![]));
        let lease_client = LeaseClient::new(client);

        let handle = lease_client.start_keepalive(12345, Duration::from_millis(100));
        assert!(handle.is_running());
        assert_eq!(handle.lease_id(), 12345);

        handle.stop();
        // After stop, the task should be cancelled
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Can't check is_running after stop since handle is consumed
    }

    #[tokio::test]
    async fn test_lease_keepalive_sends_requests() {
        use std::sync::atomic::AtomicU32;
        use std::sync::atomic::Ordering;

        // Create a mock that counts keepalive requests
        struct CountingMockClient {
            keepalive_count: AtomicU32,
        }

        impl CoordinationRpc for CountingMockClient {
            async fn send_coordination_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
                match request {
                    ClientRpcRequest::LeaseKeepalive { .. } => {
                        self.keepalive_count.fetch_add(1, Ordering::SeqCst);
                        Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
                            success: true,
                            lease_id: Some(12345),
                            ttl_seconds: Some(60),
                            error: None,
                        }))
                    }
                    _ => bail!("unexpected request"),
                }
            }
        }

        let mock = Arc::new(CountingMockClient {
            keepalive_count: AtomicU32::new(0),
        });
        let lease_client = LeaseClient::new(mock.clone());

        // Start keepalive with 50ms interval
        let handle = lease_client.start_keepalive(12345, Duration::from_millis(50));

        // Wait for a few keepalives
        tokio::time::sleep(Duration::from_millis(180)).await;

        handle.stop();

        // Should have sent at least 2-3 keepalives (one immediately, plus 2-3 more)
        let count = mock.keepalive_count.load(Ordering::SeqCst);
        assert!(count >= 2, "expected at least 2 keepalives, got {}", count);
    }
}
