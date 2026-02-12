//! Aspen CI/CD Pipeline System
//!
//! This crate provides a CI/CD system built on Aspen's distributed primitives:
//!
//! - **Nickel Configuration**: Type-safe pipeline definitions with contracts
//! - **Gossip Triggers**: Automatic builds on ref updates via iroh-gossip
//! - **Distributed Execution**: Jobs run across the cluster via aspen-jobs
//! - **Artifact Storage**: Build outputs stored in iroh-blobs (P2P)
//!
//! # Example Pipeline Configuration
//!
//! ```nickel
//! # .aspen/ci.ncl
//! {
//!   name = "my-project",
//!   stages = [
//!     {
//!       name = "build",
//!       jobs = [
//!         {
//!           name = "cargo-build",
//!           type = 'shell,
//!           command = "cargo",
//!           args = ["build", "--release"],
//!         },
//!       ],
//!     },
//!   ],
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use aspen_ci::config::load_pipeline_config;
//! use aspen_ci::orchestrator::PipelineOrchestrator;
//!
//! // Load pipeline configuration
//! let config = load_pipeline_config(Path::new(".aspen/ci.ncl"))?;
//!
//! // Execute pipeline
//! let orchestrator = PipelineOrchestrator::new(job_manager, workflow_manager);
//! let run = orchestrator.execute(config, context).await?;
//! ```

#![warn(missing_docs)]
#![allow(clippy::collapsible_if)]

#[cfg(feature = "nickel")]
pub mod adapters;
#[allow(missing_docs)]
pub mod agent;
pub mod checkout;
pub mod config;
pub mod error;
pub mod log_writer;
pub mod orchestrator;
pub mod trigger;
/// Verified pure functions for CI pipeline logic.
pub mod verified;
pub mod workers;

// Re-export main types for convenience
// Re-export adapter types for integration (requires nickel feature)
#[cfg(feature = "nickel")]
pub use adapters::ForgeConfigFetcher;
#[cfg(feature = "nickel")]
pub use adapters::OrchestratorPipelineStarter;
// Re-export checkout functions
pub use checkout::checkout_dir_for_run;
pub use checkout::checkout_repository;
pub use checkout::cleanup_checkout;
pub use checkout::prepare_for_ci_build;
#[cfg(feature = "nickel")]
pub use config::loader::load_pipeline_config;
#[cfg(feature = "nickel")]
pub use config::loader::load_pipeline_config_str;
#[cfg(feature = "nickel")]
pub use config::loader::load_pipeline_config_str_async;
pub use config::types::ArtifactConfig;
pub use config::types::ArtifactStorage;
pub use config::types::IsolationMode;
pub use config::types::JobConfig;
pub use config::types::JobType;
pub use config::types::PipelineConfig;
pub use config::types::Priority;
pub use config::types::StageConfig;
pub use config::types::TriggerConfig;
pub use error::CiError;
// Re-export log writer types
pub use log_writer::CiLogChunk;
pub use log_writer::CiLogCompleteMarker;
pub use log_writer::CiLogWriter;
pub use log_writer::SpawnedLogWriter;
pub use orchestrator::PipelineContext;
pub use orchestrator::PipelineOrchestrator;
pub use orchestrator::PipelineOrchestratorConfig;
pub use orchestrator::PipelineRun;
pub use orchestrator::PipelineStatus;
pub use orchestrator::StageStatus;
// Re-export SNIX trait types for NixBuildWorkerConfig construction
#[cfg(feature = "snix")]
pub use snix_castore::blobservice::BlobService as SnixBlobService;
#[cfg(feature = "snix")]
pub use snix_castore::directoryservice::DirectoryService as SnixDirectoryService;
#[cfg(feature = "snix")]
pub use snix_store::pathinfoservice::PathInfoService as SnixPathInfoService;
#[cfg(feature = "nickel")]
pub use trigger::CiTriggerHandler;
// Re-export trigger traits for external implementations (requires nickel feature)
#[cfg(feature = "nickel")]
pub use trigger::ConfigFetcher;
#[cfg(feature = "nickel")]
pub use trigger::PipelineStarter;
#[cfg(feature = "nickel")]
pub use trigger::TriggerEvent;
#[cfg(feature = "nickel")]
pub use trigger::TriggerService;
#[cfg(feature = "nickel")]
pub use trigger::TriggerServiceConfig;
pub use workers::CloudHypervisorPayload;
pub use workers::CloudHypervisorWorker;
pub use workers::CloudHypervisorWorkerConfig;
pub use workers::LocalExecutorPayload;
pub use workers::LocalExecutorWorker;
pub use workers::LocalExecutorWorkerConfig;
pub use workers::NetworkMode;
#[cfg(feature = "snix")]
pub use workers::NixBuildPayload;
#[cfg(feature = "snix")]
pub use workers::NixBuildWorker;
#[cfg(feature = "snix")]
pub use workers::NixBuildWorkerConfig;
