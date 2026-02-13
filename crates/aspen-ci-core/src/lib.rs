//! Core CI pipeline types and pure functions for Aspen.
//!
//! This crate provides the foundational types and pure functions for the Aspen
//! CI/CD system. It is designed to be lightweight with minimal dependencies,
//! allowing other crates to depend on it without pulling in heavy dependencies
//! like Nickel or async runtimes.
//!
//! # Modules
//!
//! - [`config`] - Pipeline configuration types (PipelineConfig, StageConfig, JobConfig)
//! - [`error`] - Core CI error types
//! - [`verified`] - Pure functions for pipeline logic (timeout, trigger, resource, pipeline)
//! - [`log_writer`] - CI log streaming types
//!
//! # Design Philosophy
//!
//! This crate follows the Functional Core, Imperative Shell pattern:
//! - Pure functions in `verified/` module are deterministic and testable
//! - Types are serializable and schema-aware for configuration validation
//! - No async code or I/O operations
//!
//! # Tiger Style
//!
//! - All time values in milliseconds (u64)
//! - Saturating arithmetic prevents overflow
//! - Fixed limits on data structures

pub mod config;
pub mod error;
pub mod log_writer;
pub mod verified;

// Re-export commonly used types at crate root
pub use config::ArtifactConfig;
pub use config::ArtifactStorage;
pub use config::IsolationMode;
pub use config::JobConfig;
pub use config::JobType;
pub use config::PipelineConfig;
pub use config::Priority;
pub use config::StageConfig;
pub use config::TriggerConfig;
pub use error::CiCoreError;
pub use error::Result;
pub use log_writer::CiLogChunk;
pub use log_writer::CiLogCompleteMarker;
pub use verified::{
    // Pipeline validation
    StageValidationError,
    // Trigger matching
    any_path_matches,
    are_dependencies_met,
    // Resource limits
    are_resource_limits_valid,
    check_pipeline_limits,
    // Timeout calculations
    compute_deadline_ms,
    compute_effective_cpu_weight,
    compute_effective_memory_limit,
    compute_effective_pid_limit,
    compute_effective_timeout_secs,
    compute_memory_high_watermark,
    compute_pipeline_timeout_secs,
    compute_stage_order,
    count_total_jobs,
    extract_branch_name,
    extract_tag_name,
    find_missing_dependency,
    find_ready_stages,
    format_bytes_for_cgroup,
    has_self_dependency,
    is_branch_ref,
    is_deadline_exceeded,
    is_tag_ref,
    ms_to_secs,
    path_matches_pattern,
    ref_matches_any_pattern,
    ref_matches_pattern,
    remaining_time_ms,
    secs_to_ms,
};
