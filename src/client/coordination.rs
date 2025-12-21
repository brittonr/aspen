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

use anyhow::{Result, bail};

use crate::client_rpc::{ClientRpcRequest, ClientRpcResponse};

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
                    bail!(
                        "lock acquisition failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
    pub async fn try_acquire(
        &self,
        holder_id: impl Into<String>,
        ttl: Duration,
    ) -> Result<Option<RemoteLockGuard<C>>> {
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
                    bail!(
                        "lock release failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
                    bail!(
                        "lock renewal failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
            .send_coordination_request(ClientRpcRequest::CounterGet {
                key: self.key.clone(),
            })
            .await?;

        Self::extract_value(response)
    }

    /// Increment the counter by 1 and return the new value.
    pub async fn increment(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterIncrement {
                key: self.key.clone(),
            })
            .await?;

        Self::extract_value(response)
    }

    /// Decrement the counter by 1 and return the new value.
    pub async fn decrement(&self) -> Result<u64> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::CounterDecrement {
                key: self.key.clone(),
            })
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
                    bail!(
                        "counter operation failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
            .send_coordination_request(ClientRpcRequest::SignedCounterGet {
                key: self.key.clone(),
            })
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
            .send_coordination_request(ClientRpcRequest::SequenceNext {
                key: self.key.clone(),
            })
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
            .send_coordination_request(ClientRpcRequest::SequenceCurrent {
                key: self.key.clone(),
            })
            .await?;

        Self::extract_value(response)
    }

    fn extract_value(response: ClientRpcResponse) -> Result<u64> {
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                if result.success {
                    Ok(result.value.unwrap_or(0))
                } else {
                    bail!(
                        "sequence operation failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
            ClientRpcResponse::RateLimiterResult(result) => {
                Ok(result.tokens_remaining.unwrap_or(0))
            }
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
                    bail!(
                        "rate limiter reset failed: {}",
                        result.error.unwrap_or_else(|| "unknown error".to_string())
                    )
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

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::BatchRead { keys })
            .await?;

        match response {
            ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                success,
                values,
                error,
            }) => {
                if success {
                    Ok(values.unwrap_or_default())
                } else {
                    bail!(
                        "batch read failed: {}",
                        error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
        use crate::client_rpc::{BatchWriteOperation, BatchWriteResultResponse};

        let ops: Vec<BatchWriteOperation> = operations
            .into_iter()
            .map(|op| match op {
                BatchWriteOp::Set { key, value } => BatchWriteOperation::Set { key, value },
                BatchWriteOp::Delete { key } => BatchWriteOperation::Delete { key },
            })
            .collect();

        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::BatchWrite { operations: ops })
            .await?;

        match response {
            ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                success,
                operations_applied,
                error,
            }) => {
                if success {
                    Ok(operations_applied.unwrap_or(0))
                } else {
                    bail!(
                        "batch write failed: {}",
                        error.unwrap_or_else(|| "unknown error".to_string())
                    )
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
        use crate::client_rpc::{
            BatchCondition, BatchWriteOperation, ConditionalBatchWriteResultResponse,
        };

        let conds: Vec<BatchCondition> = conditions
            .into_iter()
            .map(|c| match c {
                BatchConditionOp::ValueEquals { key, expected } => {
                    BatchCondition::ValueEquals { key, expected }
                }
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
            ClientRpcResponse::ConditionalBatchWriteResult(
                ConditionalBatchWriteResultResponse {
                    success: _,
                    conditions_met,
                    operations_applied,
                    failed_condition_index,
                    failed_condition_reason,
                    error,
                },
            ) => {
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
    Set { key: String, value: Vec<u8> },
    /// Delete a key.
    Delete { key: String },
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
    ValueEquals { key: String, expected: Vec<u8> },
    /// Key must exist.
    KeyExists { key: String },
    /// Key must not exist.
    KeyNotExists { key: String },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_rpc::{
        CounterResultResponse, LockResultResponse, RateLimiterResultResponse,
        SequenceResultResponse,
    };
    use std::sync::Mutex;

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
        async fn send_coordination_request(
            &self,
            _request: ClientRpcRequest,
        ) -> Result<ClientRpcResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                bail!("no more mock responses");
            }
            Ok(responses.remove(0))
        }
    }

    #[tokio::test]
    async fn test_counter_client_increment() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::CounterResult(
            CounterResultResponse {
                success: true,
                value: Some(42),
                error: None,
            },
        )]));

        let counter = CounterClient::new(client, "test");
        let value = counter.increment().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_sequence_client_next() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::SequenceResult(
            SequenceResultResponse {
                success: true,
                value: Some(1001),
                error: None,
            },
        )]));

        let seq = SequenceClient::new(client, "test");
        let value = seq.next().await.unwrap();
        assert_eq!(value, 1001);
    }

    #[tokio::test]
    async fn test_rate_limiter_acquired() {
        let client = Arc::new(MockRpcClient::new(vec![
            ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                success: true,
                tokens_remaining: Some(99),
                retry_after_ms: None,
                error: None,
            }),
        ]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_acquired());
        assert_eq!(
            result,
            RateLimitResult::Acquired {
                tokens_remaining: 99
            }
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_rate_limited() {
        let client = Arc::new(MockRpcClient::new(vec![
            ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
                success: false,
                tokens_remaining: Some(0),
                retry_after_ms: Some(500),
                error: None,
            }),
        ]));

        let limiter = RateLimiterClient::new(client, "test", 100.0, 100);
        let result = limiter.try_acquire().await.unwrap();
        assert!(result.is_rate_limited());
        assert_eq!(
            result,
            RateLimitResult::RateLimited {
                retry_after_ms: 500
            }
        );
    }

    #[tokio::test]
    async fn test_lock_client_acquire() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockResult(
            LockResultResponse {
                success: true,
                fencing_token: Some(42),
                holder_id: Some("holder-1".to_string()),
                deadline_ms: Some(1234567890),
                error: None,
            },
        )]));

        let lock = LockClient::new(client, "test");
        let guard = lock
            .acquire("holder-1", Duration::from_secs(30), Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(guard.fencing_token(), 42);
        assert_eq!(guard.deadline_ms(), 1234567890);
    }
}
