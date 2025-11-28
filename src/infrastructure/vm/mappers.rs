//! Type Mappers between Domain and Infrastructure Layers
//!
//! This module provides conversion functions between domain types (owned by the domain layer)
//! and infrastructure types (implementation details).
//!
//! Following the Dependency Inversion Principle:
//! - Domain defines its own types
//! - Infrastructure implements traits using its own types
//! - Mappers bridge the gap at the boundary

use std::sync::Arc;

// Infrastructure types
use super::health_checker::types as infra_health;
use super::job_router as infra_router;
use super::vm_types as infra_vm;

// Domain types
use crate::domain::vm::types as domain;

// ============================================================================
// VM Configuration and Mode Mapping
// ============================================================================

/// Convert domain VmMode to infrastructure VmMode
pub fn vm_mode_to_infra(mode: &domain::VmMode) -> infra_vm::VmMode {
    match mode {
        domain::VmMode::Ephemeral { job_id } => infra_vm::VmMode::Ephemeral {
            job_id: job_id.clone(),
        },
        domain::VmMode::Service {
            queue_name,
            max_jobs,
            max_uptime_secs,
        } => infra_vm::VmMode::Service {
            queue_name: queue_name.clone(),
            max_jobs: *max_jobs,
            max_uptime_secs: *max_uptime_secs,
        },
    }
}

/// Convert infrastructure VmMode to domain VmMode
pub fn vm_mode_from_infra(mode: &infra_vm::VmMode) -> domain::VmMode {
    match mode {
        infra_vm::VmMode::Ephemeral { job_id } => domain::VmMode::Ephemeral {
            job_id: job_id.clone(),
        },
        infra_vm::VmMode::Service {
            queue_name,
            max_jobs,
            max_uptime_secs,
        } => domain::VmMode::Service {
            queue_name: queue_name.clone(),
            max_jobs: *max_jobs,
            max_uptime_secs: *max_uptime_secs,
        },
    }
}

/// Convert domain IsolationLevel to infrastructure IsolationLevel
pub fn isolation_level_to_infra(level: &domain::IsolationLevel) -> infra_vm::IsolationLevel {
    match level {
        domain::IsolationLevel::Maximum => infra_vm::IsolationLevel::Maximum,
        domain::IsolationLevel::Standard => infra_vm::IsolationLevel::Standard,
        domain::IsolationLevel::Minimal => infra_vm::IsolationLevel::Minimal,
    }
}

/// Convert infrastructure IsolationLevel to domain IsolationLevel
pub fn isolation_level_from_infra(level: &infra_vm::IsolationLevel) -> domain::IsolationLevel {
    match level {
        infra_vm::IsolationLevel::Maximum => domain::IsolationLevel::Maximum,
        infra_vm::IsolationLevel::Standard => domain::IsolationLevel::Standard,
        infra_vm::IsolationLevel::Minimal => domain::IsolationLevel::Minimal,
    }
}

/// Convert domain VmConfig to infrastructure VmConfig
pub fn vm_config_to_infra(config: &domain::VmConfig) -> infra_vm::VmConfig {
    infra_vm::VmConfig {
        id: config.id,
        mode: vm_mode_to_infra(&config.mode),
        memory_mb: config.memory_mb,
        vcpus: config.vcpus,
        hypervisor: config.hypervisor.clone(),
        capabilities: config.capabilities.clone(),
        isolation_level: isolation_level_to_infra(&config.isolation_level),
    }
}

/// Convert infrastructure VmConfig to domain VmConfig
pub fn vm_config_from_infra(config: &infra_vm::VmConfig) -> domain::VmConfig {
    domain::VmConfig {
        id: config.id,
        mode: vm_mode_from_infra(&config.mode),
        memory_mb: config.memory_mb,
        vcpus: config.vcpus,
        hypervisor: config.hypervisor.clone(),
        capabilities: config.capabilities.clone(),
        isolation_level: isolation_level_from_infra(&config.isolation_level),
    }
}

// ============================================================================
// VM State Mapping
// ============================================================================

