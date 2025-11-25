//! OpenTofu/Terraform Plan Executor
//!
//! This module handles execution of Terraform/OpenTofu plans using the
//! existing ExecutionBackend infrastructure.

use anyhow::Result;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    adapters::{
        ExecutionBackend,
        ExecutionConfig,
        ExecutionHandle,
        ExecutionRegistry,
    },
    domain::types::{Job, JobStatus},
    hiqlite_service::HiqliteService,
    tofu::{
        state_backend::TofuStateBackend,
        types::*,
    },
};

/// Executes OpenTofu/Terraform plans using the execution backend system
#[derive(Clone)]
pub struct TofuPlanExecutor {
    registry: Arc<ExecutionRegistry>,
    state_backend: Arc<TofuStateBackend>,
    hiqlite: Arc<HiqliteService>,
    work_dir: PathBuf,
}

impl TofuPlanExecutor {
    /// Create a new plan executor
    pub fn new(
        registry: Arc<ExecutionRegistry>,
        hiqlite: Arc<HiqliteService>,
        work_dir: PathBuf,
    ) -> Self {
        let state_backend = Arc::new(TofuStateBackend::new(hiqlite.clone()));

        Self {
            registry,
            state_backend,
            hiqlite,
            work_dir,
        }
    }

    /// Execute a plan from a Terraform configuration directory
    pub async fn execute_plan(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        // Ensure workspace exists
        self.state_backend.get_or_create_workspace(workspace).await?;

        // Create a unique execution ID
        let execution_id = Uuid::new_v4().to_string();
        let work_dir = self.work_dir.join(&execution_id);
        tokio::fs::create_dir_all(&work_dir).await?;

        // Copy configuration to work directory
        self.copy_config_to_work_dir(config_dir, &work_dir).await?;

        // Create the plan
        let plan_output = self.create_plan(&work_dir, workspace).await?;

        // Parse plan output to determine changes
        let (resources_created, resources_updated, resources_destroyed) =
            self.parse_plan_summary(&plan_output);

        // Store the plan
        let plan_id = Uuid::new_v4().to_string();
        let plan_data = self.read_plan_file(&work_dir).await?;

        self.state_backend.store_plan(StoredPlan {
            id: plan_id.clone(),
            workspace: workspace.to_string(),
            created_at: 0, // Will be set by store_plan
            plan_data,
            plan_json: Some(plan_output.clone()),
            status: PlanStatus::Pending,
            approved_by: None,
            executed_at: None,
        }).await?;

        // Execute the plan if auto-approved
        if auto_approve {
            let apply_result = self.apply_plan(&work_dir, workspace, &plan_id).await?;

            // Update plan status
            self.state_backend.update_plan_status(
                &plan_id,
                if apply_result.success { PlanStatus::Applied } else { PlanStatus::Failed },
                Some("auto-approve".to_string()),
            ).await?;

            return Ok(apply_result);
        }

        Ok(PlanExecutionResult {
            success: true,
            output: format!("Plan created successfully. Plan ID: {}", plan_id),
            errors: vec![],
            resources_created,
            resources_updated,
            resources_destroyed,
        })
    }

    /// Apply a stored plan
    pub async fn apply_stored_plan(
        &self,
        plan_id: &str,
        approver: &str,
    ) -> Result<PlanExecutionResult> {
        // Get the stored plan
        let plan = self.state_backend.get_plan(plan_id).await?;

        if plan.status != PlanStatus::Pending && plan.status != PlanStatus::Approved {
            return Err(anyhow::anyhow!("Plan is not in a state that can be applied"));
        }

        // Update status to approved
        self.state_backend.update_plan_status(
            plan_id,
            PlanStatus::Approved,
            Some(approver.to_string()),
        ).await?;

        // Create work directory
        let execution_id = Uuid::new_v4().to_string();
        let work_dir = self.work_dir.join(&execution_id);
        tokio::fs::create_dir_all(&work_dir).await?;

        // Write plan to file
        let plan_file = work_dir.join("tfplan");
        tokio::fs::write(&plan_file, &plan.plan_data).await?;

        // Apply the plan
        let result = self.apply_plan(&work_dir, &plan.workspace, plan_id).await?;

        // Update plan status
        self.state_backend.update_plan_status(
            plan_id,
            if result.success { PlanStatus::Applied } else { PlanStatus::Failed },
            None,
        ).await?;

        // Clean up work directory
        let _ = tokio::fs::remove_dir_all(&work_dir).await;

        Ok(result)
    }

