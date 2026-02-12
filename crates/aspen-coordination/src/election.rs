//! Distributed leader election using distributed locks.
//!
//! Provides leader election with:
//! - Fencing tokens for split-brain prevention
//! - Automatic leadership renewal with configurable TTL
//! - Leadership change callbacks
//! - Graceful stepdown support
//!
//! # Verified Properties (see `verus/election_*.rs`)
//!
//! 1. **Single Leader**: At most one leader at any time (via lock)
//! 2. **Fencing Token Monotonicity**: Each term has strictly greater token
//! 3. **Valid State Transitions**: Follower <-> Transitioning <-> Leader

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aspen_core::KeyValueStore;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CoordinationError;
use crate::lock::DistributedLock;
use crate::lock::LockConfig;
use crate::lock::LockGuard;
use crate::types::FencingToken;

/// Configuration for leader election.
#[derive(Debug, Clone)]
pub struct ElectionConfig {
    /// TTL for the leadership lease in milliseconds.
    /// Leadership is lost if not renewed within this time.
    pub lease_ttl_ms: u64,
    /// How often to renew the lease (should be < lease_ttl_ms / 2).
    pub renew_interval_ms: u64,
    /// How long to wait before retrying election after losing leadership.
    pub retry_delay_ms: u64,
    /// Maximum time to wait for initial election.
    pub election_timeout_ms: u64,
}

impl Default for ElectionConfig {
    fn default() -> Self {
        Self {
            lease_ttl_ms: 15_000,        // 15 seconds
            renew_interval_ms: 5_000,    // 5 seconds (renew 3x per lease)
            retry_delay_ms: 1_000,       // 1 second before retry
            election_timeout_ms: 30_000, // 30 seconds max wait
        }
    }
}

/// Current leadership state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeadershipState {
    /// Not currently the leader.
    Follower,
    /// Currently the leader with the given fencing token.
    Leader {
        /// Monotonically increasing token to prevent split-brain scenarios.
        fencing_token: FencingToken,
    },
    /// Transitioning (acquiring or releasing leadership).
    Transitioning,
}

impl LeadershipState {
    /// Returns true if currently the leader.
    pub fn is_leader(&self) -> bool {
        matches!(self, LeadershipState::Leader { .. })
    }

    /// Get the fencing token if leader.
    pub fn fencing_token(&self) -> Option<FencingToken> {
        match self {
            LeadershipState::Leader { fencing_token } => Some(*fencing_token),
            _ => None,
        }
    }
}

