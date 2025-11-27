//! OpenTofu Integration Module
//!
//! This module provides OpenTofu/Terraform state backend and plan execution capabilities
//! using the distributed Hiqlite database for state management.

pub mod state_backend;
pub mod plan_executor;
pub mod types;
pub mod executor;
pub mod lock_manager;
pub mod plan_manager;

// Utility modules for plan execution
pub mod validation;
pub mod parser;
pub mod workspace_manager;
pub mod file_operations;

pub use state_backend::TofuStateBackend;
pub use plan_executor::TofuPlanExecutor;
pub use executor::{TofuExecutor, CliTofuExecutor, TofuOutput};
pub use lock_manager::TofuLockManager;
pub use plan_manager::TofuPlanManager;
pub use types::*;