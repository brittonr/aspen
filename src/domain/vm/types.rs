//! VM domain types
//!
//! These types represent VM concepts at the domain level.
//! For now, we re-export the infrastructure types, but in the future
//! these could become distinct domain types with mapping to/from infrastructure.

// Re-export infrastructure types at the domain level
// This breaks the direct dependency from domain → infrastructure
// by providing a domain-owned API surface
pub use crate::infrastructure::vm::{
    vm_types::{IsolationLevel, JobRequirements, VmConfig, VmInstance, VmMode, VmState},
    JobResult, VmAssignment, VmStats,
};
