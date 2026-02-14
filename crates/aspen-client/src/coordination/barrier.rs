//! Barrier client for distributed synchronization barriers.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::CoordinationRpc;

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
        use aspen_client_api::BarrierResultResponse;

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
        use aspen_client_api::BarrierResultResponse;

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
        use aspen_client_api::BarrierResultResponse;

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
