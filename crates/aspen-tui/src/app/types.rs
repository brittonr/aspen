//! Core types for the TUI application.

/// Active view/tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveView {
    /// Cluster overview with node status.
    #[default]
    Cluster,
    /// Detailed node metrics.
    Metrics,
    /// Key-value store operations.
    KeyValue,
    /// Vault browser with namespace navigation.
    Vaults,
    /// SQL query interface.
    Sql,
    /// Log viewer.
    Logs,
    /// Job queue monitoring.
    Jobs,
    /// Worker pool monitoring.
    Workers,
    /// CI pipeline monitoring.
    Ci,
    /// Help screen.
    Help,
}

impl ActiveView {
    /// Get next view in cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Cluster => Self::Metrics,
            Self::Metrics => Self::KeyValue,
            Self::KeyValue => Self::Vaults,
            Self::Vaults => Self::Sql,
            Self::Sql => Self::Logs,
            Self::Logs => Self::Jobs,
            Self::Jobs => Self::Workers,
            Self::Workers => Self::Ci,
            Self::Ci => Self::Help,
            Self::Help => Self::Cluster,
        }
    }

    /// Get previous view in cycle.
    pub fn prev(self) -> Self {
        match self {
            Self::Cluster => Self::Help,
            Self::Metrics => Self::Cluster,
            Self::KeyValue => Self::Metrics,
            Self::Vaults => Self::KeyValue,
            Self::Sql => Self::Vaults,
            Self::Logs => Self::Sql,
            Self::Jobs => Self::Logs,
            Self::Workers => Self::Jobs,
            Self::Ci => Self::Workers,
            Self::Help => Self::Ci,
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cluster => "Cluster",
            Self::Metrics => "Metrics",
            Self::KeyValue => "Key-Value",
            Self::Vaults => "Vaults",
            Self::Sql => "SQL",
            Self::Logs => "Logs",
            Self::Jobs => "Jobs",
            Self::Workers => "Workers",
            Self::Ci => "CI",
            Self::Help => "Help",
        }
    }
}

/// Input mode for text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal navigation mode.
    #[default]
    Normal,
    /// Text input mode (for key-value operations).
    Editing,
    /// SQL query editing mode.
    SqlEditing,
}

/// Cluster operation request for async execution.
#[derive(Debug, Clone)]
pub enum ClusterOperation {
    /// Initialize the cluster.
    Init,
    /// Add a learner node.
    AddLearner { node_id: u64 },
    /// Promote learners to voters.
    ChangeMembership { members: Vec<u64> },
    /// Write a key-value pair.
    WriteKey { key: String, value: Vec<u8> },
    /// Read a key.
    ReadKey { key: String },
}
