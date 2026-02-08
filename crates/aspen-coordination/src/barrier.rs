//! Distributed barrier for coordinating multiple participants.
//!
//! A barrier allows N participants to wait until all have arrived before
//! proceeding. This implements a "double barrier" pattern similar to etcd:
//!
//! 1. **Enter phase**: Participants register and wait until all arrive
//! 2. **Work phase**: All participants proceed with their work
//! 3. **Leave phase**: Participants deregister and wait until all leave
//!
//! The barrier is stored as a JSON object in the key-value store.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

/// Barrier key prefix.
const BARRIER_PREFIX: &str = "__barrier:";

/// Barrier state stored in the key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierState {
    /// Barrier name.
    pub name: String,
    /// Number of participants required.
    pub required_count: u32,
    /// Current participants.
    pub participants: Vec<String>,
    /// Current phase.
    pub phase: BarrierPhase,
    /// Creation time (ms since epoch).
    pub created_at_ms: u64,
}

/// Barrier phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarrierPhase {
    /// Waiting for participants to enter.
    Waiting,
    /// All participants have arrived, work in progress.
    Ready,
    /// Participants are leaving.
    Leaving,
}

impl BarrierPhase {
    /// Convert the phase to a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            BarrierPhase::Waiting => "waiting",
            BarrierPhase::Ready => "ready",
            BarrierPhase::Leaving => "leaving",
        }
    }
}

