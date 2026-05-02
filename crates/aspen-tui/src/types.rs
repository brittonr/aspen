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
    pub node_count: u32,
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
#[derive(Default)]
pub struct SqlState {
    /// Current query buffer.
    pub query_buffer: String,
    /// Query history (managed by rat-widgets CommandHistory).
    pub history: rat_widgets::CommandHistory,
    /// Consistency level for queries.
    pub consistency: SqlConsistency,
    /// Last query result.
    pub last_result: Option<SqlQueryResult>,
}

impl SqlState {
    /// Add a query to history.
    pub fn add_to_history(&mut self, query: String) {
        self.history.add(query);
    }

    /// Navigate to previous history entry.
    pub fn history_prev(&mut self) {
        if let Some(entry) = self.history.prev() {
            self.query_buffer = entry.to_string();
        }
    }

    /// Navigate to next history entry.
    pub fn history_next(&mut self) {
        if let Some(entry) = self.history.next_entry() {
            self.query_buffer = entry.to_string();
        } else {
            self.query_buffer.clear();
        }
    }
}

/// SQL query result for display.
#[derive(Debug, Clone)]
pub struct SqlQueryResult {
    /// Whether the query succeeded.
    pub is_success: bool,
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
            is_success: true,
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
            is_success: false,
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

/// Load SQL command history from disk using CommandHistory persistence.
pub fn load_sql_history() -> rat_widgets::CommandHistory {
    let Some(dir) = config_dir() else {
        return rat_widgets::CommandHistory::new(100);
    };
    let path = dir.join(SQL_HISTORY_FILE);
    rat_widgets::CommandHistory::load_json(&path, 100)
}

/// Save SQL command history to disk.
pub fn save_sql_history(history: &rat_widgets::CommandHistory) {
    let Some(dir) = config_dir() else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::debug!("failed to create config directory: {e}");
        return;
    }
    let path = dir.join(SQL_HISTORY_FILE);
    history.save_json(&path);
}

// =============================================================================
// Job Queue Types
// =============================================================================

/// Maximum number of jobs to display in the TUI.
///
/// Tiger Style: Bounded to prevent unbounded memory use.
pub const MAX_DISPLAYED_JOBS: usize = 1000;

/// Maximum number of workers to display in the TUI.
///
/// Tiger Style: Bounded to prevent unbounded memory use.
pub const MAX_DISPLAYED_WORKERS: usize = 100;

/// Job status filter for the jobs view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JobStatusFilter {
    /// Show all jobs.
    #[default]
    All,
    /// Show pending jobs.
    Pending,
    /// Show scheduled jobs.
    Scheduled,
    /// Show running jobs.
    Running,
    /// Show completed jobs.
    Completed,
    /// Show failed jobs.
    Failed,
    /// Show cancelled jobs.
    Cancelled,
}

impl JobStatusFilter {
    /// Cycle to the next filter.
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Pending,
            Self::Pending => Self::Scheduled,
            Self::Scheduled => Self::Running,
            Self::Running => Self::Completed,
            Self::Completed => Self::Failed,
            Self::Failed => Self::Cancelled,
            Self::Cancelled => Self::All,
        }
    }

    /// Get display string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Pending => "Pending",
            Self::Scheduled => "Scheduled",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Convert to RPC filter string (None for All).
    pub fn to_rpc_filter(&self) -> Option<String> {
        match self {
            Self::All => None,
            Self::Pending => Some("pending".to_string()),
            Self::Scheduled => Some("scheduled".to_string()),
            Self::Running => Some("running".to_string()),
            Self::Completed => Some("completed".to_string()),
            Self::Failed => Some("failed".to_string()),
            Self::Cancelled => Some("cancelled".to_string()),
        }
    }
}

/// Job priority filter for the jobs view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JobPriorityFilter {
    /// Show all priorities.
    #[default]
    All,
    /// Show low priority jobs.
    Low,
    /// Show normal priority jobs.
    Normal,
    /// Show high priority jobs.
    High,
    /// Show critical priority jobs.
    Critical,
}

impl JobPriorityFilter {
    /// Cycle to the next filter.
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Low,
            Self::Low => Self::Normal,
            Self::Normal => Self::High,
            Self::High => Self::Critical,
            Self::Critical => Self::All,
        }
    }

    /// Get display string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    /// Get priority level (0-3) or None for All.
    pub fn to_priority(&self) -> Option<u8> {
        match self {
            Self::All => None,
            Self::Low => Some(0),
            Self::Normal => Some(1),
            Self::High => Some(2),
            Self::Critical => Some(3),
        }
    }
}

