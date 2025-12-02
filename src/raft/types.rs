use std::fmt;

use openraft::{BasicNode, declare_raft_types};
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

/// Application-level requests replicated through Raft.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppRequest {
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },
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
