//! Common types for the Aspen TUI application.
//!
//! These types represent node and cluster information used by the TUI
//! for display and state management. They are populated from Iroh RPC
//! responses (TUI RPC protocol).

use std::path::PathBuf;

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

// =============================================================================
// SQL Query Types
// =============================================================================

/// Maximum SQL query size (64 KB).
///
/// Tiger Style: Bounded to prevent memory issues.
pub const MAX_SQL_QUERY_SIZE: usize = 65536;

/// Maximum SQL history entries.
///
/// Tiger Style: Bounded to prevent unbounded memory use.
pub const MAX_SQL_HISTORY: usize = 100;

/// SQL history file name.
pub const SQL_HISTORY_FILE: &str = "sql_history.json";

/// SQL query consistency level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SqlConsistency {
    /// Linearizable reads (go through Raft).
    #[default]
    Linearizable,
    /// Stale reads (local SQLite, faster but may be stale).
    Stale,
}

impl SqlConsistency {
    /// Toggle between linearizable and stale.
    pub fn toggle(self) -> Self {
        match self {
            Self::Linearizable => Self::Stale,
            Self::Stale => Self::Linearizable,
        }
    }

    /// Get display string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linearizable => "linearizable",
            Self::Stale => "stale",
        }
    }

    /// Get short display (L/S).
    pub fn short(&self) -> &'static str {
        match self {
            Self::Linearizable => "L",
            Self::Stale => "S",
        }
    }
}

/// SQL view state.
#[derive(Debug, Clone, Default)]
pub struct SqlState {
    /// Current query buffer.
    pub query_buffer: String,
    /// Query history.
    pub history: Vec<String>,
    /// Current history navigation index.
    pub history_index: usize,
    /// Whether navigating history (vs editing).
    pub history_browsing: bool,
    /// Consistency level for queries.
    pub consistency: SqlConsistency,
    /// Last query result.
    pub last_result: Option<SqlQueryResult>,
    /// Vertical scroll position in results.
    pub result_scroll_row: usize,
    /// Horizontal scroll position in results.
    pub result_scroll_col: usize,
    /// Selected row index.
    pub selected_row: usize,
}

impl SqlState {
    /// Add a query to history (avoid duplicates).
    pub fn add_to_history(&mut self, query: String) {
        // Don't add empty queries or duplicates at the end
        if query.trim().is_empty() {
            return;
        }
        if self.history.last().map(|s| s == &query).unwrap_or(false) {
            return;
        }

        self.history.push(query);

        // Tiger Style: Bounded history
        if self.history.len() > MAX_SQL_HISTORY {
            self.history.remove(0);
        }

        // Reset navigation
        self.history_index = self.history.len();
        self.history_browsing = false;
    }

    /// Navigate to previous history entry.
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        if !self.history_browsing {
            // Start browsing from the end
            self.history_browsing = true;
            self.history_index = self.history.len();
        }

        if self.history_index > 0 {
            self.history_index -= 1;
            self.query_buffer = self.history[self.history_index].clone();
        }
    }

    /// Navigate to next history entry.
    pub fn history_next(&mut self) {
        if !self.history_browsing || self.history.is_empty() {
            return;
        }

        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.query_buffer = self.history[self.history_index].clone();
        } else {
            // At the end, clear buffer
            self.history_index = self.history.len();
            self.query_buffer.clear();
            self.history_browsing = false;
        }
    }
}

/// SQL query result for display.
#[derive(Debug, Clone)]
pub struct SqlQueryResult {
    /// Whether the query succeeded.
    pub success: bool,
    /// Column names.
    pub columns: Vec<String>,
    /// Result rows (each row is a vec of string values).
    pub rows: Vec<Vec<String>>,
    /// Total row count.
    pub row_count: u32,
    /// Whether results were truncated.
    pub is_truncated: bool,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Error message if failed.
    pub error: Option<String>,
    /// Calculated column widths for display.
    pub column_widths: Vec<usize>,
}

impl SqlQueryResult {
    /// Create a successful result from RPC response.
    ///
    /// Takes `SqlCellValue` from the RPC response and converts to strings for display.
    /// This uses the PostCard-compatible `SqlCellValue` type instead of `serde_json::Value`.
    pub fn from_response(
        columns: Vec<String>,
        rows: Vec<Vec<aspen_client::SqlCellValue>>,
        row_count: u32,
        is_truncated: bool,
        execution_time_ms: u64,
    ) -> Self {
        // Convert SqlCellValue to strings for display
        let string_rows: Vec<Vec<String>> =
            rows.into_iter().map(|row| row.into_iter().map(|val| val.to_display_string()).collect()).collect();

        // Calculate column widths (min 5, max 40)
        let mut column_widths: Vec<usize> = columns.iter().map(|c| c.len().max(5).min(40)).collect();
        for row in &string_rows {
            for (i, val) in row.iter().enumerate() {
                if i < column_widths.len() {
                    column_widths[i] = column_widths[i].max(val.len().min(40));
                }
            }
        }

        Self {
            success: true,
            columns,
            rows: string_rows,
            row_count,
            is_truncated,
            execution_time_ms,
            error: None,
            column_widths,
        }
    }

    /// Create an error result.
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            columns: vec![],
            rows: vec![],
            row_count: 0,
            is_truncated: false,
            execution_time_ms: 0,
            error: Some(message),
            column_widths: vec![],
        }
    }
}

// =============================================================================
// SQL History Persistence
// =============================================================================

/// Get config directory path: ~/.config/aspen/
fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("aspen"))
}

/// Load SQL history from disk.
///
/// Returns empty vec if file doesn't exist or can't be read.
pub fn load_sql_history() -> Vec<String> {
    let Some(dir) = config_dir() else {
        return vec![];
    };
    let path = dir.join(SQL_HISTORY_FILE);

    std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Save SQL history to disk.
///
/// Creates config directory if it doesn't exist.
/// Silently ignores errors (non-critical operation).
pub fn save_sql_history(history: &[String]) {
    let Some(dir) = config_dir() else {
        return;
    };

    // Create directory if needed
    let _ = std::fs::create_dir_all(&dir);

    let path = dir.join(SQL_HISTORY_FILE);
    let _ = std::fs::write(&path, serde_json::to_string_pretty(history).unwrap_or_default());
}
