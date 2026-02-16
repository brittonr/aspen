//! Rate limiter client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

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
                if result.is_success {
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
                if result.is_success {
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
