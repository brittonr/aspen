//! Type definitions for Raft consensus configuration.
//!
//! This module defines the core type configuration for openraft, specifying
//! the concrete types used for nodes, requests, responses, and storage.
//!
//! # Type Configuration
//!
//! - **NodeId**: `u64` - Unique identifier for cluster nodes
//! - **Node**: `BasicNode` - Simple node representation (no custom metadata)
//! - **AppRequest**: Application-level write commands (Set, SetMulti)
//! - **AppResponse**: Application-level read/write responses
//!
//! # Tiger Style
//!
//! - Explicitly sized types: `u64` for NodeId (not usize for portability)
//! - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant

use std::fmt;

use openraft::{BasicNode, declare_raft_types};
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

/// Application-level requests replicated through Raft.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppRequest {
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },
    Delete { key: String },
    DeleteMulti { keys: Vec<String> },
}

impl fmt::Display for AppRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppRequest::Set { key, value } => write!(f, "Set {{ key: {key}, value: {value} }}"),
            AppRequest::SetMulti { pairs } => {
                write!(f, "SetMulti {{ pairs: [")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "({k}, {v})")?;
                }
                write!(f, "] }}")
            }
            AppRequest::Delete { key } => write!(f, "Delete {{ key: {key} }}"),
            AppRequest::DeleteMulti { keys } => {
                write!(f, "DeleteMulti {{ keys: [")?;
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}")?;
                }
                write!(f, "] }}")
            }
        }
    }
}

/// Response returned to HTTP clients after applying a request.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppResponse {
    pub value: Option<String>,
}

declare_raft_types!(
    /// Declare type config used by Aspen's embedded Raft node.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = BasicNode,
);