/// Information about a single job for TUI display.
#[derive(Debug, Clone)]
pub struct JobInfo {
    /// Job ID.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Current status.
    pub status: String,
    /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
    pub priority: u8,
    /// Progress percentage (0-100).
    pub progress: u8,
    /// Progress message.
    pub progress_message: Option<String>,
    /// Tags associated with the job.
    pub tags: Vec<String>,
    /// Submission time (ISO 8601).
    pub submitted_at: String,
    /// Start time (ISO 8601).
    pub started_at: Option<String>,
    /// Completion time (ISO 8601).
    pub completed_at: Option<String>,
    /// Worker ID processing this job.
    pub worker_id: Option<String>,
    /// Number of retry attempts.
    pub attempts: u32,
    /// Error message (if failed).
    pub error_message: Option<String>,
}

impl From<aspen_client_api::JobDetails> for JobInfo {
    fn from(job: aspen_client_api::JobDetails) -> Self {
        Self {
            job_id: job.job_id,
            job_type: job.job_type,
            status: job.status,
            priority: job.priority,
            progress: job.progress,
            progress_message: job.progress_message,
            tags: job.tags,
            submitted_at: job.submitted_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            worker_id: job.worker_id,
            attempts: job.attempts,
            error_message: job.error_message,
        }
    }
}

impl JobInfo {
    /// Get priority display string.
    pub fn priority_str(&self) -> &'static str {
        match self.priority {
            0 => "Low",
            1 => "Normal",
            2 => "High",
            3 => "Critical",
            _ => "Unknown",
        }
    }
}

/// Queue statistics for the jobs view.
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Number of pending jobs.
    pub pending_count: u64,
    /// Number of scheduled jobs.
    pub scheduled_count: u64,
    /// Number of running jobs.
    pub running_count: u64,
    /// Number of completed jobs (recent).
    pub completed_count: u64,
    /// Number of failed jobs (recent).
    pub failed_count: u64,
    /// Number of cancelled jobs (recent).
    pub cancelled_count: u64,
    /// Jobs per priority level (priority -> count).
    pub priority_counts: Vec<(u8, u64)>,
    /// Jobs per type (type_name -> count).
    pub type_counts: Vec<(String, u64)>,
}

impl QueueStats {
    /// Get total job count.
    pub fn total(&self) -> u64 {
        self.pending_count
            + self.scheduled_count
            + self.running_count
            + self.completed_count
            + self.failed_count
            + self.cancelled_count
    }
}

impl From<aspen_client_api::JobQueueStatsResultResponse> for QueueStats {
    fn from(stats: aspen_client_api::JobQueueStatsResultResponse) -> Self {
        Self {
            pending_count: stats.pending_count,
            scheduled_count: stats.scheduled_count,
            running_count: stats.running_count,
            completed_count: stats.completed_count,
            failed_count: stats.failed_count,
            cancelled_count: stats.cancelled_count,
            priority_counts: stats.priority_counts.into_iter().map(|count| (count.priority, count.count)).collect(),
            type_counts: stats.type_counts.into_iter().map(|count| (count.job_type, count.count)).collect(),
        }
    }
}

/// Worker pool information for the workers view.
#[derive(Debug, Clone, Default)]
pub struct WorkerPoolInfo {
    /// List of registered workers.
    pub workers: Vec<WorkerInfo>,
    /// Total worker count.
    pub total_workers: u32,
    /// Number of idle workers.
    pub idle_workers: u32,
    /// Number of busy workers.
    pub busy_workers: u32,
    /// Number of offline workers.
    pub offline_workers: u32,
    /// Total capacity across all workers (in job slots).
    pub total_capacity_jobs: u32,
    /// Currently used capacity (in job slots).
    pub used_capacity_jobs: u32,
}

/// Information about a single worker for TUI display.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Worker ID.
    pub worker_id: String,
    /// Worker status: idle, busy, offline.
    pub status: String,
    /// Job types this worker can handle.
    pub capabilities: Vec<String>,
    /// Maximum concurrent jobs (capacity in job count).
    pub capacity_jobs: u32,
    /// Currently active job count.
    pub active_jobs: u32,
    /// Job IDs currently being processed.
    pub active_job_ids: Vec<String>,
    /// Last heartbeat time (ISO 8601).
    pub last_heartbeat: String,
    /// Total jobs processed.
    pub total_processed: u64,
    /// Total jobs failed.
    pub total_failed: u64,
}

