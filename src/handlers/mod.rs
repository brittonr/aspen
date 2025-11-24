//! HTTP handlers organized by feature
//!
//! This module contains all HTTP request handlers organized into logical groups:
//! - `dashboard` - Real-time cluster monitoring UI with HTMX
//! - `queue` - REST API for work queue operations (used by workers)

pub mod dashboard;
pub mod queue;

// Re-export handlers for convenient access

