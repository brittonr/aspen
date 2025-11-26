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
pub mod compatibility;
pub mod errors;
pub mod event_handlers;
pub mod event_publishers;
pub mod events;
pub mod health_service;
pub mod job;
pub mod job_commands;
pub mod job_lifecycle;
pub mod job_metadata;
pub mod job_queries;
pub mod job_requirements;
pub mod plugins;
pub mod queue;
pub mod state_machine;
#[cfg(feature = "tofu-support")]
pub mod tofu_service;
pub mod types;
pub mod validation;
#[cfg(feature = "vm-backend")]
pub mod vm_management;
#[cfg(feature = "vm-backend")]
pub mod vm_service;
pub mod worker;
pub mod worker_management;


// Re-export key types for convenience
pub use cluster_status::ClusterStatusService;
pub use event_publishers::LoggingEventPublisher;
pub use events::EventPublisher;
pub use health_service::HealthService;
pub use job::{Job, JobStatus};
pub use job_commands::{JobCommandService, JobSubmission};
pub use job_lifecycle::{JobLifecycleService, format_duration, format_time_ago};
pub use job_metadata::JobMetadata;
pub use job_queries::{JobQueryService, JobSortOrder};
pub use job_requirements::{IsolationLevel, JobRequirements};
pub use queue::{HealthStatus, QueueStats};
#[cfg(feature = "tofu-support")]
pub use tofu_service::TofuService;
#[cfg(feature = "vm-backend")]
pub use vm_management::VmManagement;
#[cfg(feature = "vm-backend")]
pub use vm_service::VmService;
pub use worker::{Worker, WorkerHeartbeat, WorkerRegistration, WorkerStats, WorkerStatus, WorkerType};
pub use worker_management::WorkerManagementService;