    /// Execute OpenTofu plan using the execution backend
    pub async fn execute_via_backend(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        // Create a job for the execution
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let job = Job {
            id: Uuid::new_v4().to_string(),
            payload: serde_json::json!({
                "type": "tofu-plan",
                "workspace": workspace,
                "config_dir": config_dir.to_string_lossy(),
                "auto_approve": auto_approve,
            }),
            status: JobStatus::Pending,
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            error_message: None,
            retry_count: 0,
            compatible_worker_types: vec![],  // Empty means any worker type
        };

        // Submit job to execution registry
        let config = ExecutionConfig {
            resources: crate::adapters::ResourceRequirements {
                cpu_cores: 2.0,
                memory_mb: 2048,
                ..Default::default()
            },
            timeout: Some(std::time::Duration::from_secs(600)),
            environment: std::collections::HashMap::new(),
            ..Default::default()
        };

        let handle = self.registry.submit_job(job, config).await?;

        // Wait for execution to complete
        let timeout = Some(std::time::Duration::from_secs(600));
        let status = self.registry.wait_for_completion(&handle, timeout).await?;

        match status {
            crate::adapters::ExecutionStatus::Completed(_) => {
                Ok(PlanExecutionResult {
                    success: true,
                    output: format!("OpenTofu execution completed successfully"),
                    errors: vec![],
                    resources_created: 0,
                    resources_updated: 0,
                    resources_destroyed: 0,
                })
            }
            crate::adapters::ExecutionStatus::Failed(error) => {
                Ok(PlanExecutionResult {
                    success: false,
                    output: String::new(),
                    errors: vec![error],
                    resources_created: 0,
                    resources_updated: 0,
                    resources_destroyed: 0,
                })
            }
            _ => {
                Ok(PlanExecutionResult {
                    success: false,
                    output: String::new(),
                    errors: vec!["Execution did not complete within timeout".to_string()],
                    resources_created: 0,
                    resources_updated: 0,
                    resources_destroyed: 0,
                })
            }
        }
    }

