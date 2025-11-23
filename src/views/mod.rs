//! View models for presentation layer
//!
//! This module contains view models that decouple domain models from HTML templates.
//! Each view model is responsible for:
//! - Transforming domain data into presentation-friendly format
//! - Handling UI-specific logic (CSS classes, formatting)
//! - Being directly renderable by Askama templates
//!
//! Benefits:
//! - Handlers don't build HTML strings manually
//! - View logic is testable independently
//! - Templates are type-safe and designer-friendly
//! - Clear separation of concerns

pub mod cluster_health;
pub mod queue_stats;
pub mod job_list;
pub mod control_plane_nodes;
pub mod workers;
pub mod error;

// Re-export view types for convenience
pub use cluster_health::ClusterHealthView;
pub use queue_stats::QueueStatsView;
pub use job_list::JobListView;
pub use control_plane_nodes::ControlPlaneNodesView;
pub use workers::WorkersView;
pub use error::ErrorView;
