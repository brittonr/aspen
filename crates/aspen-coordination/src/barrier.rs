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

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
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

/// Internal result type for barrier leave operations.
enum LeaveResult {
    /// Barrier completed (all participants left).
    Completed,
    /// Waiting for other participants to leave.
    WaitForOthers,
    /// CAS conflict, retry required.
    RetryRequired,
    /// Successfully left but others still in barrier.
    LeftWaitingForOthers,
}

/// Internal result type for barrier enter operations.
enum EnterResult {
    /// Successfully entered, barrier is ready.
    Ready(u32),
    /// Successfully joined, waiting for more participants.
    Waiting,
    /// Barrier is in leaving phase, need to wait.
    InLeavingPhase,
    /// CAS conflict, retry required.
    RetryRequired,
}

/// Manager for distributed barrier operations.
pub struct BarrierManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> BarrierManager<S> {
    /// Create a new barrier manager.
    pub fn new(store: Arc<S>) -> Self {
        debug_assert!(Arc::strong_count(&store) >= 1, "BARRIER: store Arc must have at least 1 strong reference");
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
        // Tiger Style: argument validation
        debug_assert!(!name.is_empty(), "BARRIER: name must not be empty");
        debug_assert!(!participant_id.is_empty(), "BARRIER: participant_id must not be empty");
        debug_assert!(required_count > 0, "BARRIER: required_count must be positive");

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
                None => match self.enter_create(name, participant_id, required_count, &key).await? {
                    EnterResult::Ready(count) => return Ok((count, BarrierPhase::Ready.as_str().to_string())),
                    EnterResult::Waiting => return Ok((1, BarrierPhase::Waiting.as_str().to_string())),
                    EnterResult::RetryRequired => continue,
                    EnterResult::InLeavingPhase => unreachable!(),
                },
                Some(state) => match self.enter_join(name, participant_id, required_count, &key, &state).await? {
                    EnterResult::Ready(count) => return Ok((count, BarrierPhase::Ready.as_str().to_string())),
                    EnterResult::Waiting | EnterResult::InLeavingPhase => {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                    EnterResult::RetryRequired => continue,
                },
            }
        }
    }

    /// Create a new barrier with the first participant.
    async fn enter_create(
        &self,
        name: &str,
        participant_id: &str,
        required_count: u32,
        key: &str,
    ) -> Result<EnterResult> {
        assert!(required_count > 0, "BARRIER: required_count must be > 0");
        // If required_count is 1, we're already ready
        let initial_phase = crate::verified::compute_initial_barrier_phase(required_count);

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
                    key: key.to_string(),
                    expected: None,
                    new_value: new_json,
                },
            })
            .await
        {
            Ok(_) => {
                debug!(name, participant_id, "barrier created, first participant");
                if initial_phase == BarrierPhase::Ready {
                    Ok(EnterResult::Ready(1))
                } else {
                    Ok(EnterResult::Waiting)
                }
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(EnterResult::RetryRequired),
            Err(e) => bail!("barrier CAS failed: {}", e),
        }
    }

    /// Join an existing barrier.
    async fn enter_join(
        &self,
        name: &str,
        participant_id: &str,
        required_count: u32,
        key: &str,
        state: &BarrierState,
    ) -> Result<EnterResult> {
        debug_assert!(
            state.participants.len() as u32 <= state.required_count * 2,
            "BARRIER: participant count ({}) far exceeds required ({})",
            state.participants.len(),
            state.required_count
        );

        // Check if already in barrier
        if state.participants.contains(&participant_id.to_string()) {
            let count = state.participants.len() as u32;
            if count >= required_count {
                return Ok(EnterResult::Ready(count));
            }
            return Ok(EnterResult::Waiting);
        }

        // Check if barrier is in wrong phase
        if state.phase == BarrierPhase::Leaving {
            return Ok(EnterResult::InLeavingPhase);
        }

        // Add participant
        let mut new_state = state.clone();
        new_state.participants.push(participant_id.to_string());

        let count = new_state.participants.len() as u32;
        if crate::verified::should_transition_to_ready(count, required_count) {
            new_state.phase = BarrierPhase::Ready;
            debug_assert!(count >= required_count, "BARRIER: transition to Ready requires count >= required_count");
        }

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: key.to_string(),
                    expected: Some(old_json),
                    new_value: new_json,
                },
            })
            .await
        {
            Ok(_) => {
                debug!(name, participant_id, count, "joined barrier");
                if count >= required_count {
                    Ok(EnterResult::Ready(count))
                } else {
                    Ok(EnterResult::Waiting)
                }
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(EnterResult::RetryRequired),
            Err(e) => bail!("barrier CAS failed: {}", e),
        }
    }

    /// Leave a barrier, waiting until all participants leave.
    ///
    /// Returns (remaining_count, phase) on success.
    pub async fn leave(&self, name: &str, participant_id: &str, timeout: Option<Duration>) -> Result<(u32, String)> {
        // Tiger Style: argument validation
        debug_assert!(!name.is_empty(), "BARRIER: name must not be empty for leave");
        debug_assert!(!participant_id.is_empty(), "BARRIER: participant_id must not be empty for leave");

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
                    return Ok((0, "completed".to_string()));
                }
                Some(state) => match self.leave_process_state(name, participant_id, &key, state).await? {
                    LeaveResult::Completed => return Ok((0, "completed".to_string())),
                    LeaveResult::WaitForOthers | LeaveResult::RetryRequired => {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                    LeaveResult::LeftWaitingForOthers => {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                },
            }
        }
    }

    /// Process barrier state during leave operation.
    async fn leave_process_state(
        &self,
        name: &str,
        participant_id: &str,
        key: &str,
        state: BarrierState,
    ) -> Result<LeaveResult> {
        // Check if not in barrier
        if !state.participants.contains(&participant_id.to_string()) {
            if state.participants.is_empty() {
                return Ok(LeaveResult::Completed);
            }
            return Ok(LeaveResult::WaitForOthers);
        }

        // Remove participant
        let mut new_state = state.clone();
        let count_before = new_state.participants.len();
        new_state.participants.retain(|p| p != participant_id);
        new_state.phase = BarrierPhase::Leaving;

        debug_assert!(
            new_state.participants.len() < count_before,
            "BARRIER: participant should have been removed but count unchanged"
        );

        let remaining = new_state.participants.len() as u32;
        let old_json = serde_json::to_string(&state)?;

        if remaining == 0 {
            self.leave_delete_barrier(name, participant_id, key, old_json).await
        } else {
            self.leave_update_state(name, participant_id, key, old_json, &new_state, remaining).await
        }
    }

    /// Delete barrier when last participant leaves.
    async fn leave_delete_barrier(
        &self,
        name: &str,
        participant_id: &str,
        key: &str,
        old_json: String,
    ) -> Result<LeaveResult> {
        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: key.to_string(),
                    expected: Some(old_json),
                    new_value: "".to_string(),
                },
            })
            .await
        {
            Ok(_) => {
                debug!(name, participant_id, "barrier completed, last to leave");
                Ok(LeaveResult::Completed)
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(LeaveResult::RetryRequired),
            Err(e) => bail!("barrier delete failed: {}", e),
        }
    }

    /// Update barrier state after participant leaves.
    async fn leave_update_state(
        &self,
        name: &str,
        participant_id: &str,
        key: &str,
        old_json: String,
        new_state: &BarrierState,
        remaining: u32,
    ) -> Result<LeaveResult> {
        let new_json = serde_json::to_string(new_state)?;

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: key.to_string(),
                    expected: Some(old_json),
                    new_value: new_json,
                },
            })
            .await
        {
            Ok(_) => {
                debug!(name, participant_id, remaining, "left barrier");
                Ok(LeaveResult::LeftWaitingForOthers)
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(LeaveResult::RetryRequired),
            Err(e) => bail!("barrier CAS failed: {}", e),
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
                    let state: BarrierState = serde_json::from_str(&value)
                        .with_context(|| format!("failed to parse barrier state for key '{}'", key))?;
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
    use aspen_testing::DeterministicKeyValueStore;

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
