//! Execution handle abstraction for tracking executions across different backends

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of execution handle, indicating which backend is managing it
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExecutionHandleType {
    /// VM execution (Cloud Hypervisor process)
    Vm(String),
    /// Flawless WASM workflow
    Flawless(String),
    /// Local OS process
    LocalProcess(u32),
    /// Mock execution for testing
    Mock(String),
    /// Custom backend type
    Custom {
        backend_type: String,
        handle_id: String,
    },
}

impl fmt::Display for ExecutionHandleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionHandleType::Vm(id) => write!(f, "vm:{}", id),
            ExecutionHandleType::Flawless(id) => write!(f, "flawless:{}", id),
            ExecutionHandleType::LocalProcess(pid) => write!(f, "process:{}", pid),
            ExecutionHandleType::Mock(id) => write!(f, "mock:{}", id),
            ExecutionHandleType::Custom {
                backend_type,
                handle_id,
            } => write!(f, "{}:{}", backend_type, handle_id),
        }
    }
}

/// Handle for tracking and controlling an execution instance
///
/// This provides a unified way to reference executions across different backends.
/// Each backend translates this handle to its internal representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExecutionHandle {
    /// Unique identifier for this execution
    pub id: String,
    /// Type of execution (indicates which backend manages it)
    pub handle_type: ExecutionHandleType,
    /// Job ID this execution is associated with
    pub job_id: String,
    /// Backend that's managing this execution
    pub backend_name: String,
}

impl ExecutionHandle {
    /// Create a new VM execution handle
    pub fn vm(id: String, job_id: String) -> Self {
        Self {
            id: id.clone(),
            handle_type: ExecutionHandleType::Vm(id),
            job_id,
            backend_name: "vm".to_string(),
        }
    }

    /// Create a new Flawless execution handle
    pub fn flawless(id: String, job_id: String) -> Self {
        Self {
            id: id.clone(),
            handle_type: ExecutionHandleType::Flawless(id),
            job_id,
            backend_name: "flawless".to_string(),
        }
    }

    /// Create a new local process execution handle
    pub fn local_process(pid: u32, job_id: String) -> Self {
        Self {
            id: format!("pid-{}", pid),
            handle_type: ExecutionHandleType::LocalProcess(pid),
            job_id,
            backend_name: "local".to_string(),
        }
    }

    /// Create a new mock execution handle
    pub fn mock(id: String, job_id: String) -> Self {
        Self {
            id: id.clone(),
            handle_type: ExecutionHandleType::Mock(id),
            job_id,
            backend_name: "mock".to_string(),
        }
    }

    /// Create a custom execution handle for extensibility
    pub fn custom(backend_type: String, handle_id: String, job_id: String) -> Self {
        Self {
            id: format!("{}-{}", backend_type, handle_id),
            handle_type: ExecutionHandleType::Custom {
                backend_type: backend_type.clone(),
                handle_id,
            },
            job_id,
            backend_name: backend_type,
        }
    }

    /// Check if this handle is for a VM execution
    pub fn is_vm(&self) -> bool {
        matches!(self.handle_type, ExecutionHandleType::Vm(_))
    }

    /// Check if this handle is for a Flawless execution
    pub fn is_flawless(&self) -> bool {
        matches!(self.handle_type, ExecutionHandleType::Flawless(_))
    }

    /// Check if this handle is for a local process
    pub fn is_local_process(&self) -> bool {
        matches!(self.handle_type, ExecutionHandleType::LocalProcess(_))
    }

    /// Check if this handle is for a mock execution
    pub fn is_mock(&self) -> bool {
        matches!(self.handle_type, ExecutionHandleType::Mock(_))
    }

    /// Get the backend-specific identifier from the handle
    pub fn get_backend_id(&self) -> String {
        match &self.handle_type {
            ExecutionHandleType::Vm(id) => id.clone(),
            ExecutionHandleType::Flawless(id) => id.clone(),
            ExecutionHandleType::LocalProcess(pid) => pid.to_string(),
            ExecutionHandleType::Mock(id) => id.clone(),
            ExecutionHandleType::Custom { handle_id, .. } => handle_id.clone(),
        }
    }
}

impl fmt::Display for ExecutionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (job: {})", self.handle_type, self.job_id)
    }
}