    /// Create a plan using OpenTofu CLI
    async fn create_plan(&self, work_dir: &Path, workspace: &str) -> Result<String> {
        // Initialize if needed
        let init_output = Command::new("tofu")
            .args(&["init", "-backend=false"])
            .current_dir(work_dir)
            .output()
            .await?;

        if !init_output.status.success() {
            return Err(anyhow::anyhow!(
                "OpenTofu init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            ));
        }

        // Select workspace
        let _ = Command::new("tofu")
            .args(&["workspace", "new", workspace])
            .current_dir(work_dir)
            .output()
            .await;

        let _ = Command::new("tofu")
            .args(&["workspace", "select", workspace])
            .current_dir(work_dir)
            .output()
            .await;

        // Create plan
        let plan_output = Command::new("tofu")
            .args(&["plan", "-out=tfplan", "-no-color"])
            .current_dir(work_dir)
            .output()
            .await?;

        if !plan_output.status.success() {
            return Err(anyhow::anyhow!(
                "OpenTofu plan failed: {}",
                String::from_utf8_lossy(&plan_output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&plan_output.stdout).to_string())
    }

    /// Apply a plan using OpenTofu CLI
    async fn apply_plan(
        &self,
        work_dir: &Path,
        workspace: &str,
        plan_id: &str,
    ) -> Result<PlanExecutionResult> {
        // Select workspace
        let _ = Command::new("tofu")
            .args(&["workspace", "select", workspace])
            .current_dir(work_dir)
            .output()
            .await;

        // Apply the plan
        let apply_output = Command::new("tofu")
            .args(&["apply", "-auto-approve", "-no-color", "tfplan"])
            .current_dir(work_dir)
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&apply_output.stdout).to_string();
        let error_str = String::from_utf8_lossy(&apply_output.stderr).to_string();

        // Parse the output for resource counts
        let (resources_created, resources_updated, resources_destroyed) =
            self.parse_apply_summary(&output_str);

        Ok(PlanExecutionResult {
            success: apply_output.status.success(),
            output: output_str,
            errors: if apply_output.status.success() {
                vec![]
            } else {
                vec![error_str]
            },
            resources_created,
            resources_updated,
            resources_destroyed,
        })
    }

    /// Copy configuration files to work directory
    async fn copy_config_to_work_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        // Simple recursive copy - in production, use a proper copy utility
        let mut entries = tokio::fs::read_dir(src).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let dst_path = dst.join(file_name);

            if path.is_dir() {
                tokio::fs::create_dir_all(&dst_path).await?;
                Box::pin(self.copy_config_to_work_dir(&path, &dst_path)).await?;
            } else if path.extension().map_or(false, |ext| ext == "tf" || ext == "tfvars") {
                tokio::fs::copy(&path, &dst_path).await?;
            }
        }

        Ok(())
    }

    /// Read the plan file
    async fn read_plan_file(&self, work_dir: &Path) -> Result<Vec<u8>> {
        let plan_file = work_dir.join("tfplan");
        tokio::fs::read(&plan_file).await.map_err(Into::into)
    }

    /// Parse plan output for resource summary
    fn parse_plan_summary(&self, output: &str) -> (i32, i32, i32) {
        let mut created = 0;
        let mut updated = 0;
        let mut destroyed = 0;

        // Look for the summary line in OpenTofu output
        // Example: "Plan: 3 to add, 2 to change, 1 to destroy."
        for line in output.lines() {
            if line.contains("Plan:") {
                // Parse the numbers
                if let Some(add_match) = line.find(" to add") {
                    let start = line[..add_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..add_match].trim().parse::<i32>() {
                        created = num;
                    }
                }
                if let Some(change_match) = line.find(" to change") {
                    let start = line[..change_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..change_match].trim().parse::<i32>() {
                        updated = num;
                    }
                }
                if let Some(destroy_match) = line.find(" to destroy") {
                    let start = line[..destroy_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..destroy_match].trim().parse::<i32>() {
                        destroyed = num;
                    }
                }
                break;
            }
        }

        (created, updated, destroyed)
    }

    /// Parse apply output for resource summary
    fn parse_apply_summary(&self, output: &str) -> (i32, i32, i32) {
        let mut created = 0;
        let mut updated = 0;
        let mut destroyed = 0;

        // Look for the summary line in OpenTofu apply output
        // Example: "Apply complete! Resources: 3 added, 2 changed, 1 destroyed."
        for line in output.lines() {
            if line.contains("Apply complete!") && line.contains("Resources:") {
                // Parse the numbers
                if let Some(added_match) = line.find(" added") {
                    let start = line[..added_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..added_match].trim().parse::<i32>() {
                        created = num;
                    }
                }
                if let Some(changed_match) = line.find(" changed") {
                    let start = line[..changed_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..changed_match].trim().parse::<i32>() {
                        updated = num;
                    }
                }
                if let Some(destroyed_match) = line.find(" destroyed") {
                    let start = line[..destroyed_match].rfind(' ').map(|i| i + 1).unwrap_or(0);
                    if let Ok(num) = line[start..destroyed_match].trim().parse::<i32>() {
                        destroyed = num;
                    }
                }
                break;
            }
        }

        (created, updated, destroyed)
    }

    /// Destroy infrastructure
    pub async fn destroy(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        // Create work directory
        let execution_id = Uuid::new_v4().to_string();
        let work_dir = self.work_dir.join(&execution_id);
        tokio::fs::create_dir_all(&work_dir).await?;

        // Copy configuration
        self.copy_config_to_work_dir(config_dir, &work_dir).await?;

        // Initialize
        let init_output = Command::new("tofu")
            .args(&["init", "-backend=false"])
            .current_dir(&work_dir)
            .output()
            .await?;

        if !init_output.status.success() {
            return Err(anyhow::anyhow!(
                "OpenTofu init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            ));
        }

        // Select workspace
        let _ = Command::new("tofu")
            .args(&["workspace", "select", workspace])
            .current_dir(&work_dir)
            .output()
            .await;

        // Destroy
        let mut args = vec!["destroy", "-no-color"];
        if auto_approve {
            args.push("-auto-approve");
        }

        let destroy_output = Command::new("tofu")
            .args(&args)
            .current_dir(&work_dir)
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&destroy_output.stdout).to_string();
        let error_str = String::from_utf8_lossy(&destroy_output.stderr).to_string();

        // Parse for destroyed resources
        let destroyed_count = output_str.lines()
            .filter(|line| line.contains("Destroy complete!"))
            .find_map(|line| {
                line.split_whitespace()
                    .find_map(|word| word.parse::<i32>().ok())
            })
            .unwrap_or(0);

        // Clean up
        let _ = tokio::fs::remove_dir_all(&work_dir).await;

        Ok(PlanExecutionResult {
            success: destroy_output.status.success(),
            output: output_str,
            errors: if destroy_output.status.success() {
                vec![]
            } else {
                vec![error_str]
            },
            resources_created: 0,
            resources_updated: 0,
            resources_destroyed: destroyed_count,
        })
    }
}