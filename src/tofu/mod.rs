//! OpenTofu Integration Module
//!
//! This module provides OpenTofu/Terraform state backend and plan execution capabilities
//! using the distributed Hiqlite database for state management.

pub mod state_backend;
pub mod plan_executor;
pub mod types;
pub mod executor;

// Utility modules for plan execution
pub mod validation;
pub mod parser;

pub use state_backend::TofuStateBackend;
pub use plan_executor::TofuPlanExecutor;
pub use executor::{TofuExecutor, CliTofuExecutor, TofuOutput};
pub use types::*;