impl From<aspen_client_api::WorkerInfo> for WorkerInfo {
    fn from(worker: aspen_client_api::WorkerInfo) -> Self {
        Self {
            worker_id: worker.worker_id,
            status: worker.status,
            capabilities: worker.capabilities,
            capacity_jobs: worker.capacity_jobs,
            active_jobs: worker.active_jobs,
            active_job_ids: worker.active_job_ids,
            last_heartbeat: worker.last_heartbeat,
            total_processed: worker.total_processed,
            total_failed: worker.total_failed,
        }
    }
}

impl From<aspen_client_api::WorkerStatusResultResponse> for WorkerPoolInfo {
    fn from(pool: aspen_client_api::WorkerStatusResultResponse) -> Self {
        Self {
            workers: pool.workers.into_iter().map(WorkerInfo::from).collect(),
            total_workers: pool.total_workers,
            idle_workers: pool.idle_workers,
            busy_workers: pool.busy_workers,
            offline_workers: pool.offline_workers,
            total_capacity_jobs: pool.total_capacity_jobs,
            used_capacity_jobs: pool.used_capacity_jobs,
        }
    }
}

/// Jobs view state.
#[derive(Debug, Clone, Default)]
pub struct JobsState {
    /// Cached job list.
    pub jobs: Vec<JobInfo>,
    /// Selected job index (bounded by MAX_DISPLAYED_JOBS).
    pub selected_job: u32,
    /// Current status filter.
    pub status_filter: JobStatusFilter,
    /// Current priority filter.
    pub priority_filter: JobPriorityFilter,
    /// Cached queue statistics.
    pub queue_stats: QueueStats,
    /// Whether to show job details panel.
    pub show_details: bool,
}

/// Workers view state.
#[derive(Debug, Clone, Default)]
pub struct WorkersState {
    /// Worker pool information.
    pub pool_info: WorkerPoolInfo,
    /// Selected worker index (bounded by MAX_DISPLAYED_WORKERS).
    pub selected_worker: u32,
    /// Whether to show worker details panel.
    pub show_details: bool,
}

// =============================================================================
// CI Pipeline Types
// =============================================================================

/// Maximum number of CI runs to display in the TUI.
///
/// Tiger Style: Bounded to prevent unbounded memory use.
pub const MAX_DISPLAYED_CI_RUNS: usize = 100;

/// CI pipeline status filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CiStatusFilter {
    /// Show all pipelines.
    #[default]
    All,
    /// Show pending pipelines.
    Pending,
    /// Show running pipelines.
    Running,
    /// Show successful pipelines.
    Success,
    /// Show failed pipelines.
    Failed,
    /// Show cancelled pipelines.
    Cancelled,
}

impl CiStatusFilter {
    /// Cycle to the next filter.
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Pending,
            Self::Pending => Self::Running,
            Self::Running => Self::Success,
            Self::Success => Self::Failed,
            Self::Failed => Self::Cancelled,
            Self::Cancelled => Self::All,
        }
    }

    /// Get display string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Success => "Success",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Convert to RPC filter string (None for All).
    pub fn to_rpc_filter(&self) -> Option<String> {
        match self {
            Self::All => None,
            Self::Pending => Some("pending".to_string()),
            Self::Running => Some("running".to_string()),
            Self::Success => Some("success".to_string()),
            Self::Failed => Some("failed".to_string()),
            Self::Cancelled => Some("cancelled".to_string()),
        }
    }
}

/// CI pipeline run information for TUI display.
#[derive(Debug, Clone)]
pub struct CiPipelineRunInfo {
    /// Pipeline run ID.
    pub run_id: String,
    /// Repository ID (hex).
    pub repo_id: String,
    /// Git reference name.
    pub ref_name: String,
    /// Pipeline status.
    pub status: String,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
}

impl From<aspen_client_api::CiRunInfo> for CiPipelineRunInfo {
    fn from(run: aspen_client_api::CiRunInfo) -> Self {
        Self {
            run_id: run.run_id,
            repo_id: run.repo_id,
            ref_name: run.ref_name,
            status: run.status,
            created_at_ms: run.created_at_ms,
        }
    }
}

