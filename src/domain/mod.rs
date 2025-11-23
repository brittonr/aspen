//! Domain layer - Business logic and orchestration
//!
//! This module contains business logic that orchestrates multiple services
//! to implement higher-level operations. It sits between HTTP handlers and
//! infrastructure services (hiqlite, work_queue, iroh).
//!
//! Benefits:
//! - Encapsulates business rules and validation
//! - Reduces coupling between handlers and services
//! - Makes business logic testable independently of HTTP layer
//! - Provides clear API for complex operations

pub mod cluster_status;
pub mod job_lifecycle;

// Re-export key types for convenience
pub use cluster_status::ClusterStatusService;
pub use job_lifecycle::{
    JobLifecycleService, JobSortOrder, JobSubmission, format_duration, format_time_ago,
};
