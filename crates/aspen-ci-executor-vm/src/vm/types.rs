//! VM state types and shared references.

use std::sync::Arc;

use super::ManagedCiVm;

/// State of a managed CI VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// VM process is starting up.
    Creating,
    /// VM is booted, waiting for guest agent.
    Booting,
    /// VM is ready for job assignment.
    Idle,
    /// Job has been assigned, preparing workspace.
    Assigned,
    /// Job is executing in the VM.
    Running,
    /// Job completed, cleaning up workspace.
    Cleanup,
    /// VM is paused (for snapshot).
    Paused,
    /// VM has been shut down.
    Stopped,
    /// VM is in an unrecoverable error state.
    Error,
}

impl std::fmt::Display for VmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmState::Creating => write!(f, "Creating"),
            VmState::Booting => write!(f, "Booting"),
            VmState::Idle => write!(f, "Idle"),
            VmState::Assigned => write!(f, "Assigned"),
            VmState::Running => write!(f, "Running"),
            VmState::Cleanup => write!(f, "Cleanup"),
            VmState::Paused => write!(f, "Paused"),
            VmState::Stopped => write!(f, "Stopped"),
            VmState::Error => write!(f, "Error"),
        }
    }
}

/// Shared reference to a managed VM.
pub type SharedVm = Arc<ManagedCiVm>;
