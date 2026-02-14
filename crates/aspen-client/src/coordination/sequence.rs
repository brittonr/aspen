//! Sequence generator client.

use std::sync::Arc;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

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