/// Convert domain VmState to infrastructure VmState
pub fn vm_state_to_infra(state: &domain::VmState) -> infra_vm::VmState {
    match state {
        domain::VmState::Starting => infra_vm::VmState::Starting,
        domain::VmState::Ready => infra_vm::VmState::Ready,
        domain::VmState::Busy { job_id, started_at } => infra_vm::VmState::Busy {
            job_id: job_id.clone(),
            started_at: *started_at,
        },
        domain::VmState::Idle {
            jobs_completed,
            last_job_at,
        } => infra_vm::VmState::Idle {
            jobs_completed: *jobs_completed,
            last_job_at: *last_job_at,
        },
        domain::VmState::Draining => infra_vm::VmState::Draining,
        domain::VmState::Terminated { reason, exit_code } => infra_vm::VmState::Terminated {
            reason: reason.clone(),
            exit_code: *exit_code,
        },
        domain::VmState::Failed { error } => infra_vm::VmState::Failed {
            error: error.clone(),
        },
    }
}

/// Convert infrastructure VmState to domain VmState
pub fn vm_state_from_infra(state: &infra_vm::VmState) -> domain::VmState {
    match state {
        infra_vm::VmState::Starting => domain::VmState::Starting,
        infra_vm::VmState::Ready => domain::VmState::Ready,
        infra_vm::VmState::Busy { job_id, started_at } => domain::VmState::Busy {
            job_id: job_id.clone(),
            started_at: *started_at,
        },
        infra_vm::VmState::Idle {
            jobs_completed,
            last_job_at,
        } => domain::VmState::Idle {
            jobs_completed: *jobs_completed,
            last_job_at: *last_job_at,
        },
        infra_vm::VmState::Draining => domain::VmState::Draining,
        infra_vm::VmState::Terminated { reason, exit_code } => domain::VmState::Terminated {
            reason: reason.clone(),
            exit_code: *exit_code,
        },
        infra_vm::VmState::Failed { error } => domain::VmState::Failed {
            error: error.clone(),
        },
    }
}

// ============================================================================
// VM Instance Mapping
// ============================================================================

/// Convert infrastructure VmInstance to domain VmInstance
///
/// Note: Infrastructure VmInstance has more fields (metrics, paths, etc.)
/// We only map the domain-relevant fields
pub fn vm_instance_from_infra(instance: &infra_vm::VmInstance) -> domain::VmInstance {
    domain::VmInstance {
        config: Arc::new(vm_config_from_infra(&instance.config)),
        state: vm_state_from_infra(&instance.state),
        created_at: instance.created_at,
        pid: instance.pid,
        ip_address: instance.ip_address.clone(),
    }
}

// ============================================================================
// Job Requirements Mapping
// ============================================================================

/// Convert domain JobRequirements to infrastructure JobRequirements
pub fn job_requirements_to_infra(req: &domain::JobRequirements) -> infra_vm::JobRequirements {
    infra_vm::JobRequirements {
        memory_mb: req.memory_mb,
        vcpus: req.vcpus,
        capabilities: req.capabilities.clone(),
        isolation_level: isolation_level_to_infra(&req.isolation_level),
        timeout_secs: req.timeout_secs,
        gpu_required: req.gpu_required,
    }
}

// ============================================================================
// Job Results and Assignment Mapping
// ============================================================================

/// Convert infrastructure JobResult to domain JobResult
pub fn job_result_from_infra(result: crate::infrastructure::vm::JobResult) -> domain::JobResult {
    domain::JobResult {
        vm_id: result.vm_id,
        success: result.success,
        output: result.output,
        error: result.error,
    }
}

/// Convert infrastructure VmAssignment to domain VmAssignment
pub fn vm_assignment_from_infra(assignment: infra_router::VmAssignment) -> domain::VmAssignment {
    match assignment {
        infra_router::VmAssignment::Ephemeral(id) => domain::VmAssignment::Ephemeral(id),
        infra_router::VmAssignment::Service(id) => domain::VmAssignment::Service(id),
    }
}

// ============================================================================
// Statistics Mapping
// ============================================================================

/// Convert infrastructure VmStats to domain VmStats
pub fn vm_stats_from_infra(stats: crate::infrastructure::vm::VmStats) -> domain::VmStats {
    domain::VmStats {
        total_vms: stats.total_vms,
        running_vms: stats.running_vms,
        idle_vms: stats.idle_vms,
        failed_vms: stats.failed_vms,
    }
}

// ============================================================================
// Health Status Mapping
// ============================================================================

