//! OpenTofu/Terraform Plan Executor
//!
//! This module handles execution of Terraform/OpenTofu plans using the
//! existing ExecutionBackend infrastructure.

use anyhow::{Result, Context, bail};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{
    hiqlite::HiqliteService,
    tofu::{
        state_backend::TofuStateBackend,
        executor::TofuExecutor,
        types::*,
        validation, parser,
        workspace_manager::WorkspaceManager,
        file_operations::FileOperations,
    },
};

/// Executes OpenTofu/Terraform plans using the execution backend system
pub struct TofuPlanExecutor {
    state_backend: Arc<TofuStateBackend>,
    executor: Arc<dyn TofuExecutor>,
    workspace_manager: WorkspaceManager,
    file_ops: FileOperations,
    work_dir: PathBuf,
}

impl TofuPlanExecutor {
    /// Create a new plan executor with a custom executor
    pub fn new(
        hiqlite: Arc<HiqliteService>,
        executor: Arc<dyn TofuExecutor>,
        work_dir: PathBuf,
    ) -> Self {
        let state_backend = Arc::new(TofuStateBackend::new(hiqlite.clone()));
        let workspace_manager = WorkspaceManager::new(executor.clone(), work_dir.clone());
        let file_ops = FileOperations::new(work_dir.clone());

        Self {
            state_backend,
            executor,
            workspace_manager,
            file_ops,
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
        validation::validate_workspace_name(workspace)?;
        self.state_backend.get_or_create_workspace(workspace).await?;

        let work_dir = self.workspace_manager.prepare_work_directory().await?;
        self.file_ops.copy_config_to_work_dir(config_dir, &work_dir).await?;

        let plan_output = self.create_plan(&work_dir, workspace).await?;
        let plan_id = self.store_plan_from_output(workspace, &work_dir, &plan_output).await?;

        if auto_approve {
            self.apply_and_update_plan(workspace, &work_dir, &plan_id).await
        } else {
            self.create_pending_plan_result(&plan_id, &plan_output)
        }
    }

    /// Store a plan from the tofu plan output
    async fn store_plan_from_output(
        &self,
        workspace: &str,
        work_dir: &Path,
        plan_output: &str,
    ) -> Result<String> {
        let (_resources_created, _resources_updated, _resources_destroyed) =
            parser::parse_plan_summary(plan_output);

        let plan_id = Uuid::new_v4().to_string();
        let plan_data = self.file_ops.read_plan_file(work_dir).await?;

        self.state_backend.store_plan(StoredPlan {
            id: plan_id.clone(),
            workspace: workspace.to_string(),
            created_at: 0,
            plan_data,
            plan_json: Some(plan_output.to_string()),
            status: PlanStatus::Pending,
            approved_by: None,
            executed_at: None,
        }).await?;

        Ok(plan_id)
    }

    /// Apply a plan and update its status
    async fn apply_and_update_plan(
        &self,
        workspace: &str,
        work_dir: &Path,
        plan_id: &str,
    ) -> Result<PlanExecutionResult> {
        let apply_result = self.apply_plan(work_dir, workspace, plan_id).await?;

        let status = if apply_result.success {
            PlanStatus::Applied
        } else {
            PlanStatus::Failed
        };

        self.state_backend.update_plan_status(
            plan_id,
            status,
            Some("auto-approve".to_string()),
        ).await?;

        Ok(apply_result)
    }

    /// Create a pending plan result
    fn create_pending_plan_result(
        &self,
        plan_id: &str,
        plan_output: &str,
    ) -> Result<PlanExecutionResult> {
        let (resources_created, resources_updated, resources_destroyed) =
            parser::parse_plan_summary(plan_output);

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
        validation::validate_workspace_name(&plan.workspace)
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
        let work_dir = self.workspace_manager.prepare_work_directory().await?;

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
        self.workspace_manager.cleanup_work_directory(&work_dir).await;

        Ok(result)
    }

    /// Create a plan using OpenTofu CLI
    async fn create_plan(&self, work_dir: &Path, workspace: &str) -> Result<String> {
        validation::validate_workspace_name(workspace)?;
        let validated_work_dir = validation::validate_path_in_work_dir(&self.work_dir, work_dir).await?;

        tracing::info!(workspace = workspace, work_dir = ?validated_work_dir, "Creating OpenTofu plan");

        self.workspace_manager.run_tofu_init(&validated_work_dir, workspace).await?;
        self.workspace_manager.ensure_workspace_selected(&validated_work_dir, workspace).await?;
        self.run_tofu_plan(&validated_work_dir, workspace).await
    }

    /// Run tofu plan command
    async fn run_tofu_plan(&self, work_dir: &Path, workspace: &str) -> Result<String> {
        let plan_output = self.executor.plan(work_dir).await?;

        if !plan_output.success {
            tracing::error!(workspace = workspace, error = %plan_output.stderr, "OpenTofu plan failed");
            bail!("OpenTofu plan failed: {}", plan_output.stderr);
        }

        tracing::info!(workspace = workspace, "OpenTofu plan created successfully");
        Ok(plan_output.stdout)
    }

    /// Apply a plan using OpenTofu CLI
    async fn apply_plan(
        &self,
        work_dir: &Path,
        workspace: &str,
        plan_id: &str,
    ) -> Result<PlanExecutionResult> {
        validation::validate_workspace_name(workspace)?;
        let validated_work_dir = validation::validate_path_in_work_dir(&self.work_dir, work_dir).await?;

        tracing::info!(workspace = workspace, plan_id = plan_id, work_dir = ?validated_work_dir, "Applying OpenTofu plan");

        self.workspace_manager.select_workspace(&validated_work_dir, workspace).await?;
        let apply_output = self.run_tofu_apply(&validated_work_dir).await?;

        self.log_apply_result(&apply_output, workspace, plan_id);
        self.create_apply_result(apply_output)
    }

    /// Run tofu apply command
    async fn run_tofu_apply(&self, work_dir: &Path) -> Result<crate::tofu::executor::TofuOutput> {
        self.executor.apply(work_dir).await
    }

    /// Log the result of tofu apply
    fn log_apply_result(&self, output: &crate::tofu::executor::TofuOutput, workspace: &str, plan_id: &str) {
        if output.success {
            tracing::info!(workspace = workspace, plan_id = plan_id, "OpenTofu plan applied successfully");
        } else {
            tracing::error!(workspace = workspace, plan_id = plan_id, error = %output.stderr, "OpenTofu plan application failed");
        }
    }

    /// Create PlanExecutionResult from apply output
    fn create_apply_result(&self, apply_output: crate::tofu::executor::TofuOutput) -> Result<PlanExecutionResult> {
        let (resources_created, resources_updated, resources_destroyed) =
            parser::parse_apply_summary(&apply_output.stdout);

        Ok(PlanExecutionResult {
            success: apply_output.success,
            output: apply_output.stdout.clone(),
            errors: if apply_output.success { vec![] } else { vec![apply_output.stderr] },
            resources_created,
            resources_updated,
            resources_destroyed,
        })
    }

    /// Destroy infrastructure
    pub async fn destroy(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        validation::validate_workspace_name(workspace)?;

        let work_dir = self.workspace_manager.prepare_work_directory().await?;
        self.file_ops.copy_config_to_work_dir(config_dir, &work_dir).await?;
        let validated_work_dir = validation::validate_path_in_work_dir(&self.work_dir, &work_dir).await?;

        tracing::info!(workspace = workspace, work_dir = ?validated_work_dir, auto_approve = auto_approve, "Destroying OpenTofu infrastructure");

        self.workspace_manager.run_tofu_init(&validated_work_dir, workspace).await?;
        self.workspace_manager.select_workspace_strict(&validated_work_dir, workspace).await?;

        let result = self.run_tofu_destroy(&validated_work_dir, workspace, auto_approve).await?;
        self.workspace_manager.cleanup_work_directory(&work_dir).await;

        Ok(result)
    }

    /// Run tofu destroy command
    async fn run_tofu_destroy(
        &self,
        work_dir: &Path,
        workspace: &str,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        let destroy_output = self.executor.destroy(work_dir, auto_approve).await?;

        self.log_destroy_result(&destroy_output, workspace);
        self.create_destroy_result(destroy_output)
    }

    /// Log the result of tofu destroy
    fn log_destroy_result(&self, output: &crate::tofu::executor::TofuOutput, workspace: &str) {
        if output.success {
            tracing::info!(workspace = workspace, "Infrastructure destroyed successfully");
        } else {
            tracing::error!(workspace = workspace, error = %output.stderr, "Infrastructure destruction failed");
        }
    }

    /// Create PlanExecutionResult from destroy output
    fn create_destroy_result(&self, destroy_output: crate::tofu::executor::TofuOutput) -> Result<PlanExecutionResult> {
        let destroyed_count = parser::parse_destroy_summary(&destroy_output.stdout);

        Ok(PlanExecutionResult {
            success: destroy_output.success,
            output: destroy_output.stdout.clone(),
            errors: if destroy_output.success { vec![] } else { vec![destroy_output.stderr] },
            resources_created: 0,
            resources_updated: 0,
            resources_destroyed: destroyed_count,
        })
    }
}
