// VM Recovery Service - Startup recovery and self-healing
//
// Responsible for recovering VM state from persistence on startup,
// detecting dead processes, and migrating legacy data formats.

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::infrastructure::vm::vm_types::{VmInstance, VmState};

use super::cache::VmCache;
use super::index::VmStateIndex;
use super::persistence::VmPersistence;

pub struct VmRecovery<P, C, I>
where
    P: VmPersistence,
    C: VmCache,
    I: VmStateIndex,
{
    persistence: Arc<P>,
    cache: Arc<C>,
    index: Arc<I>,
}

impl<P, C, I> VmRecovery<P, C, I>
where
    P: VmPersistence,
    C: VmCache,
    I: VmStateIndex,
{
    pub fn new(persistence: Arc<P>, cache: Arc<C>, index: Arc<I>) -> Self {
        Self {
            persistence,
            cache,
            index,
        }
    }

    /// Recover all VMs from persistence
    pub async fn recover(&self) -> Result<usize> {
        let vms = self.persistence.load_all().await?;
        let mut recovered = 0;
        let mut healed = 0;

        for mut vm in vms {
            let vm_id = vm.config.id;

            // Self-healing: Check if process is still alive
            if let Some(pid) = vm.pid {
                if vm.state.is_running() && !is_process_alive(pid) {
                    tracing::warn!(
                        vm_id = %vm_id,
                        pid = pid,
                        "Process died while system was down - marking as failed"
                    );

                    vm.state = VmState::Failed {
                        error: "Process died while system was down".to_string(),
                    };

                    // Update in persistence
                    if let Err(e) = self.persistence.update_state(&vm_id, &vm.state).await {
                        tracing::error!(
                            vm_id = %vm_id,
                            error = %e,
                            "Failed to update VM state after detecting dead process"
                        );
                    } else {
                        healed += 1;
                    }
                }
            }

            // Add to cache
            self.cache.insert(vm.clone());

            // Rebuild index
            self.index.add(vm_id, &vm.state);

            recovered += 1;
        }

        tracing::info!(
            recovered,
            healed,
            "VMs recovered from persistence with self-healing"
        );

        Ok(recovered)
    }
}

/// Check if a process is still alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid as i32), Signal::SIGCONT) {
            Ok(_) => true,
            Err(nix::errno::Errno::ESRCH) => false,
            Err(_) => true, // Assume alive if we can't determine
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix platforms, assume process is alive
        let _ = pid;
        true
    }
}
