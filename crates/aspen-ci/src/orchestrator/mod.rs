//! Pipeline orchestrator module.
//!
//! This module handles pipeline execution, converting CI configuration
//! into job specs and managing pipeline runs via the aspen-jobs system.

mod pipeline;

pub use pipeline::PipelineContext;
pub use pipeline::PipelineOrchestrator;
pub use pipeline::PipelineOrchestratorConfig;
pub use pipeline::PipelineRun;
pub use pipeline::PipelineStatus;
pub use pipeline::StageStatus;
