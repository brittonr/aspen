//! VM domain abstractions
//!
//! This module defines the domain-level abstractions for VM management.
//! The infrastructure layer implements these traits, allowing the domain
//! to remain independent of infrastructure details.
//!
//! This follows the Dependency Inversion Principle: high-level modules
//! (domain) should not depend on low-level modules (infrastructure).
//! Both should depend on abstractions (traits defined here).

pub mod management;
pub mod types;

// Re-export for convenience
pub use management::VmManagement;
pub use types::{
    IsolationLevel, JobRequirements, JobResult, VmAssignment, VmConfig, VmInstance, VmMode,
    VmState, VmStats,
};
