//! HTTP handlers organized by feature
//!
//! This module contains all HTTP request handlers organized into logical groups:
//! - `dashboard` - Real-time cluster monitoring UI with HTMX
//! - `queue` - REST API for work queue operations (used by workers)
//! - `worker` - REST API for worker management (registration, heartbeats)
//! - `view_renderer` - Utilities for rendering templates with error handling

pub mod dashboard;
pub mod queue;
pub mod worker;
pub(crate) mod view_renderer;

// Re-export handlers for convenient access

