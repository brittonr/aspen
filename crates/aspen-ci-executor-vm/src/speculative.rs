//! Speculative execution via parallel VM forks.
//!
//! Forks multiple VMs from the same golden snapshot, each with independent
//! workspace state (KV branch). The first fork to complete successfully
//! commits its results; all others are killed and their branches dropped.

use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::vm::SharedVm;

/// Maximum number of speculative forks per job (hard cap).
pub const MAX_SPECULATIVE_FORKS: u32 = 8;

/// A group of speculative VM forks running the same job.
///
/// Each fork has its own VM and workspace isolation (via KV branch prefix).
/// The group monitors all forks and implements first-success-wins semantics.
pub struct SpeculativeGroup {
    /// The forked VMs in this group.
    forks: Vec<SharedVm>,

    /// Job ID this group belongs to.
    job_id: String,
}

impl SpeculativeGroup {
    /// Create a new speculative group from a set of VMs.
    pub fn new(forks: Vec<SharedVm>, job_id: String) -> Self {
        info!(
            job_id = %job_id,
            fork_count = forks.len(),
            "created speculative group"
        );
        Self { forks, job_id }
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
        assert!(MAX_SPECULATIVE_FORKS > 0);
        assert!(MAX_SPECULATIVE_FORKS <= 16, "speculative forks should not exceed 16");
    }
}
