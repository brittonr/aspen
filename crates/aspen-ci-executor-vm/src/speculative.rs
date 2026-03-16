//! Speculative execution via parallel VM forks.
//!
//! Forks multiple VMs from the same golden snapshot, each with independent
//! workspace state (KV branch). The first fork to complete successfully
//! commits its results; all others are killed and their branches dropped.

use tokio::sync::mpsc;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::vm::SharedVm;

/// Maximum number of speculative forks per job (hard cap).
pub const MAX_SPECULATIVE_FORKS: u32 = 8;

/// Result of a single speculative fork.
#[derive(Debug)]
pub struct ForkResult {
    /// Index of the fork in the group (0-based).
    pub fork_index: u32,
    /// VM ID of the fork.
    pub vm_id: String,
    /// Whether the fork succeeded.
    pub success: bool,
    /// Optional error message on failure.
    pub error: Option<String>,
}

/// Outcome of `wait_first_success()`.
#[derive(Debug)]
pub enum SpeculativeOutcome {
    /// A fork succeeded. Contains the winner's index and VM ID.
    Winner {
        /// Index of the winning fork.
        fork_index: u32,
        /// VM ID of the winning fork.
        vm_id: String,
    },
    /// All forks failed. Contains all failure messages.
    AllFailed {
        /// Error messages from each fork.
        errors: Vec<String>,
    },
}

/// A group of speculative VM forks running the same job.
///
/// Each fork has its own VM and workspace isolation (via KV branch prefix).
/// The group monitors all forks and implements first-success-wins semantics.
///
/// # Usage
///
/// 1. Create the group via `VmPool::acquire_speculative()`
/// 2. Clone `result_tx()` for each fork's execution task
/// 3. Spawn job execution on each fork, send `ForkResult` on completion
/// 4. Call `wait_first_success()` to race forks and get the winner
/// 5. Call `cleanup()` to destroy all forks
pub struct SpeculativeGroup {
    /// The forked VMs in this group.
    forks: Vec<SharedVm>,

    /// Job ID this group belongs to.
    job_id: String,

    /// Channel sender for fork results. Taken (dropped) in `wait_first_success()`
    /// so the channel closes when all external senders finish.
    result_tx: Option<mpsc::Sender<ForkResult>>,

    /// Receiver for fork completion signals.
    result_rx: mpsc::Receiver<ForkResult>,
}

impl SpeculativeGroup {
    /// Create a new speculative group from a set of VMs.
    pub fn new(forks: Vec<SharedVm>, job_id: String) -> Self {
        let (result_tx, result_rx) = mpsc::channel(forks.len().max(1));

        info!(
            job_id = %job_id,
            fork_count = forks.len(),
            "created speculative group"
        );
        Self {
            forks,
            job_id,
            result_tx: Some(result_tx),
            result_rx,
        }
    }

    /// Get the VMs in this group.
    pub fn forks(&self) -> &[SharedVm] {
        &self.forks
    }

    /// Get the number of forks in the group.
    pub fn fork_count(&self) -> u32 {
        self.forks.len() as u32
    }

    /// Get the job ID.
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// Get a sender for reporting fork results.
    ///
    /// Clone this for each fork's execution task. Send a `ForkResult`
    /// when the fork completes (success or failure). Drop the sender
    /// when done — `wait_first_success()` terminates when all senders close.
    pub fn result_tx(&self) -> mpsc::Sender<ForkResult> {
        self.result_tx.as_ref().expect("result_tx already consumed by wait_first_success").clone()
    }

    /// Wait for the first successful fork, or all forks to fail.
    ///
    /// Blocks until either:
    /// - A fork reports success → kills losers, returns `SpeculativeOutcome::Winner`
    /// - All senders drop (all forks done) with no success → returns `AllFailed`
    ///
    /// The winner's VM is NOT cleaned up here — the caller should commit its
    /// KV branch, then call `cleanup()` to destroy everything.
    pub async fn wait_first_success(&mut self) -> SpeculativeOutcome {
        // Drop our internal sender so the channel closes when all fork
        // senders finish. Without this, recv() would never return None.
        drop(self.result_tx.take());

        let mut errors = Vec::new();

        info!(
            job_id = %self.job_id,
            fork_count = self.forks.len(),
            "waiting for first successful fork"
        );

        while let Some(result) = self.result_rx.recv().await {
            if result.success {
                info!(
                    job_id = %self.job_id,
                    winner_vm = %result.vm_id,
                    winner_index = result.fork_index,
                    failures_so_far = errors.len(),
                    "speculative fork succeeded — killing losers"
                );

                // Kill all non-winner forks
                for (i, vm) in self.forks.iter().enumerate() {
                    if i as u32 != result.fork_index {
                        debug!(
                            vm_id = %vm.id,
                            job_id = %self.job_id,
                            "destroying losing speculative fork"
                        );
                        vm.full_fork_cleanup().await;
                        if let Err(e) = vm.shutdown().await {
                            warn!(vm_id = %vm.id, error = %e, "error shutting down losing fork");
                        }
                    }
                }

                return SpeculativeOutcome::Winner {
                    fork_index: result.fork_index,
                    vm_id: result.vm_id,
                };
            }

            // Fork failed
            let err_msg = result.error.unwrap_or_else(|| "unknown error".to_string());
            warn!(
                job_id = %self.job_id,
                vm_id = %result.vm_id,
                fork_index = result.fork_index,
                error = %err_msg,
                "speculative fork failed"
            );
            errors.push(format!("fork {} ({}): {}", result.fork_index, result.vm_id, err_msg));
        }

        // Channel closed — all senders dropped, no success
        warn!(
            job_id = %self.job_id,
            failures = errors.len(),
            "all speculative forks failed"
        );
        SpeculativeOutcome::AllFailed { errors }
    }

