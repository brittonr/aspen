//! VM domain types
//!
//! These types represent VM concepts at the domain level.
//! They are independent from infrastructure implementation details,
//! following the Dependency Inversion Principle.
//!
//! The domain owns these types, and infrastructure adapters map
//! to/from these domain types.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// VM Configuration and Mode
// ============================================================================

/// VM execution mode (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VmMode {
    /// VM executes single job then terminates
    Ephemeral {
        job_id: String,
    },
    /// VM stays alive processing multiple jobs from a queue
    Service {
        queue_name: String,
        max_jobs: Option<u32>,
        max_uptime_secs: Option<u64>,
    },
}

/// Security isolation level (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsolationLevel {
    /// Highest isolation - new VM per job (untrusted code)
    Maximum,
    /// Standard isolation - shared VM with process isolation
    Standard,
    /// Low isolation - for trusted internal workloads
    Minimal,
}

/// VM configuration (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub id: Uuid,
    pub mode: VmMode,
    pub memory_mb: u32,
    pub vcpus: u32,
    pub hypervisor: String,
    pub capabilities: Vec<String>,
    pub isolation_level: IsolationLevel,
}

// ============================================================================
// VM State and Instance
// ============================================================================

/// Current VM state (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VmState {
    /// VM is being created
    Starting,
    /// VM is ready to accept jobs
    Ready,
    /// VM is processing a job (ephemeral or service mode)
    Busy {
        job_id: String,
        started_at: i64,
    },
    /// VM is idle waiting for jobs (service mode only)
    Idle {
        jobs_completed: u32,
        last_job_at: i64,
    },
    /// VM is shutting down gracefully
    Draining,
    /// VM has terminated normally
    Terminated {
        reason: String,
        exit_code: i32,
    },
    /// VM failed or crashed
    Failed {
        error: String,
    },
}

impl VmState {
    /// Check if VM is in a running state
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            VmState::Ready | VmState::Busy { .. } | VmState::Idle { .. }
        )
    }

    /// Check if VM is available for jobs
    pub fn is_available(&self) -> bool {
        matches!(self, VmState::Ready | VmState::Idle { .. })
    }

    /// Check if VM is in terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, VmState::Terminated { .. } | VmState::Failed { .. })
    }

    /// Check if VM is stopped (terminated or draining)
    pub fn is_stopped(&self) -> bool {
        matches!(self, VmState::Terminated { .. } | VmState::Draining)
    }

    /// Check if VM has failed
    pub fn is_failed(&self) -> bool {
        matches!(self, VmState::Failed { .. })
    }
}

/// VM instance (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInstance {
    pub config: Arc<VmConfig>,
    pub state: VmState,
    pub created_at: i64,
    pub pid: Option<u32>,
    pub ip_address: Option<String>,
}

// ============================================================================
// Job Requirements and Results
// ============================================================================

/// Job requirements for VM selection (domain representation)
#[derive(Debug, Clone)]
pub struct JobRequirements {
    pub memory_mb: u32,
    pub vcpus: u32,
    pub capabilities: Vec<String>,
    pub isolation_level: IsolationLevel,
    pub timeout_secs: Option<u64>,
    pub gpu_required: bool,
}

impl Default for JobRequirements {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            vcpus: 1,
            capabilities: vec![],
            isolation_level: IsolationLevel::Standard,
            timeout_secs: Some(300), // 5 minutes default
            gpu_required: false,
        }
    }
}

/// Result from job execution (domain representation)
#[derive(Debug, Clone)]
pub struct JobResult {
    pub vm_id: Uuid,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// VM assignment result (domain representation)
#[derive(Debug, Clone)]
pub enum VmAssignment {
    /// Job assigned to ephemeral VM (one job then terminate)
    Ephemeral(Uuid),
    /// Job assigned to service VM (long-running)
    Service(Uuid),
}

impl VmAssignment {
    /// Get the VM ID from the assignment
    pub fn vm_id(&self) -> Uuid {
        match self {
            VmAssignment::Ephemeral(id) => *id,
            VmAssignment::Service(id) => *id,
        }
    }
}

// ============================================================================
// VM Statistics and Monitoring
// ============================================================================

/// VM fleet statistics (domain representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStats {
    pub total_vms: usize,
    pub running_vms: usize,
    pub idle_vms: usize,
    pub failed_vms: usize,
}

/// Health status of a VM (domain representation)
///
/// Represents the current health state with relevant metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// VM health unknown (not yet checked)
    Unknown,

    /// VM is healthy and responding
    Healthy {
        last_check: i64,
        response_time_ms: u64,
    },

    /// VM is degraded but still functioning
    Degraded {
        last_check: i64,
        failures: u32,
        last_error: String,
    },

    /// VM is not responding (circuit open)
    Unhealthy {
        since: i64,
        failures: u32,
        last_error: String,
    },
}

impl HealthStatus {
    /// Check if the VM is in a running state (healthy or degraded)
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            HealthStatus::Healthy { .. } | HealthStatus::Degraded { .. }
        )
    }

    /// Check if the VM is specifically healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy { .. })
    }

    /// Check if the VM is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, HealthStatus::Degraded { .. })
    }

    /// Check if the VM is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthStatus::Unhealthy { .. })
    }
}
