//! OpenTofu/Terraform Plan Executor
//!
//! This module handles execution of Terraform/OpenTofu plans using the
//! existing ExecutionBackend infrastructure.

use anyhow::{Result, Context, bail};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    hiqlite_service::HiqliteService,
    tofu::{
        state_backend::TofuStateBackend,
        types::*,
    },
};

/// Executes OpenTofu/Terraform plans using the execution backend system
#[derive(Clone)]
pub struct TofuPlanExecutor {
    state_backend: Arc<TofuStateBackend>,
    work_dir: PathBuf,
}

impl TofuPlanExecutor {
    /// Create a new plan executor
    pub fn new(
        hiqlite: Arc<HiqliteService>,
        work_dir: PathBuf,
    ) -> Self {
        let state_backend = Arc::new(TofuStateBackend::new(hiqlite.clone()));

        Self {
            state_backend,
            work_dir,
        }
    }

    /// Validate workspace name to prevent command injection
    ///
    /// Workspace names must be alphanumeric with optional hyphens, underscores, and dots.
    /// This prevents shell metacharacters and command injection attacks.
    fn validate_workspace_name(workspace: &str) -> Result<()> {
        // Check length constraints
        if workspace.is_empty() {
            bail!("Workspace name cannot be empty");
        }
        if workspace.len() > 90 {
            bail!("Workspace name exceeds maximum length of 90 characters");
        }

        // Check for valid characters: alphanumeric, hyphen, underscore, and dot only
        // This prevents command injection via shell metacharacters
        if !workspace.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
            bail!("Workspace name contains invalid characters. Only alphanumeric, hyphen, underscore, and dot are allowed");
        }

        // Prevent path traversal in workspace names
        if workspace.contains("..") || workspace.starts_with('/') || workspace.starts_with('\\') {
            bail!("Workspace name cannot contain path traversal sequences");
        }

        // Log security-relevant validation
        tracing::debug!(
            workspace = workspace,
            "Validated workspace name"
        );

