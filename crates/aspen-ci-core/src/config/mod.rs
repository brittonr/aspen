//! CI pipeline configuration types.
//!
//! This module provides the configuration types for CI pipelines. These types
//! are deserialized from Nickel configuration files after validation against
//! the CI schema contracts.
//!
//! # Types
//!
//! - [`PipelineConfig`] - Complete pipeline configuration
//! - [`StageConfig`] - Configuration for a pipeline stage
//! - [`JobConfig`] - Configuration for a single job within a stage
//! - [`TriggerConfig`] - Automatic pipeline trigger configuration
//! - [`ArtifactConfig`] - Artifact storage configuration
//!
//! # Enums
//!
//! - [`JobType`] - Job execution type (Nix, Shell, VM)
//! - [`IsolationMode`] - Job isolation mode (NixSandbox, VM, None)
//! - [`ArtifactStorage`] - Artifact storage backend (Blobs, Local, None)
//! - [`Priority`] - Pipeline priority level (High, Normal, Low)

mod types;

pub use types::ArtifactConfig;
pub use types::ArtifactStorage;
pub use types::CI_JOB_TYPE_DEPLOY;
pub use types::CI_JOB_TYPE_NIX;
pub use types::CI_JOB_TYPE_SHELL;
pub use types::CI_JOB_TYPE_VM;
pub use types::IsolationMode;
pub use types::JobConfig;
pub use types::JobType;
pub use types::PipelineConfig;
pub use types::Priority;
pub use types::StageConfig;
pub use types::TriggerConfig;
pub use types::job_type_route;
pub use types::retry_count_to_jobs_policy;
pub use types::to_jobs_priority;