/// Leader election coordinator.
///
/// Manages leader election using a distributed lock. Only one node
/// can be leader at a time. Leadership is maintained through periodic
/// lease renewal.
///
/// # Fencing Tokens
///
/// Each leadership term has a monotonically increasing fencing token.
/// Include this token in all operations that require leader authority.
/// External services should reject operations with stale tokens.
///
/// # Example
///
/// ```ignore
/// let election = LeaderElection::new(
///     store,
///     "my-service-leader",
///     "node-1",
///     ElectionConfig::default(),
/// );
///
/// // Start participating in election
/// let handle = election.start().await?;
///
/// // Watch for leadership changes
/// let mut rx = handle.subscribe();
/// while rx.changed().await.is_ok() {
///     let state = rx.borrow().clone();
///     if state.is_leader() {
///         println!("Became leader with token {:?}", state.fencing_token());
///     }
/// }
/// ```
pub struct LeaderElection<S: KeyValueStore + ?Sized + 'static> {
    lock: DistributedLock<S>,
    config: ElectionConfig,
    candidate_id: String,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> LeaderElection<S> {
    /// Create a new leader election coordinator.
    ///
    /// # Arguments
    /// * `store` - The underlying key-value store
    /// * `election_key` - Unique key for this election (e.g., "my-service-leader")
    /// * `candidate_id` - Unique identifier for this candidate node
    /// * `config` - Election configuration
    pub fn new(
        store: Arc<S>,
        election_key: impl Into<String>,
        candidate_id: impl Into<String>,
        config: ElectionConfig,
    ) -> Self {
        let candidate_id_str: String = candidate_id.into();
        let lock_config = LockConfig {
            ttl_ms: config.lease_ttl_ms,
            acquire_timeout_ms: config.election_timeout_ms,
            initial_backoff_ms: 50,
            max_backoff_ms: config.retry_delay_ms,
        };

        Self {
            lock: DistributedLock::new(store, election_key, candidate_id_str.clone(), lock_config),
            config,
            candidate_id: candidate_id_str,
        }
    }

    /// Start participating in leader election.
    ///
    /// Returns a handle that can be used to:
    /// - Subscribe to leadership state changes
    /// - Check current leadership status
    /// - Gracefully step down
    pub async fn start(self) -> Result<ElectionHandle, CoordinationError> {
        let (state_tx, state_rx) = watch::channel(LeadershipState::Follower);
        let running = Arc::new(AtomicBool::new(true));
        let candidate_id = self.candidate_id.clone();

        // Start the election loop in a background task
        let election_task = tokio::spawn(self.election_loop(state_tx, running.clone()));

        Ok(ElectionHandle {
            state_rx,
            running,
            task: Some(election_task),
            candidate_id,
        })
    }

    /// Try to acquire leadership once without blocking.
    ///
    /// Returns the fencing token if leadership was acquired, None otherwise.
    pub async fn try_acquire_leadership(&self) -> Result<Option<FencingToken>, CoordinationError> {
        match self.lock.try_acquire().await {
            Ok(guard) => {
                let token = guard.fencing_token();
                // Note: guard is intentionally NOT dropped here - caller must manage
                // For one-shot leadership, use start() instead
                std::mem::forget(guard);
                Ok(Some(token))
            }
            Err(CoordinationError::LockHeld { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Main election loop.
    async fn election_loop(self, state_tx: watch::Sender<LeadershipState>, running: Arc<AtomicBool>) {
        while running.load(Ordering::SeqCst) {
            // Try to become leader
            let _ = state_tx.send(LeadershipState::Transitioning);

            match self.lock.try_acquire().await {
                Ok(guard) => {
                    let token = guard.fencing_token();
                    info!(
                        candidate = %self.candidate_id,
                        fencing_token = token.value(),
                        "acquired leadership"
                    );

                    let _ = state_tx.send(LeadershipState::Leader { fencing_token: token });

                    // Maintain leadership through renewal
                    self.maintain_leadership(guard, &state_tx, &running).await;

                    info!(candidate = %self.candidate_id, "lost leadership");
                    let _ = state_tx.send(LeadershipState::Follower);
                }
                Err(CoordinationError::LockHeld { holder, .. }) => {
                    debug!(
                        candidate = %self.candidate_id,
                        current_leader = %holder,
                        "election lost, current leader exists"
                    );
                    let _ = state_tx.send(LeadershipState::Follower);
                }
                Err(CoordinationError::Timeout { .. }) => {
                    debug!(candidate = %self.candidate_id, "election timeout");
                    let _ = state_tx.send(LeadershipState::Follower);
                }
                Err(e) => {
                    warn!(candidate = %self.candidate_id, error = %e, "election error");
                    let _ = state_tx.send(LeadershipState::Follower);
                }
            }

            // Wait before retrying (unless stopped)
            if running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }
        }
    }

    /// Maintain leadership by periodically renewing the lease.
    async fn maintain_leadership(
        &self,
        guard: LockGuard<S>,
        state_tx: &watch::Sender<LeadershipState>,
        running: &Arc<AtomicBool>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(self.config.renew_interval_ms));

        loop {
            interval.tick().await;

            if !running.load(Ordering::SeqCst) {
                // Graceful stepdown requested
                debug!(candidate = %self.candidate_id, "stepping down from leadership");
                if let Err(e) = guard.release().await {
                    warn!(error = %e, "error releasing leadership lock");
                }
                return;
            }

            // Try to renew the lease
            match self.lock.renew(&guard).await {
                Ok(()) => {
                    debug!(
                        candidate = %self.candidate_id,
                        fencing_token = guard.fencing_token().value(),
                        "leadership lease renewed"
                    );
                }
                Err(CoordinationError::LockLost { current_holder, .. }) => {
                    warn!(
                        candidate = %self.candidate_id,
                        new_leader = %current_holder,
                        "leadership lost to another candidate"
                    );
                    return;
                }
                Err(e) => {
                    warn!(
                        candidate = %self.candidate_id,
                        error = %e,
                        "failed to renew leadership lease"
                    );
                    // Continue trying - the lock will expire if we can't renew
                }
            }

            // Update state with current token (in case it was somehow updated)
            let _ = state_tx.send(LeadershipState::Leader {
                fencing_token: guard.fencing_token(),
            });
        }
    }
}

/// Handle for managing an ongoing election.
pub struct ElectionHandle {
    state_rx: watch::Receiver<LeadershipState>,
    running: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
    candidate_id: String,
}

impl ElectionHandle {
    /// Get the current leadership state.
    pub fn state(&self) -> LeadershipState {
        self.state_rx.borrow().clone()
    }

    /// Check if currently the leader.
    pub fn is_leader(&self) -> bool {
        self.state().is_leader()
    }

    /// Get the current fencing token (if leader).
    pub fn fencing_token(&self) -> Option<FencingToken> {
        self.state().fencing_token()
    }

    /// Subscribe to leadership state changes.
    ///
    /// The receiver will be notified whenever leadership state changes.
    pub fn subscribe(&self) -> watch::Receiver<LeadershipState> {
        self.state_rx.clone()
    }

    /// Get the candidate ID for this election participant.
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    /// Request graceful stepdown from leadership.
    ///
    /// If currently the leader, this will release the leadership lock
    /// and transition to follower state. If not the leader, this is a no-op.
    pub fn stepdown(&self) {
        if self.is_leader() {
            info!(candidate = %self.candidate_id, "stepdown requested");
            self.running.store(false, Ordering::SeqCst);
        }
    }

    /// Stop participating in the election entirely.
    ///
    /// This will stepdown if leader and stop the election loop.
    pub async fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for ElectionHandle {
    fn drop(&mut self) {
        // Signal the election loop to stop
        self.running.store(false, Ordering::SeqCst);
        // Note: we can't await the task in drop, so it will be cancelled
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_single_candidate_becomes_leader() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let election = LeaderElection::new(store, "test-election", "candidate-1", ElectionConfig {
            lease_ttl_ms: 1000,
            renew_interval_ms: 200,
            retry_delay_ms: 100,
            election_timeout_ms: 5000,
        });

        let handle = election.start().await.unwrap();

        // Wait for leadership
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(handle.is_leader());
        assert!(handle.fencing_token().is_some());

        handle.stop().await;
    }

    #[tokio::test]
    async fn test_two_candidates_one_leader() {
        let store = Arc::new(DeterministicKeyValueStore::new());

        let config = ElectionConfig {
            lease_ttl_ms: 1000,
            renew_interval_ms: 200,
            retry_delay_ms: 100,
            election_timeout_ms: 5000,
        };

        let election1 = LeaderElection::new(store.clone(), "test-election", "candidate-1", config.clone());

        let election2 = LeaderElection::new(store, "test-election", "candidate-2", config);

        let handle1 = election1.start().await.unwrap();
        let handle2 = election2.start().await.unwrap();

        // Wait for at least one to become leader
        let mut found_leader = false;
        for _ in 0..30 {
            if handle1.is_leader() || handle2.is_leader() {
                found_leader = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(found_leader, "At least one candidate should become leader");

        // Exactly one should be leader at any given time
        let leader_count = [handle1.is_leader(), handle2.is_leader()].iter().filter(|&&x| x).count();
        assert_eq!(leader_count, 1, "Exactly one candidate should be leader");

        handle1.stop().await;
        handle2.stop().await;
    }

    #[tokio::test]
    async fn test_stepdown_releases_leadership() {
        let store = Arc::new(DeterministicKeyValueStore::new());

        let election1 = LeaderElection::new(store.clone(), "test-election", "candidate-1", ElectionConfig {
            lease_ttl_ms: 1000,
            renew_interval_ms: 200,
            retry_delay_ms: 100,
            election_timeout_ms: 5000,
        });

        let handle1 = election1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(handle1.is_leader());

        // Stepdown
        handle1.stepdown();
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Should no longer be leader
        assert!(!handle1.is_leader());

        handle1.stop().await;
    }

    #[tokio::test]
    async fn test_fencing_token_increases_on_reelection() {
        let store = Arc::new(DeterministicKeyValueStore::new());

        let config = ElectionConfig {
            lease_ttl_ms: 500,
            renew_interval_ms: 100,
            retry_delay_ms: 50,
            election_timeout_ms: 5000,
        };

        // First election
        let election1 = LeaderElection::new(store.clone(), "test-election", "candidate-1", config.clone());
        let handle1 = election1.start().await.unwrap();

        // Wait for leadership with polling
        let mut token1 = None;
        for _ in 0..20 {
            if let Some(t) = handle1.fencing_token() {
                token1 = Some(t);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let token1 = token1.expect("first election should succeed");
        handle1.stop().await;

        // Second election (same candidate re-elected)
        tokio::time::sleep(Duration::from_millis(100)).await;
        let election2 = LeaderElection::new(store, "test-election", "candidate-1", config);
        let handle2 = election2.start().await.unwrap();

        // Wait for leadership with polling
        let mut token2 = None;
        for _ in 0..20 {
            if let Some(t) = handle2.fencing_token() {
                token2 = Some(t);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let token2 = token2.expect("second election should succeed");

        assert!(
            token2.value() > token1.value(),
            "Fencing token should increase on re-election: token1={}, token2={}",
            token1.value(),
            token2.value()
        );

        handle2.stop().await;
    }

    #[tokio::test]
    async fn test_leadership_state_subscription() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let election = LeaderElection::new(store, "test-election", "candidate-1", ElectionConfig {
            lease_ttl_ms: 1000,
            renew_interval_ms: 200,
            retry_delay_ms: 100,
            election_timeout_ms: 5000,
        });

        let handle = election.start().await.unwrap();
        let mut rx = handle.subscribe();

        // Wait for at least one state change
        tokio::time::timeout(Duration::from_secs(1), rx.changed())
            .await
            .expect("timeout")
            .expect("channel closed");

        // Should eventually become leader
        let mut became_leader = false;
        for _ in 0..10 {
            if rx.borrow().is_leader() {
                became_leader = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert!(became_leader, "Should have become leader");

        handle.stop().await;
    }
}