/// CI pipeline run detail with stages.
#[derive(Debug, Clone)]
pub struct CiPipelineDetail {
    /// Pipeline run ID.
    pub run_id: String,
    /// Repository ID (hex).
    pub repo_id: String,
    /// Git reference name.
    pub ref_name: String,
    /// Commit hash (hex).
    pub commit_hash: String,
    /// Pipeline status.
    pub status: String,
    /// Stages in this pipeline.
    pub stages: Vec<CiStageInfo>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
    /// Completion time (Unix timestamp in milliseconds).
    pub completed_at_ms: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl From<aspen_client_api::CiGetStatusResponse> for CiPipelineDetail {
    fn from(status: aspen_client_api::CiGetStatusResponse) -> Self {
        Self {
            run_id: status.run_id.unwrap_or_default(),
            repo_id: status.repo_id.unwrap_or_default(),
            ref_name: status.ref_name.unwrap_or_default(),
            commit_hash: status.commit_hash.unwrap_or_default(),
            status: status.status.unwrap_or_default(),
            stages: status.stages.into_iter().map(CiStageInfo::from).collect(),
            created_at_ms: status.created_at_ms.unwrap_or(0),
            completed_at_ms: status.completed_at_ms,
            error: status.error,
        }
    }
}

/// CI stage information.
#[derive(Debug, Clone)]
pub struct CiStageInfo {
    /// Stage name.
    pub name: String,
    /// Stage status.
    pub status: String,
    /// Jobs in this stage.
    pub jobs: Vec<CiJobInfo>,
}

impl From<aspen_client_api::CiStageInfo> for CiStageInfo {
    fn from(stage: aspen_client_api::CiStageInfo) -> Self {
        Self {
            name: stage.name,
            status: stage.status,
            jobs: stage.jobs.into_iter().map(CiJobInfo::from).collect(),
        }
    }
}

/// CI job information.
#[derive(Debug, Clone)]
pub struct CiJobInfo {
    /// Job ID.
    pub id: String,
    /// Job name.
    pub name: String,
    /// Job status.
    pub status: String,
    /// Job start time (Unix timestamp in milliseconds).
    pub started_at_ms: Option<u64>,
    /// Job end time (Unix timestamp in milliseconds).
    pub ended_at_ms: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl From<aspen_client_api::CiJobInfo> for CiJobInfo {
    fn from(job: aspen_client_api::CiJobInfo) -> Self {
        Self {
            id: job.id,
            name: job.name,
            status: job.status,
            started_at_ms: job.started_at_ms,
            ended_at_ms: job.ended_at_ms,
            error: job.error,
        }
    }
}

/// CI view state.
#[derive(Debug, Clone, Default)]
pub struct CiState {
    /// Cached pipeline runs.
    pub runs: Vec<CiPipelineRunInfo>,
    /// Selected run index (bounded by MAX_DISPLAYED_CI_RUNS).
    pub selected_run: u32,
    /// Current status filter.
    pub status_filter: CiStatusFilter,
    /// Optional repository filter.
    pub repo_filter: Option<String>,
    /// Whether to show details panel.
    pub show_details: bool,
    /// Selected run detail (fetched on demand).
    pub selected_detail: Option<CiPipelineDetail>,
    /// Log stream state for viewing job logs.
    pub log_stream: CiLogStreamState,
    /// Selected job index within the current run (for log viewing).
    pub selected_job_index: Option<u32>,
}

// ============================================================================
// CI Log Streaming Types
// ============================================================================

/// A single log line for display in the TUI (used by watch channel).
#[derive(Debug, Clone)]
pub struct CiLogLine {
    /// Log content (single line).
    pub content: String,
    /// Stream source: "stdout", "stderr", "build".
    pub stream: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Log stream metadata for CI job log viewing.
///
/// Line storage and scrolling are handled by `rat_streaming::StreamingOutput`.
#[derive(Debug, Clone, Default)]
pub struct CiLogStreamState {
    /// Pipeline run ID being watched.
    pub run_id: Option<String>,
    /// Job ID being watched.
    pub job_id: Option<String>,
    /// Whether the stream is active (job still running).
    pub is_streaming: bool,
    /// Last received chunk index.
    pub last_chunk_index: u32,
    /// Error message if stream failed.
    pub error: Option<String>,
    /// Whether log panel is visible.
    pub is_visible: bool,
}

// CiLogStreamState methods removed — line storage and scrolling
// now handled by rat_streaming::StreamingOutput in App.ci_log_output
