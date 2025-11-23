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
pub mod event_publishers;
pub mod events;
pub mod health_service;
pub mod job_commands;
pub mod job_lifecycle;
pub mod job_queries;
pub mod types;

// Re-export key types for convenience
pub use cluster_status::ClusterStatusService;
pub use event_publishers::{InMemoryEventPublisher, LoggingEventPublisher, NoOpEventPublisher};
pub use events::{DomainEvent, EventPublisher};
pub use health_service::HealthService;
pub use job_commands::{JobCommandService, JobSubmission};
pub use job_lifecycle::{JobLifecycleService, format_duration, format_time_ago};
pub use job_queries::{JobQueryService, JobSortOrder, EnrichedJob};
pub use types::{Job, JobStatus, QueueStats};
