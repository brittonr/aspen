// VM Types and State Definitions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// VM execution mode
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

/// VM configuration
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

impl VmConfig {
    /// Create default ephemeral VM config
    pub fn default_ephemeral(job_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            mode: VmMode::Ephemeral { job_id },
            memory_mb: 512,
            vcpus: 1,
            hypervisor: "firecracker".to_string(),
            capabilities: vec![],
            isolation_level: IsolationLevel::Maximum,
        }
    }

    /// Create default service VM config
    pub fn default_service() -> Self {
        Self {
            id: Uuid::new_v4(),
            mode: VmMode::Service {
                queue_name: "default".to_string(),
                max_jobs: Some(50),
                max_uptime_secs: Some(3600), // 1 hour
            },
            memory_mb: 1024,
            vcpus: 2,
            hypervisor: "qemu".to_string(),
            capabilities: vec![],
            isolation_level: IsolationLevel::Standard,
        }
    }
}

/// Security isolation level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsolationLevel {
    /// Highest isolation - new VM per job (untrusted code)
    Maximum,
    /// Standard isolation - shared VM with process isolation
    Standard,
    /// Low isolation - for trusted internal workloads
    Minimal,
}

/// Current VM state
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
}

/// VM instance runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInstance {
    pub config: VmConfig,
    pub state: VmState,
    pub created_at: i64,
    pub pid: Option<u32>,
    pub control_socket: Option<PathBuf>,
    pub job_dir: Option<PathBuf>,
    pub ip_address: Option<String>,
    pub metrics: VmMetrics,
}

impl VmInstance {
    /// Create new VM instance
    pub fn new(config: VmConfig) -> Self {
        Self {
            config,
            state: VmState::Starting,
            created_at: chrono::Utc::now().timestamp(),
            pid: None,
            control_socket: None,
            job_dir: None,
            ip_address: None,
            metrics: VmMetrics::default(),
        }
    }

    /// Check if VM can accept a job
    pub fn can_accept_job(&self) -> bool {
        self.state.is_available() && !matches!(self.state, VmState::Draining)
    }

    /// Get VM uptime in seconds
    pub fn uptime_secs(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.created_at
    }

    /// Check if VM should be recycled based on limits
    pub fn should_recycle(&self) -> bool {
        match &self.config.mode {
            VmMode::Service {
                max_jobs,
                max_uptime_secs,
                ..
            } => {
                // Check job limit
                if let Some(limit) = max_jobs {
                    if self.metrics.jobs_completed >= *limit {
                        return true;
                    }
                }

                // Check uptime limit
                if let Some(limit) = max_uptime_secs {
                    if self.uptime_secs() >= *limit as i64 {
                        return true;
                    }
                }

                // Check memory pressure
                if self.metrics.current_memory_mb > (self.config.memory_mb as f32 * 0.9) as u32 {
                    return true;
                }

                false
            }
            VmMode::Ephemeral { .. } => false, // Ephemeral VMs terminate after one job
        }
    }
}

/// VM performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmMetrics {
    pub jobs_completed: u32,
    pub jobs_failed: u32,
    pub total_cpu_time_ms: u64,
    pub current_memory_mb: u32,
    pub peak_memory_mb: u32,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub last_health_check: i64,
    pub health_check_failures: u32,
}

impl VmMetrics {
    /// Record job completion
    pub fn record_job_completion(&mut self, success: bool, cpu_time_ms: u64) {
        if success {
            self.jobs_completed += 1;
        } else {
            self.jobs_failed += 1;
        }
        self.total_cpu_time_ms += cpu_time_ms;
    }

    /// Update memory metrics
    pub fn update_memory(&mut self, current_mb: u32) {
        self.current_memory_mb = current_mb;
        if current_mb > self.peak_memory_mb {
            self.peak_memory_mb = current_mb;
        }
    }

    /// Record health check result
    pub fn record_health_check(&mut self, success: bool) {
        self.last_health_check = chrono::Utc::now().timestamp();
        if success {
            self.health_check_failures = 0;
        } else {
            self.health_check_failures += 1;
        }
    }
}

/// Job requirements for VM selection
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

/// Control messages for VM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VmControlMessage {
    /// Execute a job
    ExecuteJob {
        job: serde_json::Value,
    },
    /// Ping for health check
    Ping,
    /// Pong response
    Pong {
        uptime_secs: i64,
        jobs_completed: u32,
    },
    /// Get VM status
    GetStatus,
    /// Status response
    Status {
        state: String,
        metrics: VmMetrics,
    },
    /// Graceful shutdown
    Shutdown {
        timeout_secs: u32,
    },
    /// Acknowledge message received
    Ack,
    /// Error response
    Error {
        message: String,
    },
}

/// VM lifecycle events for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmEvent {
    pub vm_id: Uuid,
    pub timestamp: i64,
    pub event_type: VmEventType,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VmEventType {
    Created,
    Started,
    JobAssigned,
    JobCompleted,
    JobFailed,
    HealthCheckPassed,
    HealthCheckFailed,
    Recycling,
    Terminating,
    Terminated,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_state_transitions() {
        let state = VmState::Starting;
        assert!(!state.is_running());
        assert!(!state.is_available());

        let state = VmState::Ready;
        assert!(state.is_running());
        assert!(state.is_available());

        let state = VmState::Busy {
            job_id: "test-123".to_string(),
            started_at: 0,
        };
        assert!(state.is_running());
        assert!(!state.is_available());

        let state = VmState::Failed {
            error: "test error".to_string(),
        };
        assert!(state.is_terminal());
    }

    #[test]
    fn test_vm_recycling_logic() {
        let mut config = VmConfig::default_service();
        config.mode = VmMode::Service {
            queue_name: "test".to_string(),
            max_jobs: Some(5),
            max_uptime_secs: Some(60),
        };

        let mut vm = VmInstance::new(config);
        assert!(!vm.should_recycle());

        // Test job limit
        vm.metrics.jobs_completed = 5;
        assert!(vm.should_recycle());

        // Test memory pressure
        vm.metrics.jobs_completed = 3;
        vm.config.memory_mb = 100;
        vm.metrics.current_memory_mb = 95;
        assert!(vm.should_recycle());
    }
}