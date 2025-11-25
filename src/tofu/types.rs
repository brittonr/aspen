//! OpenTofu/Terraform types and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents a Terraform/OpenTofu workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TofuWorkspace {
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub current_state: Option<TofuState>,
    pub state_version: i64,
    pub state_lineage: Option<String>,
    pub lock: Option<WorkspaceLock>,
}

/// Terraform/OpenTofu state format (compatible with tfstate JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TofuState {
    pub version: i32,
    pub terraform_version: String,
    pub serial: i64,
    pub lineage: String,
    pub outputs: HashMap<String, StateOutput>,
    pub resources: Vec<StateResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<StateCheck>>,
}

/// State output value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateOutput {
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub output_type: Option<String>,
    pub sensitive: bool,
}

/// State resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResource {
    pub module: Option<String>,
    pub mode: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    pub provider: String,
    pub instances: Vec<ResourceInstance>,
}

/// Resource instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInstance {
    pub schema_version: i32,
    pub attributes: serde_json::Value,
    pub sensitive_attributes: Option<Vec<String>>,
    pub private: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

/// State check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheck {
    pub address: String,
    pub status: String,
}

/// Workspace lock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLock {
    pub id: String,
    pub operation: String,
    pub info: String,
    pub who: String,
    pub version: String,
    pub created: i64,
    pub path: String,
}

/// Lock request from OpenTofu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockRequest {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Operation")]
    pub operation: String,
    #[serde(rename = "Info")]
    pub info: String,
    #[serde(rename = "Who")]
    pub who: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Created")]
    pub created: String,
    #[serde(rename = "Path")]
    pub path: String,
}

/// Plan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
    Failed,
}

/// Stored plan data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlan {
    pub id: String,
    pub workspace: String,
    pub created_at: i64,
    pub plan_data: Vec<u8>,  // Binary plan format
    pub plan_json: Option<String>,  // JSON representation for inspection
    pub status: PlanStatus,
    pub approved_by: Option<String>,
    pub executed_at: Option<i64>,
}

/// Plan execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionRequest {
    pub plan_id: String,
    pub workspace: String,
    pub auto_approve: bool,
}

/// Plan execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
    pub resources_created: i32,
    pub resources_updated: i32,
    pub resources_destroyed: i32,
}

/// Error types for OpenTofu operations
#[derive(Debug)]
pub enum TofuError {
    WorkspaceLocked,
    StateVersionMismatch { expected: i64, actual: i64 },
    WorkspaceNotFound(String),
    PlanNotFound(String),
    LockNotFound(String),
    InvalidStateFormat(String),
    DatabaseError(anyhow::Error),
    SerializationError(serde_json::Error),
}

impl fmt::Display for TofuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TofuError::WorkspaceLocked => write!(f, "Workspace is locked by another process"),
            TofuError::StateVersionMismatch { expected, actual } => {
                write!(f, "State version mismatch: expected {}, got {}", expected, actual)
            }
            TofuError::WorkspaceNotFound(s) => write!(f, "Workspace not found: {}", s),
            TofuError::PlanNotFound(s) => write!(f, "Plan not found: {}", s),
            TofuError::LockNotFound(s) => write!(f, "Lock not found: {}", s),
            TofuError::InvalidStateFormat(s) => write!(f, "Invalid state format: {}", s),
            TofuError::DatabaseError(e) => write!(f, "Database error: {}", e),
            TofuError::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for TofuError {}

impl From<anyhow::Error> for TofuError {
    fn from(e: anyhow::Error) -> Self {
        TofuError::DatabaseError(e)
    }
}

impl From<serde_json::Error> for TofuError {
    fn from(e: serde_json::Error) -> Self {
        TofuError::SerializationError(e)
    }
}

// Note: anyhow already implements From for any type that implements std::error::Error
// So we don't need an explicit From impl here