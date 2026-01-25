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

mod pipeline;
mod recovery;

pub use pipeline::PipelineContext;
pub use pipeline::PipelineOrchestrator;
pub use pipeline::PipelineOrchestratorConfig;
pub use pipeline::PipelineRun;
pub use pipeline::PipelineStatus;
pub use pipeline::StageStatus;
pub use recovery::OrphanedPipelineRecovery;
pub use recovery::RecoveryAction;
pub use recovery::RecoveryResult;
