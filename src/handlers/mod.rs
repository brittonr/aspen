//! HTTP handlers organized by feature
//!
//! This module contains all HTTP request handlers organized into logical groups:
//! - `dashboard` - Real-time cluster monitoring UI with HTMX
//! - `jobs` - Legacy job management UI
//! - `queue` - REST API for work queue operations (used by workers)

pub mod dashboard;
pub mod jobs;
pub mod queue;

// Re-export handlers for convenient access
pub use dashboard::{
    dashboard,
    dashboard_cluster_health,
    dashboard_control_plane_nodes,
    dashboard_queue_stats,
    dashboard_recent_jobs,
    dashboard_submit_job,
    dashboard_workers,
};

pub use jobs::{
    handle_assets,
    index,
    list,
    new_job,
    timeout,
    ui_update,
};

pub use queue::{
    hiqlite_health,
    queue_claim,
    queue_list,
    queue_publish,
    queue_stats,
    queue_update_status,
};
