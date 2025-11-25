// Job Router - Intelligent job distribution to VMs

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::vm_controller::VmController;
use super::vm_registry::VmRegistry;
use super::vm_types::{IsolationLevel, JobRequirements, VmConfig, VmMode, VmState};
use crate::Job;

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

/// Rules for routing jobs to VMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRules {
    /// Patterns that require high isolation (regex)
    pub high_isolation_patterns: Vec<String>,
    /// Job types that can share VMs
    pub shareable_job_types: Vec<String>,
    /// Job types that require dedicated VMs
    pub dedicated_job_types: Vec<String>,
    /// Maximum jobs per service VM
    pub max_jobs_per_vm: u32,
    /// Enable automatic VM creation
    pub auto_create_vms: bool,
    /// Prefer service VMs over ephemeral when possible
    pub prefer_service_vms: bool,
}

impl Default for RoutingRules {
    fn default() -> Self {
        Self {
            high_isolation_patterns: vec![
                r".*untrusted.*".to_string(),
                r".*external.*".to_string(),
                r".*sandbox.*".to_string(),
            ],
            shareable_job_types: vec![
                "build".to_string(),
                "test".to_string(),
                "analysis".to_string(),
            ],
            dedicated_job_types: vec![
                "security-scan".to_string(),
                "penetration-test".to_string(),
            ],
            max_jobs_per_vm: 50,
            auto_create_vms: true,
            prefer_service_vms: true,
        }
    }
}

/// Job router for intelligent distribution
pub struct JobRouter {
    registry: Arc<VmRegistry>,
    controller: Arc<VmController>,
    rules: Arc<RwLock<RoutingRules>>,
    stopped: Arc<RwLock<bool>>,
}

impl JobRouter {
    /// Create new job router
    pub fn new(registry: Arc<VmRegistry>, controller: Arc<VmController>) -> Self {
        Self {
            registry,
            controller,
            rules: Arc::new(RwLock::new(RoutingRules::default())),
            stopped: Arc::new(RwLock::new(false)),
        }
    }