        Ok(())
    }

    /// Validate and canonicalize a path to ensure it's within the allowed work directory
    ///
    /// This prevents path traversal attacks by ensuring the path doesn't escape
    /// the work directory, even through symlinks.
    async fn validate_path_in_work_dir(&self, path: &Path) -> Result<PathBuf> {
        // Canonicalize both paths to resolve symlinks and relative components
        let canonical_path = tokio::fs::canonicalize(path)
            .await
            .context("Failed to canonicalize path")?;

        let canonical_work_dir = tokio::fs::canonicalize(&self.work_dir)
            .await
            .context("Failed to canonicalize work directory")?;

        // Ensure the canonical path is within the work directory
        if !canonical_path.starts_with(&canonical_work_dir) {
            tracing::warn!(
                path = ?path,
                canonical_path = ?canonical_path,
                work_dir = ?canonical_work_dir,
                "Path traversal attempt detected"
            );
            bail!("Path traversal detected: path escapes work directory");
        }

        tracing::debug!(
            original_path = ?path,
            canonical_path = ?canonical_path,
            "Validated path within work directory"
        );

        Ok(canonical_path)
    }

    /// Validate a source path before copying to prevent path traversal
    async fn validate_source_path(&self, src: &Path, dst: &Path) -> Result<()> {
        // Ensure source exists
        if !tokio::fs::try_exists(src).await? {
            bail!("Source path does not exist: {}", src.display());
        }

        // Get canonical paths
        let canonical_src = tokio::fs::canonicalize(src)
            .await
            .context("Failed to canonicalize source path")?;

        let canonical_dst = if tokio::fs::try_exists(dst).await? {
            tokio::fs::canonicalize(dst)
                .await
                .context("Failed to canonicalize destination path")?
        } else {
            // For non-existent destinations, canonicalize parent and append filename
            let parent = dst.parent().context("Destination has no parent directory")?;
            let canonical_parent = tokio::fs::canonicalize(parent)
                .await
                .context("Failed to canonicalize destination parent")?;
            canonical_parent.join(dst.file_name().context("Destination has no filename")?)
        };

        // Ensure destination is within work directory
        let canonical_work_dir = tokio::fs::canonicalize(&self.work_dir)
            .await
            .context("Failed to canonicalize work directory")?;

        if !canonical_dst.starts_with(&canonical_work_dir) {
            tracing::warn!(
                src = ?src,
                dst = ?dst,
                canonical_dst = ?canonical_dst,
                work_dir = ?canonical_work_dir,
                "Path traversal attempt detected during file copy"
            );
            bail!("Path traversal detected: destination escapes work directory");
        }

        tracing::debug!(
            src = ?canonical_src,
            dst = ?canonical_dst,
            "Validated source and destination paths"
        );

        Ok(())
    }

    /// Execute a plan from a Terraform configuration directory
    pub async fn execute_plan(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        // Validate workspace name to prevent command injection
        Self::validate_workspace_name(workspace)
            .context("Invalid workspace name")?;

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

        // Validate workspace name from stored plan
        Self::validate_workspace_name(&plan.workspace)
            .context("Invalid workspace name in stored plan")?;

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
        if let Err(e) = tokio::fs::remove_dir_all(&work_dir).await {
            tracing::warn!(error = %e, work_dir = %work_dir.display(), "Failed to clean up work directory");
        }

        Ok(result)
    }

    /// Create a plan using OpenTofu CLI
    async fn create_plan(&self, work_dir: &Path, workspace: &str) -> Result<String> {
        // Validate workspace name before using in commands (defense in depth)
        Self::validate_workspace_name(workspace)
            .context("Invalid workspace name in create_plan")?;

        // Validate work directory to prevent path traversal
        let validated_work_dir = self.validate_path_in_work_dir(work_dir).await
            .context("Invalid work directory in create_plan")?;

        tracing::info!(
            workspace = workspace,
            work_dir = ?validated_work_dir,
            "Creating OpenTofu plan"
        );

        // Initialize if needed
        let init_output = Command::new("tofu")
            .args(&["init", "-backend=false"])
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !init_output.status.success() {
            let error_msg = String::from_utf8_lossy(&init_output.stderr);
            tracing::error!(
                workspace = workspace,
                error = %error_msg,
                "OpenTofu init failed"
            );
            return Err(anyhow::anyhow!(
                "OpenTofu init failed: {}",
                error_msg
            ));
        }

        // Select workspace - workspace name is validated, safe to use
        // Try to create workspace (it's ok if it fails - workspace may already exist)
        if let Err(e) = Command::new("tofu")
            .arg("workspace")
            .arg("new")
            .arg(workspace)
            .current_dir(&validated_work_dir)
            .output()
            .await
        {
            tracing::warn!(error = %e, workspace = workspace, "Failed to create workspace (may already exist)");
        }

        let select_output = Command::new("tofu")
            .arg("workspace")
            .arg("select")
            .arg(workspace)
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !select_output.status.success() {
            tracing::warn!(
                workspace = workspace,
                error = %String::from_utf8_lossy(&select_output.stderr),
                "Workspace selection failed (may be expected if workspace doesn't exist)"
            );
        }

        // Create plan
        let plan_output = Command::new("tofu")
            .args(&["plan", "-out=tfplan", "-no-color"])
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !plan_output.status.success() {
            let error_msg = String::from_utf8_lossy(&plan_output.stderr);
            tracing::error!(
                workspace = workspace,
                error = %error_msg,
                "OpenTofu plan failed"
            );
            return Err(anyhow::anyhow!(
                "OpenTofu plan failed: {}",
                error_msg
            ));
        }

        tracing::info!(
            workspace = workspace,
            "OpenTofu plan created successfully"
        );

        Ok(String::from_utf8_lossy(&plan_output.stdout).to_string())
    }

    /// Apply a plan using OpenTofu CLI
    async fn apply_plan(
        &self,
        work_dir: &Path,
        workspace: &str,
        plan_id: &str,
    ) -> Result<PlanExecutionResult> {
        // Validate workspace name before using in commands (defense in depth)
        Self::validate_workspace_name(workspace)
            .context("Invalid workspace name in apply_plan")?;

        // Validate work directory to prevent path traversal
        let validated_work_dir = self.validate_path_in_work_dir(work_dir).await
            .context("Invalid work directory in apply_plan")?;

        tracing::info!(
            workspace = workspace,
            plan_id = plan_id,
            work_dir = ?validated_work_dir,
            "Applying OpenTofu plan"
        );

        // Select workspace - workspace name is validated, safe to use
        let select_output = Command::new("tofu")
            .arg("workspace")
            .arg("select")
            .arg(workspace)
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !select_output.status.success() {
            tracing::warn!(
                workspace = workspace,
                error = %String::from_utf8_lossy(&select_output.stderr),
                "Workspace selection failed during apply"
            );
        }

        // Apply the plan
        let apply_output = Command::new("tofu")
            .args(&["apply", "-auto-approve", "-no-color", "tfplan"])
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&apply_output.stdout).to_string();
        let error_str = String::from_utf8_lossy(&apply_output.stderr).to_string();

        if apply_output.status.success() {
            tracing::info!(
                workspace = workspace,
                plan_id = plan_id,
                "OpenTofu plan applied successfully"
            );
        } else {
            tracing::error!(
                workspace = workspace,
                plan_id = plan_id,
                error = %error_str,
                "OpenTofu plan application failed"
            );
        }

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
        // Validate source and destination paths before copying
        self.validate_source_path(src, dst).await
            .context("Path validation failed for config copy")?;

        tracing::debug!(
            src = ?src,
            dst = ?dst,
            "Copying configuration files to work directory"
        );

        let mut entries = tokio::fs::read_dir(src).await
            .context("Failed to read source directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let dst_path = dst.join(&file_name);

            // Validate that the destination path is still within work directory
            // This prevents symlink attacks where a symlink could point outside the work dir
            self.validate_source_path(&path, &dst_path).await
                .context("Path validation failed during recursive copy")?;

            // Get metadata without following symlinks to detect symlink attacks
            let metadata = tokio::fs::symlink_metadata(&path).await?;

            if metadata.is_symlink() {
                // Log and skip symlinks to prevent path traversal attacks
                tracing::warn!(
                    path = ?path,
                    "Skipping symlink in configuration directory (security policy)"
                );
                continue;
            }

            if metadata.is_dir() {
                tokio::fs::create_dir_all(&dst_path).await?;
                Box::pin(self.copy_config_to_work_dir(&path, &dst_path)).await?;
            } else if metadata.is_file() {
                // Only copy Terraform files
                if path.extension().map_or(false, |ext| ext == "tf" || ext == "tfvars") {
                    tokio::fs::copy(&path, &dst_path).await
                        .context("Failed to copy configuration file")?;

                    tracing::debug!(
                        src = ?path,
                        dst = ?dst_path,
                        "Copied configuration file"
                    );
                }
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
        // Validate workspace name to prevent command injection
        Self::validate_workspace_name(workspace)
            .context("Invalid workspace name")?;

        // Create work directory
        let execution_id = Uuid::new_v4().to_string();
        let work_dir = self.work_dir.join(&execution_id);
        tokio::fs::create_dir_all(&work_dir).await?;

        // Copy configuration
        self.copy_config_to_work_dir(config_dir, &work_dir).await?;

        // Validate work directory to prevent path traversal
        let validated_work_dir = self.validate_path_in_work_dir(&work_dir).await
            .context("Invalid work directory in destroy")?;

        tracing::info!(
            workspace = workspace,
            work_dir = ?validated_work_dir,
            auto_approve = auto_approve,
            "Destroying OpenTofu infrastructure"
        );

        // Initialize
        let init_output = Command::new("tofu")
            .args(&["init", "-backend=false"])
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !init_output.status.success() {
            let error_msg = String::from_utf8_lossy(&init_output.stderr);
            tracing::error!(
                workspace = workspace,
                error = %error_msg,
                "OpenTofu init failed during destroy"
            );
            return Err(anyhow::anyhow!(
                "OpenTofu init failed: {}",
                error_msg
            ));
        }

        // Select workspace - workspace name is validated, safe to use
        let select_output = Command::new("tofu")
            .arg("workspace")
            .arg("select")
            .arg(workspace)
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        if !select_output.status.success() {
            let error_msg = String::from_utf8_lossy(&select_output.stderr);
            tracing::error!(
                error = %error_msg,
                workspace = workspace,
                "Failed to select workspace during destroy"
            );
            return Err(anyhow::anyhow!("Failed to select workspace: {}", error_msg));
        }

        // Destroy
        let mut args = vec!["destroy", "-no-color"];
        if auto_approve {
            args.push("-auto-approve");
        }

        let destroy_output = Command::new("tofu")
            .args(&args)
            .current_dir(&validated_work_dir)
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&destroy_output.stdout).to_string();
        let error_str = String::from_utf8_lossy(&destroy_output.stderr).to_string();

        if destroy_output.status.success() {
            tracing::info!(
                workspace = workspace,
                "Infrastructure destroyed successfully"
            );
        } else {
            tracing::error!(
                workspace = workspace,
                error = %error_str,
                "Infrastructure destruction failed"
            );
        }

        // Parse for destroyed resources
        let destroyed_count = output_str.lines()
            .filter(|line| line.contains("Destroy complete!"))
            .find_map(|line| {
                line.split_whitespace()
                    .find_map(|word| word.parse::<i32>().ok())
            })
            .unwrap_or(0);

        // Clean up
        if let Err(e) = tokio::fs::remove_dir_all(&work_dir).await {
            tracing::warn!(error = %e, work_dir = %work_dir.display(), "Failed to clean up work directory");
        }

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