//! OpenTofu Integration Module
//!
//! This module provides OpenTofu/Terraform state backend and plan execution capabilities
//! using the distributed Hiqlite database for state management.

pub mod state_backend;
pub mod plan_executor;
pub mod types;

pub use state_backend::TofuStateBackend;
pub use plan_executor::TofuPlanExecutor;
pub use types::*;