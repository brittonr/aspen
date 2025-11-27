//! TofuService - Unified domain service for OpenTofu state and plan operations
//!
//! This service provides a single access point for all OpenTofu/Terraform operations,
//! combining state backend and plan execution capabilities.
//!
//! Unlike the lower-level TofuStateBackend and TofuPlanExecutor, this service is:
//! - Created once at startup with all dependencies injected
//! - Shared across all request handlers via Arc
//! - Follows the CQRS pattern with other domain services
//! - Sits in the domain layer between HTTP handlers and infrastructure services

use anyhow::Result;
use std::sync::Arc;
use std::path::{Path, PathBuf};

use crate::{
    adapters::ExecutionRegistry,
    hiqlite::HiqliteService,
    tofu::{
        TofuStateBackend,
        TofuPlanExecutor,
        types::*,
    },
};

/// Unified service for all OpenTofu/Terraform operations
///
/// This domain service combines state management and plan execution capabilities,
/// providing a consistent interface for OpenTofu operations throughout the application.
/// It is created once at startup and shared across all request handlers.
///
/// # Design
///
/// - Wraps TofuStateBackend for state operations
/// - Wraps TofuPlanExecutor for plan execution operations
/// - Eliminates per-request service creation (anti-pattern that was repeated in handlers)
/// - Arc-friendly for sharing across handlers
/// - Follows the same pattern as JobCommandService and JobQueryService
#[derive(Clone)]
pub struct TofuService {
    state_backend: Arc<TofuStateBackend>,
    plan_executor: Arc<TofuPlanExecutor>,
}

impl TofuService {
    /// Create a new TofuService
    ///
    /// Creates a new domain service for OpenTofu operations. This should be called once
    /// at startup and the instance shared across all handlers via Arc.
    ///
    /// # Arguments
    /// * `_execution_registry` - Registry for execution backends (reserved for future use)
    /// * `hiqlite` - Hiqlite database service
    /// * `work_dir` - Working directory for plan execution
    pub fn new(
        _execution_registry: Arc<ExecutionRegistry>,
        hiqlite: Arc<HiqliteService>,
        work_dir: PathBuf,
    ) -> Self {
        let state_backend = Arc::new(TofuStateBackend::new(hiqlite.clone()));
        let plan_executor = Arc::new(TofuPlanExecutor::new(
            hiqlite.clone(),
            work_dir,
        ));

        Self {
            state_backend,
            plan_executor,
        }
    }

    /// Create a new TofuService from pre-built backend components
    ///
    /// This is an alternative constructor for testing or advanced use cases.
    ///
    /// # Arguments
    /// * `state_backend` - Pre-configured TofuStateBackend
    /// * `plan_executor` - Pre-configured TofuPlanExecutor
    pub fn from_components(
        state_backend: Arc<TofuStateBackend>,
        plan_executor: Arc<TofuPlanExecutor>,
    ) -> Self {
        Self {
            state_backend,
            plan_executor,
        }
    }

    // === State Operations ===

    /// Get the current state for a workspace
    ///
    /// Returns the latest Terraform/OpenTofu state if one exists.
    pub async fn get_state(&self, workspace: &str) -> Result<Option<TofuState>> {
        self.state_backend.get_state(workspace).await
    }

    /// Update the state for a workspace
    ///
    /// Stores a new state version and maintains version history.
    /// Optionally validates the expected version to detect conflicts.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `state` - New state to store
    /// * `expected_version` - Optional version for optimistic concurrency control
    pub async fn put_state(
        &self,
        workspace: &str,
        state: TofuState,
        expected_version: Option<i64>,
    ) -> Result<()> {
        self.state_backend.put_state(workspace, state, expected_version).await
    }

    /// Get lock information for a workspace
    ///
    /// Returns the current lock if the workspace is locked.
    pub async fn get_lock_info(&self, workspace: &str) -> Result<Option<WorkspaceLock>> {
        self.state_backend.get_lock_info(workspace).await
    }

