//! CI/CD pipeline RPC handlers.
//!
//! Handles pipeline triggering, status queries, and repository watching
//! through the aspen-ci orchestrator and trigger service.

pub mod artifacts;
pub mod helpers;
pub mod logs;
pub mod pipeline;
#[cfg(feature = "forge")]
pub mod watch;
