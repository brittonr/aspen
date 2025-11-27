//! Tofu/Terraform execution abstraction
//!
//! This module provides a trait-based abstraction for executing OpenTofu/Terraform
//! commands, enabling testability by decoupling business logic from CLI execution.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

/// Output from a tofu command execution
#[derive(Debug, Clone)]
pub struct TofuOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl From<std::process::Output> for TofuOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }
}

/// Trait for executing OpenTofu/Terraform commands
///
/// This abstraction allows business logic to be tested without requiring
/// actual tofu/terraform installations.
#[async_trait]
pub trait TofuExecutor: Send + Sync {
    /// Initialize tofu in the work directory
    async fn init(&self, work_dir: &Path) -> Result<TofuOutput>;

    /// Create a new workspace
    async fn workspace_new(&self, work_dir: &Path, workspace: &str) -> Result<TofuOutput>;

    /// Select a workspace
    async fn workspace_select(&self, work_dir: &Path, workspace: &str) -> Result<TofuOutput>;

    /// Run tofu plan command
    async fn plan(&self, work_dir: &Path) -> Result<TofuOutput>;

    /// Apply a plan file
    async fn apply(&self, work_dir: &Path) -> Result<TofuOutput>;

    /// Destroy infrastructure
    async fn destroy(&self, work_dir: &Path, auto_approve: bool) -> Result<TofuOutput>;
}

/// Real CLI-based tofu executor
///
/// This implementation uses actual OpenTofu/Terraform CLI commands.
pub struct CliTofuExecutor;

impl CliTofuExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliTofuExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TofuExecutor for CliTofuExecutor {
    async fn init(&self, work_dir: &Path) -> Result<TofuOutput> {
        let output = Command::new("tofu")
            .args(&["init", "-backend=false"])
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }

    async fn workspace_new(&self, work_dir: &Path, workspace: &str) -> Result<TofuOutput> {
        let output = Command::new("tofu")
            .arg("workspace")
            .arg("new")
            .arg(workspace)
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }

    async fn workspace_select(&self, work_dir: &Path, workspace: &str) -> Result<TofuOutput> {
        let output = Command::new("tofu")
            .arg("workspace")
            .arg("select")
            .arg(workspace)
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }

    async fn plan(&self, work_dir: &Path) -> Result<TofuOutput> {
        let output = Command::new("tofu")
            .args(&["plan", "-out=tfplan", "-no-color"])
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }

    async fn apply(&self, work_dir: &Path) -> Result<TofuOutput> {
        let output = Command::new("tofu")
            .args(&["apply", "-auto-approve", "-no-color", "tfplan"])
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }

    async fn destroy(&self, work_dir: &Path, auto_approve: bool) -> Result<TofuOutput> {
        let mut args = vec!["destroy", "-no-color"];
        if auto_approve {
            args.push("-auto-approve");
        }

        let output = Command::new("tofu")
            .args(&args)
            .current_dir(work_dir)
            .output()
            .await?;

        Ok(output.into())
    }
}

/// Mock tofu executor for testing
///
/// This implementation simulates tofu behavior without requiring
/// actual CLI installations.
#[cfg(test)]
pub struct MockTofuExecutor {
    pub should_succeed: bool,
    pub plan_output: String,
}

#[cfg(test)]
impl MockTofuExecutor {
    pub fn new(should_succeed: bool) -> Self {
        Self {
            should_succeed,
            plan_output: "Plan: 1 to add, 0 to change, 0 to destroy.".to_string(),
        }
    }

    pub fn with_plan_output(mut self, output: String) -> Self {
        self.plan_output = output;
        self
    }
}

#[cfg(test)]
#[async_trait]
impl TofuExecutor for MockTofuExecutor {
    async fn init(&self, _work_dir: &Path) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: "Terraform has been successfully initialized!".to_string(),
            stderr: if self.should_succeed { String::new() } else { "Init failed".to_string() },
        })
    }

    async fn workspace_new(&self, _work_dir: &Path, workspace: &str) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: format!("Created workspace: {}", workspace),
            stderr: String::new(),
        })
    }

    async fn workspace_select(&self, _work_dir: &Path, workspace: &str) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: format!("Switched to workspace: {}", workspace),
            stderr: String::new(),
        })
    }

    async fn plan(&self, _work_dir: &Path) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: self.plan_output.clone(),
            stderr: if self.should_succeed { String::new() } else { "Plan failed".to_string() },
        })
    }

    async fn apply(&self, _work_dir: &Path) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: "Apply complete! Resources: 1 added, 0 changed, 0 destroyed.".to_string(),
            stderr: if self.should_succeed { String::new() } else { "Apply failed".to_string() },
        })
    }

    async fn destroy(&self, _work_dir: &Path, _auto_approve: bool) -> Result<TofuOutput> {
        Ok(TofuOutput {
            success: self.should_succeed,
            stdout: "Destroy complete! Resources: 1 destroyed.".to_string(),
            stderr: if self.should_succeed { String::new() } else { "Destroy failed".to_string() },
        })
    }
}