    /// Check if a workspace is currently locked
    ///
    /// Returns true if the workspace has an active lock.
    pub async fn is_locked(&self, workspace: &str) -> Result<bool> {
        self.state_backend.is_locked(workspace).await
    }

    /// Lock a workspace
    ///
    /// Prevents concurrent modifications to the workspace state.
    /// Returns error if workspace is already locked.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `lock_request` - Lock information from OpenTofu
    pub async fn lock_workspace(&self, workspace: &str, lock_request: LockRequest) -> Result<()> {
        self.state_backend.lock_workspace(workspace, lock_request).await
    }

    /// Unlock a workspace
    ///
    /// Releases the lock on a workspace, allowing modifications.
    /// Validates that the lock ID matches before unlocking.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `lock_id` - Expected lock ID (must match current lock)
    pub async fn unlock_workspace(&self, workspace: &str, lock_id: &str) -> Result<()> {
        self.state_backend.unlock_workspace(workspace, lock_id).await
    }

    /// Force unlock a workspace (admin operation)
    ///
    /// Releases any lock on a workspace without validation.
    /// Should only be used by administrators in emergency situations.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    pub async fn force_unlock_workspace(&self, workspace: &str) -> Result<()> {
        self.state_backend.force_unlock_workspace(workspace).await
    }

    /// List all workspaces
    ///
    /// Returns the names of all existing workspaces.
    pub async fn list_workspaces(&self) -> Result<Vec<String>> {
        self.state_backend.list_workspaces().await
    }

    /// Delete a workspace
    ///
    /// Removes a workspace and all its associated state history.
    /// Workspace must not be locked.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    pub async fn delete_workspace(&self, workspace: &str) -> Result<()> {
        self.state_backend.delete_workspace(workspace).await
    }

    /// Get state history for a workspace
    ///
    /// Returns historical state versions for a workspace.
    /// Useful for auditing and rollback operations.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `limit` - Optional limit on number of history entries to return
    pub async fn get_state_history(
        &self,
        workspace: &str,
        limit: Option<i64>,
    ) -> Result<Vec<(i64, TofuState, i64)>> {
        self.state_backend.get_state_history(workspace, limit).await
    }

    /// Rollback to a previous state version
    ///
    /// Restores a workspace to a previous state version.
    /// All intermediate versions are preserved in history.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `version` - Target version to rollback to
    pub async fn rollback_state(&self, workspace: &str, version: i64) -> Result<()> {
        self.state_backend.rollback_state(workspace, version).await
    }

    /// List plans for a workspace
    ///
    /// Returns all stored plans (both pending and executed) for a workspace.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    pub async fn list_plans(&self, workspace: &str) -> Result<Vec<StoredPlan>> {
        self.state_backend.list_plans(workspace).await
    }

    // === Plan Execution Operations ===

    /// Execute a plan from a Terraform configuration directory
    ///
    /// Creates and optionally applies a new plan for the given configuration.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `config_dir` - Path to Terraform configuration directory
    /// * `auto_approve` - If true, automatically applies the plan
    pub async fn execute_plan(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        self.plan_executor.execute_plan(workspace, config_dir, auto_approve).await
    }

    /// Apply a stored plan
    ///
    /// Executes a previously created plan that has been approved.
    ///
    /// # Arguments
    /// * `plan_id` - ID of the plan to apply
    /// * `approver` - User ID of the person approving the application
    pub async fn apply_stored_plan(&self, plan_id: &str, approver: &str) -> Result<PlanExecutionResult> {
        self.plan_executor.apply_stored_plan(plan_id, approver).await
    }

    /// Destroy infrastructure
    ///
    /// Executes a destroy operation for all resources in the workspace.
    ///
    /// # Arguments
    /// * `workspace` - Target workspace name
    /// * `config_dir` - Path to Terraform configuration directory
    /// * `auto_approve` - If true, automatically destroys without confirmation
    pub async fn destroy(
        &self,
        workspace: &str,
        config_dir: &Path,
        auto_approve: bool,
    ) -> Result<PlanExecutionResult> {
        self.plan_executor.destroy(workspace, config_dir, auto_approve).await
    }
}
