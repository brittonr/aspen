//! Atomic counter clients.

use std::sync::Arc;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

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