/// Convert infrastructure HealthStatus to domain HealthStatus
pub fn health_status_from_infra(status: &infra_health::HealthStatus) -> domain::HealthStatus {
    match status {
        infra_health::HealthStatus::Unknown => domain::HealthStatus::Unknown,
        infra_health::HealthStatus::Healthy {
            last_check,
            response_time_ms,
        } => domain::HealthStatus::Healthy {
            last_check: *last_check,
            response_time_ms: *response_time_ms,
        },
        infra_health::HealthStatus::Degraded {
            last_check,
            failures,
            last_error,
        } => domain::HealthStatus::Degraded {
            last_check: *last_check,
            failures: *failures,
            last_error: last_error.clone(),
        },
        infra_health::HealthStatus::Unhealthy {
            since,
            failures,
            last_error,
        } => domain::HealthStatus::Unhealthy {
            since: *since,
            failures: *failures,
            last_error: last_error.clone(),
        },
    }
}

/// Convert domain HealthStatus to infrastructure HealthStatus
pub fn health_status_to_infra(status: &domain::HealthStatus) -> infra_health::HealthStatus {
    match status {
        domain::HealthStatus::Unknown => infra_health::HealthStatus::Unknown,
        domain::HealthStatus::Healthy {
            last_check,
            response_time_ms,
        } => infra_health::HealthStatus::Healthy {
            last_check: *last_check,
            response_time_ms: *response_time_ms,
        },
        domain::HealthStatus::Degraded {
            last_check,
            failures,
            last_error,
        } => infra_health::HealthStatus::Degraded {
            last_check: *last_check,
            failures: *failures,
            last_error: last_error.clone(),
        },
        domain::HealthStatus::Unhealthy {
            since,
            failures,
            last_error,
        } => infra_health::HealthStatus::Unhealthy {
            since: *since,
            failures: *failures,
            last_error: last_error.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_isolation_level_roundtrip() {
        let levels = vec![
            domain::IsolationLevel::Maximum,
            domain::IsolationLevel::Standard,
            domain::IsolationLevel::Minimal,
        ];

        for level in levels {
            let infra = isolation_level_to_infra(&level);
            let back = isolation_level_from_infra(&infra);
            assert_eq!(level, back);
        }
    }

    #[test]
    fn test_vm_mode_roundtrip() {
        let modes = vec![
            domain::VmMode::Ephemeral {
                job_id: "test-123".to_string(),
            },
            domain::VmMode::Service {
                queue_name: "default".to_string(),
                max_jobs: Some(50),
                max_uptime_secs: Some(3600),
            },
        ];

        for mode in modes {
            let infra = vm_mode_to_infra(&mode);
            let back = vm_mode_from_infra(&infra);
            assert_eq!(mode, back);
        }
    }

    #[test]
    fn test_health_status_roundtrip() {
        let statuses = vec![
            domain::HealthStatus::Unknown,
            domain::HealthStatus::Healthy {
                last_check: 1234567890,
                response_time_ms: 42,
            },
            domain::HealthStatus::Degraded {
                last_check: 1234567890,
                failures: 2,
                last_error: "timeout".to_string(),
            },
            domain::HealthStatus::Unhealthy {
                since: 1234567890,
                failures: 5,
                last_error: "connection refused".to_string(),
            },
        ];

        for status in statuses {
            let infra = health_status_to_infra(&status);
            let back = health_status_from_infra(&infra);
            assert_eq!(status, back);
        }
    }

    #[test]
    fn test_vm_config_conversion() {
        let domain_config = domain::VmConfig {
            id: Uuid::new_v4(),
            mode: domain::VmMode::Ephemeral {
                job_id: "job-123".to_string(),
            },
            memory_mb: 512,
            vcpus: 2,
            hypervisor: "firecracker".to_string(),
            capabilities: vec!["network".to_string()],
            isolation_level: domain::IsolationLevel::Maximum,
        };

        let infra_config = vm_config_to_infra(&domain_config);
        let back = vm_config_from_infra(&infra_config);

        assert_eq!(domain_config.id, back.id);
        assert_eq!(domain_config.mode, back.mode);
        assert_eq!(domain_config.memory_mb, back.memory_mb);
        assert_eq!(domain_config.vcpus, back.vcpus);
    }
}