    /// Route a job to appropriate VM
    pub async fn route_job(&self, job: &Job) -> Result<VmAssignment> {
        // Check if routing is stopped
        if *self.stopped.read().await {
            return Err(anyhow!("Job routing is stopped"));
        }

        let requirements = self.analyze_job_requirements(job).await?;

        tracing::debug!(
            job_id = %job.id,
            isolation = ?requirements.isolation_level,
            memory_mb = requirements.memory_mb,
            vcpus = requirements.vcpus,
            "Analyzed job requirements"
        );

        // Route based on isolation level
        match requirements.isolation_level.clone() {
            IsolationLevel::Maximum => {
                // Always create new ephemeral VM for maximum isolation
                self.route_to_ephemeral_vm(job, requirements).await
            }
            IsolationLevel::Standard => {
                // Try to use service VM, fall back to ephemeral
                if self.rules.read().await.prefer_service_vms {
                    // Clone requirements for fallback case
                    let req_clone = requirements.clone();
                    match self.route_to_service_vm(job, requirements).await {
                        Ok(assignment) => Ok(assignment),
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                "Failed to route to service VM, falling back to ephemeral"
                            );
                            self.route_to_ephemeral_vm(job, req_clone).await
                        }
                    }
                } else {
                    self.route_to_ephemeral_vm(job, requirements).await
                }
            }
            IsolationLevel::Minimal => {
                // Prefer service VMs for minimal isolation
                self.route_to_service_vm(job, requirements).await
            }
        }
    }

    /// Route job to ephemeral VM
    async fn route_to_ephemeral_vm(
        &self,
        job: &Job,
        requirements: JobRequirements,
    ) -> Result<VmAssignment> {
        tracing::info!(job_id = %job.id, "Creating ephemeral VM for job");

        let config = VmConfig {
            id: Uuid::new_v4(),
            mode: VmMode::Ephemeral {
                job_id: job.id.clone(),
            },
            memory_mb: requirements.memory_mb,
            vcpus: requirements.vcpus,
            hypervisor: if requirements.isolation_level == IsolationLevel::Maximum {
                "firecracker".to_string()
            } else {
                "qemu".to_string()
            },
            capabilities: requirements.capabilities,
            isolation_level: requirements.isolation_level,
        };

        let vm = self.controller.start_vm(config).await?;

        Ok(VmAssignment::Ephemeral(vm.config.id))
    }

    /// Route job to service VM
    async fn route_to_service_vm(
        &self,
        job: &Job,
        requirements: JobRequirements,
    ) -> Result<VmAssignment> {
        // Try to find existing suitable VM
        if let Some(vm_id) = self
            .registry
            .get_available_service_vm()
            .await
        {
            tracing::info!(
                job_id = %job.id,
                vm_id = %vm_id,
                "Found existing service VM for job"
            );

            // Update VM state to busy
            self.registry
                .update_state(
                    vm_id,
                    VmState::Busy {
                        job_id: job.id.clone(),
                        started_at: chrono::Utc::now().timestamp(),
                    },
                )
                .await?;

            return Ok(VmAssignment::Service(vm_id));
        }

        // No suitable VM found, create new one if allowed
        if !self.rules.read().await.auto_create_vms {
            return Err(anyhow!("No available service VM and auto-creation disabled"));
        }

        tracing::info!(job_id = %job.id, "Creating new service VM for job");

        let config = VmConfig {
            id: Uuid::new_v4(),
            mode: VmMode::Service {
                queue_name: "default".to_string(),
                max_jobs: Some(self.rules.read().await.max_jobs_per_vm),
                max_uptime_secs: Some(3600), // 1 hour
            },
            memory_mb: requirements.memory_mb.max(1024), // Service VMs get more memory
            vcpus: requirements.vcpus.max(2),             // Service VMs get more CPUs
            hypervisor: "qemu".to_string(),
            capabilities: requirements.capabilities,
            isolation_level: requirements.isolation_level,
        };

        let vm = self.controller.start_vm(config).await?;

        // Mark as busy with this job
        self.registry
            .update_state(
                vm.config.id,
                VmState::Busy {
                    job_id: job.id.clone(),
                    started_at: chrono::Utc::now().timestamp(),
                },
            )
            .await?;

        Ok(VmAssignment::Service(vm.config.id))
    }

    /// Analyze job to determine requirements
    async fn analyze_job_requirements(&self, job: &Job) -> Result<JobRequirements> {
        let payload = &job.payload;
        let rules = self.rules.read().await;

        // Determine isolation level
        let isolation_level = if self.requires_high_isolation(job, &rules).await {
            IsolationLevel::Maximum
        } else if self.is_trusted_workload(job, &rules).await {
            IsolationLevel::Minimal
        } else {
            IsolationLevel::Standard
        };

        // Extract resource requirements from job payload
        let memory_mb = payload
            .get("memory_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(512) as u32;

        let vcpus = payload
            .get("vcpus")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let timeout_secs = payload
            .get("timeout_secs")
            .and_then(|v| v.as_u64());

        let gpu_required = payload
            .get("gpu_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract capabilities
        let capabilities = payload
            .get("capabilities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(JobRequirements {
            memory_mb,
            vcpus,
            capabilities,
            isolation_level,
            timeout_secs,
            gpu_required,
        })
    }

    /// Check if job requires high isolation
    async fn requires_high_isolation(&self, job: &Job, rules: &RoutingRules) -> bool {
        // Check job type
        let job_type = job.payload
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if rules.dedicated_job_types.contains(&job_type.to_string()) {
            return true;
        }

        // Check patterns
        let job_str = job.payload.to_string();
        for pattern in &rules.high_isolation_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(&job_str) {
                    return true;
                }
            }
        }

        // Check explicit isolation flag
        job.payload
            .get("require_isolation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Check if job is trusted workload
    async fn is_trusted_workload(&self, job: &Job, rules: &RoutingRules) -> bool {
        // Check job type
        let job_type = job.payload
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if rules.shareable_job_types.contains(&job_type.to_string()) {
            return true;
        }

        // Check trust flag
        job.payload
            .get("trusted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Update routing rules
    pub async fn update_rules(&self, rules: RoutingRules) {
        *self.rules.write().await = rules;
        tracing::info!("Routing rules updated");
    }

    /// Stop routing (for graceful shutdown)
    pub async fn stop_routing(&self) {
        *self.stopped.write().await = true;
        tracing::info!("Job routing stopped");
    }

    /// Get routing statistics
    pub async fn get_stats(&self) -> RouteStats {
        let total_vms = self.registry.count_all().await;
        let service_vms = self.registry
            .list_all_vms()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|vm| matches!(vm.config.mode, VmMode::Service { .. }))
            .count();

        RouteStats {
            total_vms,
            ephemeral_vms: total_vms - service_vms,
            service_vms,
            available_service_vms: self
                .registry
                .list_vms_by_state(VmState::Idle {
                    jobs_completed: 0,
                    last_job_at: 0,
                })
                .await
                .unwrap_or_default()
                .len(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteStats {
    pub total_vms: usize,
    pub ephemeral_vms: usize,
    pub service_vms: usize,
    pub available_service_vms: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_isolation_analysis() {
        let router = JobRouter::new(
            Arc::new(VmRegistry::new(&std::path::PathBuf::from("/tmp")).await.unwrap()),
            Arc::new(VmController::new(
                crate::vm_manager::VmManagerConfig::default(),
                Arc::new(VmRegistry::new(&std::path::PathBuf::from("/tmp")).await.unwrap()),
            ).unwrap()),
        );

        // Test high isolation job
        let job = Job {
            id: "test-1".to_string(),
            status: crate::JobStatus::Pending,
            payload: json!({
                "type": "security-scan",
                "require_isolation": true
            }),
            // ... other fields
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],
        };

        let requirements = router.analyze_job_requirements(&job).await.unwrap();
        assert_eq!(requirements.isolation_level, IsolationLevel::Maximum);

        // Test trusted job
        let job = Job {
            id: "test-2".to_string(),
            status: crate::JobStatus::Pending,
            payload: json!({
                "type": "build",
                "trusted": true
            }),
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],
        };

        let requirements = router.analyze_job_requirements(&job).await.unwrap();
        assert_eq!(requirements.isolation_level, IsolationLevel::Minimal);
    }
}