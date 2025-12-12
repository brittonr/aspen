//! Common types for the Aspen TUI application.
//!
//! These types represent node and cluster information used by the TUI
//! for display and state management. They are populated from Iroh RPC
//! responses (TUI RPC protocol).

/// Node health status.
///
/// Represents the health state of a node for display in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeStatus {
    /// Node is healthy and responsive.
    Healthy,
    /// Node is responding but has warnings.
    Degraded,
    /// Node is unhealthy or unreachable.
    Unhealthy,
    /// Node status is unknown.
    #[default]
    Unknown,
}

impl NodeStatus {
    /// Parse a status string from RPC response.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            "unhealthy" => Self::Unhealthy,
            _ => Self::Unknown,
        }
    }
}

/// Information about a single node.
///
/// Aggregates node state from various RPC responses for TUI display.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node identifier.
    pub node_id: u64,
    /// Current health status.
    pub status: NodeStatus,
    /// Whether this node is the Raft leader.
    pub is_leader: bool,
    /// Last applied log index.
    pub last_applied_index: Option<u64>,
    /// Current Raft term.
    pub current_term: Option<u64>,
    /// Uptime in seconds.
    pub uptime_secs: Option<u64>,
    /// Address (Iroh endpoint or HTTP).
    pub addr: String,
}

impl Default for NodeInfo {
    fn default() -> Self {
        Self {
            node_id: 0,
            status: NodeStatus::Unknown,
            is_leader: false,
            last_applied_index: None,
            current_term: None,
            uptime_secs: None,
            addr: String::new(),
        }
    }
}

/// Aggregated cluster metrics.
///
/// Combines metrics from multiple nodes for cluster-wide view.
#[derive(Debug, Clone, Default)]
pub struct ClusterMetrics {
    /// Current leader node ID.
    pub leader: Option<u64>,
    /// Current Raft term.
    pub term: u64,
    /// Number of nodes in cluster.
    pub node_count: usize,
    /// Last log index across cluster.
    pub last_log_index: Option<u64>,
    /// Last applied index across cluster.
    pub last_applied_index: Option<u64>,
}
