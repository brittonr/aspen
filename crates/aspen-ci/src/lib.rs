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
//!
//! # Crate Structure
//!
//! This crate re-exports types from specialized executor crates:
//!
//! - `aspen-ci-core`: Pipeline config types, errors, verified functions, log writer types
//! - `aspen-ci-executor-shell`: LocalExecutorWorker, ResourceLimiter, common utilities
//! - `aspen-ci-executor-vm`: CloudHypervisorWorker, VmPool, VM management
//! - `aspen-ci-executor-nix`: NixBuildWorker for Nix builds

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
/// CI build workers.
pub mod workers;

// ============================================================================
// Re-exports from aspen-ci-core
// ============================================================================

// Re-export core types for convenience
// ============================================================================
// Re-exports from this crate's modules
// ============================================================================

// Re-export adapter types for integration (requires nickel feature)
#[cfg(feature = "nickel")]
pub use adapters::ForgeConfigFetcher;
#[cfg(feature = "nickel")]
pub use adapters::OrchestratorPipelineStarter;
pub use aspen_ci_core::ArtifactConfig;
pub use aspen_ci_core::ArtifactStorage;
pub use aspen_ci_core::CiCoreError;
// Re-export log chunk types from core
pub use aspen_ci_core::CiLogChunk;
pub use aspen_ci_core::CiLogCompleteMarker;
pub use aspen_ci_core::IsolationMode;
pub use aspen_ci_core::JobConfig;
pub use aspen_ci_core::JobType;
pub use aspen_ci_core::PipelineConfig;
pub use aspen_ci_core::Priority;
pub use aspen_ci_core::StageConfig;
// Re-export verified functions from core
pub use aspen_ci_core::StageValidationError;
pub use aspen_ci_core::TriggerConfig;
pub use aspen_ci_core::any_path_matches;
pub use aspen_ci_core::are_dependencies_met;
pub use aspen_ci_core::are_resource_limits_valid;
pub use aspen_ci_core::check_pipeline_limits;
pub use aspen_ci_core::compute_deadline_ms;
pub use aspen_ci_core::compute_effective_cpu_weight;
pub use aspen_ci_core::compute_effective_memory_limit;
pub use aspen_ci_core::compute_effective_pid_limit;
pub use aspen_ci_core::compute_effective_timeout_secs;
pub use aspen_ci_core::compute_memory_high_watermark;
pub use aspen_ci_core::compute_pipeline_timeout_secs;
pub use aspen_ci_core::compute_stage_order;
pub use aspen_ci_core::count_total_jobs;
pub use aspen_ci_core::extract_branch_name;
pub use aspen_ci_core::extract_tag_name;
pub use aspen_ci_core::find_missing_dependency;
pub use aspen_ci_core::find_ready_stages;
pub use aspen_ci_core::format_bytes_for_cgroup;
pub use aspen_ci_core::has_self_dependency;
pub use aspen_ci_core::is_branch_ref;
pub use aspen_ci_core::is_deadline_exceeded;
pub use aspen_ci_core::is_tag_ref;
pub use aspen_ci_core::ms_to_secs;
pub use aspen_ci_core::path_matches_pattern;
pub use aspen_ci_core::ref_matches_any_pattern;
pub use aspen_ci_core::ref_matches_pattern;
pub use aspen_ci_core::remaining_time_ms;
pub use aspen_ci_core::secs_to_ms;
// ============================================================================
// Re-exports from aspen-ci-executor-nix (nix-executor feature)
// ============================================================================
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildPayload;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildWorker;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::NixBuildWorkerConfig;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::UploadedStorePath;
#[cfg(feature = "nix-executor")]
pub use aspen_ci_executor_nix::UploadedStorePathSnix;
// ============================================================================
// Re-exports from aspen-ci-executor-shell (shell-executor feature)
// ============================================================================
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ArtifactCollectionResult;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ArtifactUploadResult;
// Cache proxy re-exports (requires nix-cache-proxy feature)
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::CacheProxy;
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::CacheProxyError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::CollectedArtifact;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ExecutionRequest;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ExecutionResult;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::Executor;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorPayload;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorWorker;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LocalExecutorWorkerConfig;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::LogMessage;
#[cfg(all(feature = "shell-executor", feature = "nix-cache-proxy"))]
pub use aspen_ci_executor_shell::NIX_CACHE_ALPN;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::NetworkIsolation;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::OutputRef;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimiter;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimiterError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::ResourceLimits;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::UploadedArtifact;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::WorkerUtilError;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::collect_artifacts;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::create_limiter;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::create_source_archive;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::seed_workspace_from_blob;
#[cfg(feature = "shell-executor")]
pub use aspen_ci_executor_shell::upload_artifacts_to_blob_store;
// ============================================================================
// Re-exports from aspen-ci-executor-vm (plugins-vm feature)
// ============================================================================
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::CloudHypervisorError;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::CloudHypervisorPayload;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::CloudHypervisorWorker;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::CloudHypervisorWorkerConfig;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::ManagedCiVm;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::NetworkMode;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::PoolStatus;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::SharedVm;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::VmApiClient;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::VmPool;
#[cfg(feature = "plugins-vm")]
pub use aspen_ci_executor_vm::VmState;
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
// Re-export priority conversion function
pub use config::types::to_jobs_priority;
pub use error::CiError;
// Re-export log writer types (the actual writer implementation, not just types)
pub use log_writer::CiLogWriter;
pub use log_writer::SpawnedLogWriter;
pub use orchestrator::PipelineContext;
pub use orchestrator::PipelineOrchestrator;
pub use orchestrator::PipelineOrchestratorConfig;
pub use orchestrator::PipelineRun;
pub use orchestrator::PipelineStatus;
pub use orchestrator::StageStatus;
// ============================================================================
// Re-exports from other Aspen crates
// ============================================================================

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
