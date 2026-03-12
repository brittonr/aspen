//! Pipeline orchestrator module.
//!
//! This module handles pipeline execution, converting CI configuration
//! into job specs and managing pipeline runs via the aspen-jobs system.
//!
//! # Recovery
//!
//! The [`recovery`] module provides support for recovering orphaned pipelines
//! after a leader failover. Use [`OrphanedPipelineRecovery`] to scan for and
//! recover pipelines that were in progress when the previous leader crashed.

pub mod deploy_executor;
pub mod deploy_monitor;
mod pipeline;
mod recovery;

pub use deploy_executor::DeployArtifact;
pub use deploy_executor::DeployDispatcher;
pub use deploy_executor::DeployExecutor;
pub use deploy_executor::DeployInitResult;
pub use deploy_executor::DeployJobParams;
pub use deploy_executor::DeployJobResult;
pub use deploy_executor::DeployNodeStatus;
pub use deploy_executor::DeployRequest;
pub use deploy_executor::DeployStatusResult;
pub use deploy_monitor::DeployJobInfo;
pub use deploy_monitor::DeployStageInfo;
pub use deploy_monitor::extract_deploy_stages;
pub use deploy_monitor::extract_deploy_stages_for_ref;
pub use deploy_monitor::spawn_deploy_monitor;
pub use pipeline::PipelineContext;
pub use pipeline::PipelineOrchestrator;
pub use pipeline::PipelineOrchestratorConfig;
pub use pipeline::PipelineRun;
pub use pipeline::PipelineStatus;
pub use pipeline::RefStatus;
pub use pipeline::StageStatus;
pub use recovery::OrphanedPipelineRecovery;
pub use recovery::RecoveryAction;
pub use recovery::RecoveryResult;