/// Manager for distributed barrier operations.
pub struct BarrierManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> BarrierManager<S> {
    /// Create a new barrier manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Enter a barrier, waiting until all participants arrive.
    ///
    /// Returns (current_count, phase) on success.
    pub async fn enter(
        &self,
        name: &str,
        participant_id: &str,
        required_count: u32,
        timeout: Option<Duration>,
    ) -> Result<(u32, String)> {
        let key = format!("{}{}", BARRIER_PREFIX, name);
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        loop {
            // Check timeout
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                bail!("barrier enter timeout");
            }

            // Read current state
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Create new barrier
                    // If required_count is 1, we're already ready
                    let initial_phase = crate::pure::compute_initial_barrier_phase(required_count);

                    let state = BarrierState {
                        name: name.to_string(),
                        required_count,
                        participants: vec![participant_id.to_string()],
                        phase: initial_phase,
                        created_at_ms: crate::types::now_unix_ms(),
                    };
                    let new_json = serde_json::to_string(&state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: None,
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(name, participant_id, "barrier created, first participant");
                            return Ok((1, initial_phase.as_str().to_string()));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            // Race condition, retry
                            continue;
                        }
                        Err(e) => bail!("barrier CAS failed: {}", e),
                    }
                }
                Some(state) => {
                    // Check if already in barrier
                    if state.participants.contains(&participant_id.to_string()) {
                        // Already entered, just return current status
                        let count = state.participants.len() as u32;
                        if count >= required_count {
                            return Ok((count, BarrierPhase::Ready.as_str().to_string()));
                        } else {
                            // Wait for more participants
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            continue;
                        }
                    }

                    // Check if barrier is in wrong phase
                    if state.phase == BarrierPhase::Leaving {
                        // Wait for leave phase to complete
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }

                    // Add participant
                    let mut new_state = state.clone();
                    new_state.participants.push(participant_id.to_string());

                    let count = new_state.participants.len() as u32;
                    if crate::pure::should_transition_to_ready(count, required_count) {
                        new_state.phase = BarrierPhase::Ready;
                    }

                    let old_json = serde_json::to_string(&state)?;
                    let new_json = serde_json::to_string(&new_state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(name, participant_id, count, "joined barrier");
                            if count >= required_count {
                                return Ok((count, BarrierPhase::Ready.as_str().to_string()));
                            }
                            // Wait for more participants
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            // Race condition, retry
                            continue;
                        }
                        Err(e) => bail!("barrier CAS failed: {}", e),
                    }
                }
            }

            // Wait before polling again
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Leave a barrier, waiting until all participants leave.
    ///
    /// Returns (remaining_count, phase) on success.
    pub async fn leave(&self, name: &str, participant_id: &str, timeout: Option<Duration>) -> Result<(u32, String)> {
        let key = format!("{}{}", BARRIER_PREFIX, name);
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        loop {
            // Check timeout
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                bail!("barrier leave timeout");
            }

            // Read current state
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Barrier doesn't exist, already left
                    return Ok((0, "completed".to_string()));
                }
                Some(state) => {
                    // Check if not in barrier
                    if !state.participants.contains(&participant_id.to_string()) {
                        // Not in barrier, check if we need to wait for others
                        if state.participants.is_empty() {
                            return Ok((0, "completed".to_string()));
                        }
                        // Wait for others to leave
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }

                    // Remove participant
                    let mut new_state = state.clone();
                    new_state.participants.retain(|p| p != participant_id);
                    new_state.phase = BarrierPhase::Leaving;

                    let remaining = new_state.participants.len() as u32;

                    let old_json = serde_json::to_string(&state)?;

                    if remaining == 0 {
                        // Last participant, delete the barrier
                        match self
                            .store
                            .write(WriteRequest {
                                command: WriteCommand::CompareAndSwap {
                                    key: key.clone(),
                                    expected: Some(old_json),
                                    new_value: "".to_string(), // Empty = delete
                                },
                            })
                            .await
                        {
                            Ok(_) => {
                                debug!(name, participant_id, "barrier completed, last to leave");
                                return Ok((0, "completed".to_string()));
                            }
                            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                                continue;
                            }
                            Err(e) => bail!("barrier delete failed: {}", e),
                        }
                    } else {
                        let new_json = serde_json::to_string(&new_state)?;

                        match self
                            .store
                            .write(WriteRequest {
                                command: WriteCommand::CompareAndSwap {
                                    key: key.clone(),
                                    expected: Some(old_json),
                                    new_value: new_json,
                                },
                            })
                            .await
                        {
                            Ok(_) => {
                                debug!(name, participant_id, remaining, "left barrier");
                            }
                            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                                continue;
                            }
                            Err(e) => bail!("barrier CAS failed: {}", e),
                        }
                    }
                }
            }

            // Wait for all to leave
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Get barrier status without modifying it.
    ///
    /// Returns (current_count, required_count, phase).
    pub async fn status(&self, name: &str) -> Result<(u32, u32, String)> {
        let key = format!("{}{}", BARRIER_PREFIX, name);

        match self.read_state(&key).await? {
            Some(state) => {
                Ok((state.participants.len() as u32, state.required_count, state.phase.as_str().to_string()))
            }
            None => Ok((0, 0, "none".to_string())),
        }
    }

    /// Read barrier state from the store.
    async fn read_state(&self, key: &str) -> Result<Option<BarrierState>> {
        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let state: BarrierState = serde_json::from_str(&value)?;
                    Ok(Some(state))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("barrier read failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_barrier_single_participant() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = BarrierManager::new(store);

        // Enter with required_count = 1
        let (count, phase) = manager.enter("test", "p1", 1, Some(Duration::from_secs(1))).await.unwrap();

        assert_eq!(count, 1);
        assert_eq!(phase, "ready");

        // Leave
        let (remaining, phase) = manager.leave("test", "p1", Some(Duration::from_secs(1))).await.unwrap();

        assert_eq!(remaining, 0);
        assert_eq!(phase, "completed");
    }

    #[tokio::test]
    async fn test_barrier_status() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = BarrierManager::new(store);

        // No barrier yet
        let (count, required, phase) = manager.status("test").await.unwrap();
        assert_eq!(count, 0);
        assert_eq!(required, 0);
        assert_eq!(phase, "none");

        // Create barrier
        manager.enter("test", "p1", 3, Some(Duration::from_millis(100))).await.ok(); // Will timeout waiting for others

        let (count, required, phase) = manager.status("test").await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(required, 3);
        assert_eq!(phase, "waiting");
    }
}
