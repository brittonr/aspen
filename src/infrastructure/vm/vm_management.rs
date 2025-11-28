//! VmManagement trait re-export
//!
//! The VmManagement trait is now defined in the domain layer (crate::domain::vm::VmManagement)
//! to follow the Dependency Inversion Principle. This module re-exports it for backward
//! compatibility and to make it available to infrastructure implementations.
//!
//! Infrastructure layer provides the concrete implementation (VmManager).

// Re-export the trait from domain
pub use crate::domain::vm::VmManagement;
