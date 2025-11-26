//! Job requirements and constraints
//!
//! This module defines domain types for job requirements that are independent
//! of infrastructure concerns like worker assignment.

use serde::{Deserialize, Serialize};
use super::types::WorkerType;

/// Job requirements specify constraints and resource needs
///
/// These are domain concepts that describe what a job needs,
/// separate from infrastructure concerns about how it's assigned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobRequirements {
    /// Worker types that can execute this job (empty = any worker type)
    pub compatible_worker_types: Vec<WorkerType>,
    /// Isolation level required (for VM-based execution)
    pub isolation_level: Option<IsolationLevel>,
    /// Memory required in megabytes
    pub memory_mb: Option<u32>,
    /// Virtual CPUs required
    pub vcpus: Option<u32>,
}

impl Default for JobRequirements {
    fn default() -> Self {
        Self {
            compatible_worker_types: Vec::new(),
            isolation_level: None,
            memory_mb: None,
            vcpus: None,
        }
    }
}

impl JobRequirements {
    /// Create requirements with no constraints (any worker can execute)
    pub fn any() -> Self {
        Self::default()
    }

    /// Create requirements for specific worker types
    pub fn for_worker_types(worker_types: Vec<WorkerType>) -> Self {
        Self {
            compatible_worker_types: worker_types,
            ..Default::default()
        }
    }

    /// Check if this job is compatible with a given worker type
    pub fn is_compatible_with(&self, worker_type: WorkerType) -> bool {
        if self.compatible_worker_types.is_empty() {
            return true;
        }
        self.compatible_worker_types.contains(&worker_type)
    }

    /// Check if this job has any worker type constraints
    pub fn has_worker_type_constraints(&self) -> bool {
        !self.compatible_worker_types.is_empty()
    }
}

/// Isolation level for job execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IsolationLevel {
    /// No isolation (shared process)
    None,
    /// Process-level isolation
    Process,
    /// Container isolation
    Container,
    /// VM-level isolation (strongest)
    Vm,
}

impl std::fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationLevel::None => write!(f, "none"),
            IsolationLevel::Process => write!(f, "process"),
            IsolationLevel::Container => write!(f, "container"),
            IsolationLevel::Vm => write!(f, "vm"),
        }
    }
}

impl std::str::FromStr for IsolationLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(IsolationLevel::None),
            "process" => Ok(IsolationLevel::Process),
            "container" => Ok(IsolationLevel::Container),
            "vm" => Ok(IsolationLevel::Vm),
            _ => Err(anyhow::anyhow!("Invalid isolation level: {}", s)),
        }
    }
}