    /// Clean up all forks in the group.
    ///
    /// Kills all VMs (including any winner), releases resources.
    /// Called after the speculative race completes.
    pub async fn cleanup(&self) {
        info!(
            job_id = %self.job_id,
            fork_count = self.forks.len(),
            "cleaning up speculative group"
        );

        for vm in &self.forks {
            debug!(vm_id = %vm.id, job_id = %self.job_id, "destroying speculative fork");
            vm.full_fork_cleanup().await;
            if let Err(e) = vm.shutdown().await {
                warn!(vm_id = %vm.id, error = %e, "error shutting down speculative fork");
            }
        }

        info!(job_id = %self.job_id, "speculative group cleanup complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_speculative_forks_bounded() {
        const { assert!(MAX_SPECULATIVE_FORKS > 0) };
        const { assert!(MAX_SPECULATIVE_FORKS <= 16) };
    }

    #[tokio::test]
    async fn test_wait_first_success_single_winner() {
        let mut group = SpeculativeGroup::new(vec![], "test-job".to_string());
        let tx = group.result_tx();

        tokio::spawn(async move {
            // Fork 0 fails
            tx.send(ForkResult {
                fork_index: 0,
                vm_id: "vm-0".to_string(),
                success: false,
                error: Some("build failed".to_string()),
            })
            .await
            .unwrap();

            // Fork 1 succeeds
            tx.send(ForkResult {
                fork_index: 1,
                vm_id: "vm-1".to_string(),
                success: true,
                error: None,
            })
            .await
            .unwrap();
        });

        match group.wait_first_success().await {
            SpeculativeOutcome::Winner { fork_index, vm_id } => {
                assert_eq!(fork_index, 1);
                assert_eq!(vm_id, "vm-1");
            }
            SpeculativeOutcome::AllFailed { .. } => panic!("expected a winner"),
        }
    }

    #[tokio::test]
    async fn test_wait_first_success_all_failed() {
        let mut group = SpeculativeGroup::new(vec![], "test-job".to_string());
        let tx = group.result_tx();

        tokio::spawn(async move {
            tx.send(ForkResult {
                fork_index: 0,
                vm_id: "vm-0".to_string(),
                success: false,
                error: Some("error A".to_string()),
            })
            .await
            .unwrap();

            tx.send(ForkResult {
                fork_index: 1,
                vm_id: "vm-1".to_string(),
                success: false,
                error: Some("error B".to_string()),
            })
            .await
            .unwrap();

            // Sender drops here → channel closes → recv returns None
        });

        match group.wait_first_success().await {
            SpeculativeOutcome::AllFailed { errors } => {
                assert_eq!(errors.len(), 2);
                assert!(errors[0].contains("error A"));
                assert!(errors[1].contains("error B"));
            }
            SpeculativeOutcome::Winner { .. } => panic!("expected all failed"),
        }
    }

    #[tokio::test]
    async fn test_wait_first_success_immediate_winner() {
        let mut group = SpeculativeGroup::new(vec![], "test-job".to_string());
        let tx = group.result_tx();

        tokio::spawn(async move {
            tx.send(ForkResult {
                fork_index: 0,
                vm_id: "vm-0".to_string(),
                success: true,
                error: None,
            })
            .await
            .unwrap();
        });

        match group.wait_first_success().await {
            SpeculativeOutcome::Winner { fork_index, vm_id } => {
                assert_eq!(fork_index, 0);
                assert_eq!(vm_id, "vm-0");
            }
            SpeculativeOutcome::AllFailed { .. } => panic!("expected a winner"),
        }
    }

    #[tokio::test]
    async fn test_wait_first_success_channel_closed_empty() {
        let mut group = SpeculativeGroup::new(vec![], "test-job".to_string());
        // Don't clone any external senders — channel closes immediately
        // when wait_first_success drops the internal sender.

        match group.wait_first_success().await {
            SpeculativeOutcome::AllFailed { errors } => {
                assert!(errors.is_empty(), "no results sent means no errors");
            }
            SpeculativeOutcome::Winner { .. } => panic!("expected all failed"),
        }
    }

    #[tokio::test]
    async fn test_wait_first_success_winner_among_many_failures() {
        let mut group = SpeculativeGroup::new(vec![], "test-job".to_string());
        let tx = group.result_tx();

        tokio::spawn(async move {
            // 4 failures, then a success
            for i in 0..4 {
                tx.send(ForkResult {
                    fork_index: i,
                    vm_id: format!("vm-{i}"),
                    success: false,
                    error: Some(format!("error {i}")),
                })
                .await
                .unwrap();
            }

            tx.send(ForkResult {
                fork_index: 4,
                vm_id: "vm-4".to_string(),
                success: true,
                error: None,
            })
            .await
            .unwrap();
        });

        match group.wait_first_success().await {
            SpeculativeOutcome::Winner { fork_index, vm_id } => {
                assert_eq!(fork_index, 4);
                assert_eq!(vm_id, "vm-4");
            }
            SpeculativeOutcome::AllFailed { .. } => panic!("expected a winner"),
        }
    }

    #[test]
    fn test_fork_result_debug() {
        let result = ForkResult {
            fork_index: 0,
            vm_id: "vm-0".to_string(),
            success: true,
            error: None,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ForkResult"));
    }

    #[tokio::test]
    async fn test_speculative_group_three_forks_one_winner() {
        // 3 forks: first two fail, third succeeds
        let mut group = SpeculativeGroup::new(vec![], "job-3fork".to_string());
        let tx = group.result_tx();

        tokio::spawn(async move {
            // Fork 0 fails
            tx.send(ForkResult {
                fork_index: 0,
                vm_id: "vm-0".to_string(),
                success: false,
                error: Some("compile error".to_string()),
            })
            .await
            .unwrap();

            // Fork 1 fails
            tx.send(ForkResult {
                fork_index: 1,
                vm_id: "vm-1".to_string(),
                success: false,
                error: Some("link error".to_string()),
            })
            .await
            .unwrap();

            // Fork 2 succeeds
            tx.send(ForkResult {
                fork_index: 2,
                vm_id: "vm-2".to_string(),
                success: true,
                error: None,
            })
            .await
            .unwrap();
        });

        match group.wait_first_success().await {
            SpeculativeOutcome::Winner { fork_index, vm_id } => {
                assert_eq!(fork_index, 2, "third fork should win");
                assert_eq!(vm_id, "vm-2");
            }
            SpeculativeOutcome::AllFailed { errors } => {
                panic!("expected winner, got {} failures: {:?}", errors.len(), errors);
            }
        }
    }

    #[test]
    fn test_adaptive_fork_count_warning_pressure() {
        // At Warning pressure, fork count should be halved
        use crate::verified::snapshot::PRESSURE_WARNING;
        use crate::verified::snapshot::compute_adaptive_fork_count;

        assert_eq!(compute_adaptive_fork_count(4, MAX_SPECULATIVE_FORKS, PRESSURE_WARNING), 2);
        assert_eq!(compute_adaptive_fork_count(6, MAX_SPECULATIVE_FORKS, PRESSURE_WARNING), 3);
        assert_eq!(compute_adaptive_fork_count(1, MAX_SPECULATIVE_FORKS, PRESSURE_WARNING), 1);
    }

    #[test]
    fn test_adaptive_fork_count_critical_pressure() {
        // At Critical pressure, always 1 (no speculation)
        use crate::verified::snapshot::PRESSURE_CRITICAL;
        use crate::verified::snapshot::compute_adaptive_fork_count;

        assert_eq!(compute_adaptive_fork_count(4, MAX_SPECULATIVE_FORKS, PRESSURE_CRITICAL), 1);
        assert_eq!(compute_adaptive_fork_count(8, MAX_SPECULATIVE_FORKS, PRESSURE_CRITICAL), 1);
        assert_eq!(compute_adaptive_fork_count(1, MAX_SPECULATIVE_FORKS, PRESSURE_CRITICAL), 1);
    }

    #[test]
    fn test_speculative_outcome_debug() {
        let outcome = SpeculativeOutcome::Winner {
            fork_index: 0,
            vm_id: "vm-0".to_string(),
        };
        let debug_str = format!("{:?}", outcome);
        assert!(debug_str.contains("Winner"));

        let outcome = SpeculativeOutcome::AllFailed {
            errors: vec!["err".to_string()],
        };
        let debug_str = format!("{:?}", outcome);
        assert!(debug_str.contains("AllFailed"));
    }
}
