// Message types for inter-component communication in the VM Manager
//
// This module defines all message types used for asynchronous communication
// between VM Manager components. Using message passing decouples components
// and enables better testing, clearer boundaries, and more maintainable code.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use super::health_checker::HealthStatus;
use super::vm_types::{VmConfig, VmInstance, VmMetrics, VmState};
use crate::Job;

// ============================================================================
// Commands - Requests from VmManager to components
// ============================================================================

/// Commands sent to the VmController for lifecycle operations
#[derive(Debug)]
pub enum VmCommand {
    /// Start a new VM with the given configuration
    Start {
        config: VmConfig,
        response: oneshot::Sender<Result<VmInstance>>,
    },
    /// Stop a running VM
    Stop {
        vm_id: Uuid,
        graceful: bool,
        response: oneshot::Sender<Result<()>>,
    },
    /// Send a job to a specific service VM
    SendJob {
        vm_id: Uuid,
        job: Job,
        response: oneshot::Sender<Result<()>>,
    },
    /// Restart a VM
    Restart {
        vm_id: Uuid,
        response: oneshot::Sender<Result<()>>,
    },
    /// Get status of a VM
    GetStatus {
        vm_id: Uuid,
        response: oneshot::Sender<Result<Option<VmState>>>,
    },
    /// Shutdown the controller
    Shutdown {
        response: oneshot::Sender<Result<()>>,
    },
}

/// Requests sent to the JobRouter for job assignment
#[derive(Debug)]
pub enum RouterRequest {
    /// Route a job to the best available VM
    RouteJob {
        job: Job,
        response: oneshot::Sender<Result<VmAssignment>>,
    },
    /// Get an available service VM
    GetAvailableVm {
        response: oneshot::Sender<Option<Uuid>>,
    },
    /// Stop accepting new jobs (for graceful shutdown)
    StopRouting {
        response: oneshot::Sender<()>,
    },
    /// Get routing statistics
    GetStats {
        response: oneshot::Sender<RouteStats>,
    },
}

/// Commands for the ResourceMonitor
#[derive(Debug)]
pub enum MonitorCommand {
    /// Start monitoring loop
    Start,
    /// Stop monitoring
    Stop,
    /// Force resource check now
    CheckNow {
        response: oneshot::Sender<Result<()>>,
    },
}

/// Commands for the HealthChecker
#[derive(Debug)]
pub enum HealthCommand {
    /// Start health checking loop
    Start,
    /// Stop health checking
    Stop,
    /// Force health check on specific VM
    ForceCheck {
        vm_id: Uuid,
        response: oneshot::Sender<Result<()>>,
    },
    /// Get health status of a VM
    GetStatus {
        vm_id: Uuid,
        response: oneshot::Sender<HealthStatus>,
    },
    /// Reset health status for a VM
    ResetHealth {
        vm_id: Uuid,
    },
}

// ============================================================================
// Events - Notifications emitted by components
// ============================================================================

/// Events emitted by VmController about VM lifecycle changes
#[derive(Debug, Clone)]
pub enum VmEvent {
    /// VM has started successfully
    Started {
        vm_id: Uuid,
        config: VmConfig,
    },
    /// VM has stopped
    Stopped {
        vm_id: Uuid,
        reason: String,
    },
    /// VM failed to start or crashed
    Failed {
        vm_id: Uuid,
        error: String,
    },
    /// Job was assigned to a VM
    JobAssigned {
        vm_id: Uuid,
        job_id: String,
    },
    /// Job completed on a VM
    JobCompleted {
        vm_id: Uuid,
        job_id: String,
        success: bool,
    },
    /// VM state changed
    StateChanged {
        vm_id: Uuid,
        old_state: VmState,
        new_state: VmState,
    },
}

/// Events emitted by ResourceMonitor about resource usage
#[derive(Debug, Clone)]
pub enum MonitoringEvent {
    /// Resource threshold exceeded (memory, CPU, etc.)
    ResourceThreshold {
        vm_id: Uuid,
        resource: ResourceType,
        current: u64,
        threshold: u64,
    },
    /// VM should be recycled due to limits
    VmShouldRecycle {
        vm_id: Uuid,
        reason: RecycleReason,
    },
    /// Idle VM detected
    IdleVmDetected {
        vm_id: Uuid,
        idle_duration_secs: i64,
    },
    /// Auto-scaling triggered
    AutoScalingTriggered {
        action: ScalingAction,
        count: usize,
    },
}

/// Events emitted by HealthChecker about VM health
#[derive(Debug, Clone)]
pub enum HealthEvent {
    /// VM health check passed
    HealthCheckPassed {
        vm_id: Uuid,
        response_time_ms: u64,
    },
    /// VM health check failed
    HealthCheckFailed {
        vm_id: Uuid,
        error: String,
        failures: u32,
    },
    /// VM marked as unhealthy
    VmUnhealthy {
        vm_id: Uuid,
        failures: u32,
        error: String,
    },
    /// VM recovered to healthy state
    VmRecovered {
        vm_id: Uuid,
    },
    /// Circuit breaker opened for VM
    CircuitBreakerOpened {
        vm_id: Uuid,
        duration_secs: u64,
    },
}

// ============================================================================
// Supporting types
// ============================================================================

/// Result of job routing
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

/// Types of resources that can be monitored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Memory,
    Cpu,
    Disk,
    Network,
}

/// Reasons why a VM should be recycled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecycleReason {
    JobLimitReached { limit: u32, completed: u32 },
    UptimeLimitReached { limit_secs: u64, uptime_secs: i64 },
    MemoryPressure { current_mb: u32, limit_mb: u32 },
    HealthCheckFailures { failures: u32 },
}

/// Auto-scaling actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
}

/// Routing statistics
#[derive(Debug, Clone, Serialize)]
pub struct RouteStats {
    pub total_vms: usize,
    pub ephemeral_vms: usize,
    pub service_vms: usize,
    pub available_service_vms: usize,
}

// ============================================================================
// Registry notifications (for event sourcing pattern)
// ============================================================================

/// Events emitted by VmRegistry when state changes
#[derive(Debug, Clone)]
pub enum RegistryEvent {
    /// VM was registered
    VmRegistered {
        vm_id: Uuid,
        config: VmConfig,
    },
    /// VM state was updated
    VmStateUpdated {
        vm_id: Uuid,
        old_state: VmState,
        new_state: VmState,
    },
    /// VM metrics were updated
    VmMetricsUpdated {
        vm_id: Uuid,
        metrics: VmMetrics,
    },
    /// VM was removed
    VmRemoved {
        vm_id: Uuid,
    },
}
