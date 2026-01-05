//! Application state and logic for the Aspen TUI.
//!
//! Implements The Elm Architecture (TEA) pattern with:
//! - Model: `App` struct holding all application state
//! - Update: `App::handle_event()` for state transitions
//! - View: Handled by `ui` module

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use color_eyre::Result;
use tracing::info;
use tracing::warn;

use crate::client_trait::ClientImpl;
use crate::client_trait::ClusterClient;
use crate::commands::Command;
use crate::commands::key_to_command;
use crate::event::Event;
use crate::iroh_client::MultiNodeClient;
use crate::iroh_client::parse_cluster_ticket;
use crate::types::ClusterMetrics;
use crate::types::JobsState;
use crate::types::MAX_DISPLAYED_JOBS;
use crate::types::MAX_SQL_QUERY_SIZE;
use crate::types::NodeInfo;
use crate::types::NodeStatus;
use crate::types::SqlQueryResult;
use crate::types::SqlState;
use crate::types::WorkersState;
use crate::types::load_sql_history;
use crate::types::save_sql_history;

/// Maximum input buffer size (8KB).
/// Tiger Style: Prevents memory issues from large paste operations.
const MAX_INPUT_SIZE: usize = 8192;

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
            Self::Workers => Self::Help,
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
            Self::Help => Self::Workers,
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

/// Main application state.
///
/// Tiger Style: All state in a single struct for clear ownership.
pub struct App {
    /// Flag to exit main loop.
    pub should_quit: bool,

    /// Current active view.
    pub active_view: ActiveView,

    /// Current input mode.
    pub input_mode: InputMode,

    /// Debug mode enabled.
    pub debug_mode: bool,

    /// Maximum nodes to display.
    pub max_display_nodes: usize,

    /// Client for cluster communication (HTTP or Iroh).
    client: Arc<dyn ClusterClient>,

    /// Cached node information.
    pub nodes: BTreeMap<u64, NodeInfo>,

    /// Cached cluster metrics.
    pub cluster_metrics: Option<ClusterMetrics>,

    /// Selected node index in the list.
    pub selected_node: usize,

    /// Last refresh timestamp.
    pub last_refresh: Option<Instant>,

    /// Refresh in progress flag.
    pub refreshing: bool,

    /// Status message for display.
    pub status_message: Option<(String, Instant)>,

    /// Input buffer for text entry.
    pub input_buffer: String,

    /// Key input buffer for KV operations.
    pub key_buffer: String,

    /// Value input buffer for KV operations.
    pub value_buffer: String,

    /// Last read result.
    pub last_read_result: Option<(String, Option<Vec<u8>>)>,

    /// Scroll position for log view.
    pub log_scroll: u16,

    /// Cached vault list.
    pub vaults: Vec<crate::client_trait::VaultSummary>,

    /// Selected vault index.
    pub selected_vault: usize,

    /// Currently displayed vault keys (when viewing a specific vault).
    pub vault_keys: Vec<crate::client_trait::VaultKeyEntry>,

    /// Selected key index within a vault.
    pub selected_vault_key: usize,

    /// Currently active vault name (None = vault list view, Some = vault contents view).
    pub active_vault: Option<String>,

    /// SQL view state.
    pub sql_state: SqlState,

    /// Jobs view state.
    pub jobs_state: JobsState,

    /// Workers view state.
    pub workers_state: WorkersState,
}

impl App {
    /// Create new application state with HTTP client.
    ///
    /// Tiger Style: Explicit initialization of all fields.
    pub fn new(_node_urls: Vec<String>, debug_mode: bool, max_display_nodes: usize) -> Self {
        // Start with disconnected client - use connect command or ticket to connect
        let client: Arc<dyn ClusterClient> =
            Arc::new(ClientImpl::Disconnected(crate::client_trait::DisconnectedClient));

        // Load SQL history from disk
        let mut sql_state = SqlState::default();
        sql_state.history = load_sql_history();
        sql_state.history_index = sql_state.history.len();

        Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            debug_mode,
            max_display_nodes,
            client,
            nodes: BTreeMap::new(),
            cluster_metrics: None,
            selected_node: 0,
            last_refresh: None,
            refreshing: false,
            status_message: None,
            input_buffer: String::new(),
            key_buffer: String::new(),
            value_buffer: String::new(),
            last_read_result: None,
            log_scroll: 0,
            vaults: Vec::new(),
            selected_vault: 0,
            vault_keys: Vec::new(),
            selected_vault_key: 0,
            active_vault: None,
            sql_state,
            jobs_state: JobsState::default(),
            workers_state: WorkersState::default(),
        }
    }

    /// Create new application state with Iroh multi-node client.
    ///
    /// Uses the MultiNodeClient to automatically discover and connect to all
    /// nodes in the cluster.
    pub async fn new_with_iroh(ticket: String, debug_mode: bool, max_display_nodes: usize) -> Result<Self> {
        // Parse the ticket to get all endpoint addresses
        let endpoint_addrs = parse_cluster_ticket(&ticket).map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;

        info!("Ticket contains {} bootstrap peers, creating multi-node client", endpoint_addrs.len());

        // Create MultiNodeClient with all bootstrap endpoints
        let multi_client =
            MultiNodeClient::new(endpoint_addrs).await.map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;
        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::MultiNode(multi_client));

        // Load SQL history from disk
        let mut sql_state = SqlState::default();
        sql_state.history = load_sql_history();
        sql_state.history_index = sql_state.history.len();

        Ok(Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            debug_mode,
            max_display_nodes,
            client,
            nodes: BTreeMap::new(),
            cluster_metrics: None,
            selected_node: 0,
            last_refresh: None,
            refreshing: false,
            status_message: None,
            input_buffer: String::new(),
            key_buffer: String::new(),
            value_buffer: String::new(),
            last_read_result: None,
            log_scroll: 0,
            vaults: Vec::new(),
            selected_vault: 0,
            vault_keys: Vec::new(),
            selected_vault_key: 0,
            active_vault: None,
            sql_state,
            jobs_state: JobsState::default(),
            workers_state: WorkersState::default(),
        })
    }

    /// Create new application state without any initial connection.
    ///
    /// Starts in disconnected mode, allowing user to connect later via commands.
    pub fn new_disconnected(debug_mode: bool, max_display_nodes: usize) -> Self {
        use crate::client_trait::DisconnectedClient;

        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Disconnected(DisconnectedClient));

        // Load SQL history from disk
        let mut sql_state = SqlState::default();
        sql_state.history = load_sql_history();
        sql_state.history_index = sql_state.history.len();

        Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            debug_mode,
            max_display_nodes,
            client,
            nodes: BTreeMap::new(),
            cluster_metrics: None,
            selected_node: 0,
            last_refresh: None,
            refreshing: false,
            status_message: Some((
                "Not connected - Press 'c' to connect to nodes or 't' for ticket".to_string(),
                Instant::now(),
            )),
            input_buffer: String::new(),
            key_buffer: String::new(),
            value_buffer: String::new(),
            last_read_result: None,
            log_scroll: 0,
            vaults: Vec::new(),
            selected_vault: 0,
            vault_keys: Vec::new(),
            selected_vault_key: 0,
            active_vault: None,
            sql_state,
            jobs_state: JobsState::default(),
            workers_state: WorkersState::default(),
        }
    }

    /// Handle incoming event.
    ///
    /// Tiger Style: Centralized event dispatch.
    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Tick => self.on_tick().await,
            Event::Key(key_event) => self.on_key(key_event).await,
            Event::Mouse(_) => {}     // Ignore mouse events for now
            Event::Resize(_, _) => {} // Terminal handles resize
        }
        Ok(())
    }

    /// Handle tick event (periodic refresh).
    async fn on_tick(&mut self) {
        // Auto-refresh every tick if not already refreshing
        if !self.refreshing {
            // Wrap refresh in a timeout to prevent UI freezing
            // If refresh takes too long, skip it and try again next tick
            let refresh_timeout = Duration::from_secs(3);
            match tokio::time::timeout(refresh_timeout, self.refresh_cluster_state()).await {
                Ok(_) => {
                    // Refresh completed successfully
                }
                Err(_) => {
                    // Refresh timed out, mark as not refreshing so we try again
                    self.refreshing = false;
                    self.set_status("Network timeout - retrying...");
                    warn!("Refresh timed out after {:?}", refresh_timeout);
                }
            }
        }

        // Clear old status messages (after 5 seconds)
        if let Some((_, timestamp)) = &self.status_message
            && timestamp.elapsed() > Duration::from_secs(5)
        {
            self.status_message = None;
        }
    }

    /// Handle key event.
    ///
    /// Uses the command pattern to separate key interpretation from execution.
    /// Tiger Style: Keeps this function small by delegating to `execute_command`.
    async fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        if let Some(cmd) = key_to_command(key, self.active_view, self.input_mode) {
            self.execute_command(cmd).await;
        }
    }

    /// Execute a command.
    ///
    /// Tiger Style: Single point of command execution for testability.
    async fn execute_command(&mut self, cmd: Command) {
        match cmd {
            // Global commands
            Command::Quit => self.should_quit = true,
            Command::Refresh => self.refresh_cluster_state().await,

            // View navigation
            Command::NextView => self.switch_to_next_view().await,
            Command::PrevView => self.switch_to_prev_view().await,
            Command::SwitchToView(view) => self.switch_to_view(view).await,
            Command::ShowHelp => self.active_view = ActiveView::Help,

            // List navigation
            Command::NavigateUp => self.navigate_up(),
            Command::NavigateDown => self.navigate_down(),

            // Input mode transitions
            Command::EnterEditMode => self.input_mode = InputMode::Editing,
            Command::EnterSqlEditMode => {
                self.input_mode = InputMode::SqlEditing;
                self.set_status("Editing SQL query (Enter to save, Esc to cancel, Up/Down for history)");
            }
            Command::ExitEditMode => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            Command::ExitSqlEditMode => {
                self.input_mode = InputMode::Normal;
                self.set_status("Edit cancelled");
            }

            // Cluster operations
            Command::InitCluster => self.init_cluster().await,
            Command::ConnectHttp => {
                self.input_mode = InputMode::Editing;
                self.input_buffer = "http://127.0.0.1:21001".to_string();
                self.set_status("Enter HTTP node address(es) separated by spaces, then press Enter");
            }
            Command::ConnectTicket => {
                self.input_mode = InputMode::Editing;
                self.input_buffer.clear();
                self.set_status("Paste Iroh cluster ticket, then press Enter");
            }

            // Key-value operations
            Command::ExecuteKvOperation => {
                // Handled in InputEnter based on context
            }

            // Vault operations
            Command::EnterVault => self.enter_vault().await,
            Command::ExitVault => self.exit_vault(),

            // SQL operations
            Command::ExecuteSqlQuery => self.execute_sql_query().await,
            Command::ToggleSqlConsistency => {
                self.sql_state.consistency = self.sql_state.consistency.toggle();
                self.set_status(&format!("Consistency: {}", self.sql_state.consistency.as_str()));
            }
            Command::SqlScrollLeft => {
                if self.sql_state.result_scroll_col > 0 {
                    self.sql_state.result_scroll_col -= 1;
                }
            }
            Command::SqlScrollRight => {
                if let Some(result) = &self.sql_state.last_result {
                    let max = result.columns.len().saturating_sub(1);
                    if self.sql_state.result_scroll_col < max {
                        self.sql_state.result_scroll_col += 1;
                    }
                }
            }

            // Log operations
            Command::LogScrollUp => self.log_scroll = self.log_scroll.saturating_sub(10),
            Command::LogScrollDown => self.log_scroll = self.log_scroll.saturating_add(10),

            // Jobs operations
            Command::CycleJobStatusFilter => {
                self.jobs_state.status_filter = self.jobs_state.status_filter.next();
                self.set_status(&format!("Status filter: {}", self.jobs_state.status_filter.as_str()));
                self.refresh_jobs().await;
            }
            Command::CycleJobPriorityFilter => {
                self.jobs_state.priority_filter = self.jobs_state.priority_filter.next();
                self.set_status(&format!("Priority filter: {}", self.jobs_state.priority_filter.as_str()));
                self.refresh_jobs().await;
            }
            Command::ToggleJobDetails => self.jobs_state.show_details = !self.jobs_state.show_details,
            Command::CancelSelectedJob => self.cancel_selected_job().await,

            // Workers operations
            Command::ToggleWorkerDetails => self.workers_state.show_details = !self.workers_state.show_details,

            // Input handling
            Command::InputChar(c) => self.handle_input_char(c),
            Command::InputBackspace => self.handle_input_backspace(),
            Command::InputTab => std::mem::swap(&mut self.key_buffer, &mut self.input_buffer),
            Command::InputEnter => self.handle_input_enter().await,
            Command::HistoryPrev => self.sql_state.history_prev(),
            Command::HistoryNext => self.sql_state.history_next(),
        }
    }

    /// Switch to the next view in cycle.
    async fn switch_to_next_view(&mut self) {
        let prev_view = self.active_view;
        self.active_view = self.active_view.next();
        self.on_view_change(prev_view).await;
    }

    /// Switch to the previous view in cycle.
    async fn switch_to_prev_view(&mut self) {
        let prev_view = self.active_view;
        self.active_view = self.active_view.prev();
        self.on_view_change(prev_view).await;
    }

    /// Switch to a specific view.
    async fn switch_to_view(&mut self, view: ActiveView) {
        let prev_view = self.active_view;
        self.active_view = view;
        self.on_view_change(prev_view).await;
    }

    /// Handle view change side effects (auto-refresh).
    async fn on_view_change(&mut self, prev_view: ActiveView) {
        match self.active_view {
            ActiveView::Vaults if prev_view != ActiveView::Vaults => self.refresh_vaults().await,
            ActiveView::Jobs if prev_view != ActiveView::Jobs => self.refresh_jobs().await,
            ActiveView::Workers if prev_view != ActiveView::Workers => self.refresh_workers().await,
            _ => {}
        }
    }

    /// Navigate up in current list.
    fn navigate_up(&mut self) {
        match self.active_view {
            ActiveView::Vaults => {
                if self.active_vault.is_some() {
                    if self.selected_vault_key > 0 {
                        self.selected_vault_key -= 1;
                    }
                } else if self.selected_vault > 0 {
                    self.selected_vault -= 1;
                }
            }
            ActiveView::Sql => {
                if self.sql_state.selected_row > 0 {
                    self.sql_state.selected_row -= 1;
                }
            }
            ActiveView::Jobs => {
                if self.jobs_state.selected_job > 0 {
                    self.jobs_state.selected_job -= 1;
                }
            }
            ActiveView::Workers => {
                if self.workers_state.selected_worker > 0 {
                    self.workers_state.selected_worker -= 1;
                }
            }
            _ => {
                if self.selected_node > 0 {
                    self.selected_node -= 1;
                }
            }
        }
    }

    /// Navigate down in current list.
    fn navigate_down(&mut self) {
        match self.active_view {
            ActiveView::Vaults => {
                if self.active_vault.is_some() {
                    let max = self.vault_keys.len().saturating_sub(1);
                    if self.selected_vault_key < max {
                        self.selected_vault_key += 1;
                    }
                } else {
                    let max = self.vaults.len().saturating_sub(1);
                    if self.selected_vault < max {
                        self.selected_vault += 1;
                    }
                }
            }
            ActiveView::Sql => {
                if let Some(result) = &self.sql_state.last_result {
                    let max = result.rows.len().saturating_sub(1);
                    if self.sql_state.selected_row < max {
                        self.sql_state.selected_row += 1;
                    }
                }
            }
            ActiveView::Jobs => {
                let max = self.jobs_state.jobs.len().saturating_sub(1);
                if self.jobs_state.selected_job < max {
                    self.jobs_state.selected_job += 1;
                }
            }
            ActiveView::Workers => {
                let max = self.workers_state.pool_info.workers.len().saturating_sub(1);
                if self.workers_state.selected_worker < max {
                    self.workers_state.selected_worker += 1;
                }
            }
            _ => {
                let max = self.nodes.len().saturating_sub(1);
                if self.selected_node < max {
                    self.selected_node += 1;
                }
            }
        }
    }

    /// Enter a vault (drill down).
    async fn enter_vault(&mut self) {
        if self.active_vault.is_none() && !self.vaults.is_empty() {
            if let Some(vault) = self.vaults.get(self.selected_vault) {
                let vault_name = vault.name.clone();
                self.active_vault = Some(vault_name.clone());
                self.selected_vault_key = 0;
                self.refresh_vault_keys(&vault_name).await;
            }
        }
    }

    /// Exit vault (go back to vault list).
    fn exit_vault(&mut self) {
        if self.active_vault.is_some() {
            self.active_vault = None;
            self.vault_keys.clear();
            self.selected_vault_key = 0;
        } else {
            // If not in a vault, Esc quits
            self.should_quit = true;
        }
    }

    /// Handle character input in editing modes.
    fn handle_input_char(&mut self, c: char) {
        match self.input_mode {
            InputMode::Editing => {
                if self.input_buffer.len() < MAX_INPUT_SIZE {
                    self.input_buffer.push(c);
                }
            }
            InputMode::SqlEditing => {
                if self.sql_state.query_buffer.len() < MAX_SQL_QUERY_SIZE {
                    self.sql_state.query_buffer.push(c);
                    self.sql_state.history_browsing = false;
                }
            }
            InputMode::Normal => {}
        }
    }

    /// Handle backspace in editing modes.
    fn handle_input_backspace(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                self.input_buffer.pop();
            }
            InputMode::SqlEditing => {
                self.sql_state.query_buffer.pop();
                self.sql_state.history_browsing = false;
            }
            InputMode::Normal => {}
        }
    }

    /// Handle enter in editing modes.
    async fn handle_input_enter(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                let is_ticket_connect = self
                    .status_message
                    .as_ref()
                    .map(|(msg, _)| msg.contains("Paste Iroh cluster ticket"))
                    .unwrap_or(false);

                if is_ticket_connect {
                    self.connect_iroh_ticket(&self.input_buffer.clone()).await;
                } else {
                    self.execute_kv_operation().await;
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            InputMode::SqlEditing => {
                self.input_mode = InputMode::Normal;
                self.set_status("Query saved (press 'e' to execute)");
            }
            InputMode::Normal => {}
        }
    }

    /// Refresh cluster state from the node.
    ///
    /// For HTTP clients: queries each known node port individually.
    /// For MultiNode clients: uses GetClusterState to discover all nodes automatically.
    async fn refresh_cluster_state(&mut self) {
        self.refreshing = true;
        let mut new_nodes = BTreeMap::new();

        // First, get info from the connected node
        match self.client.get_node_info().await {
            Ok(info) => {
                new_nodes.insert(info.node_id, info);

                // Try to get metrics - this also triggers discovery for MultiNode client
                if let Ok(metrics) = self.client.get_metrics().await {
                    self.cluster_metrics = Some(metrics.clone());

                    // For MultiNode (Iroh) client, populate all discovered nodes
                    {
                        // Use the trait method to get discovered nodes
                        match self.client.get_discovered_nodes().await {
                            Ok(discovered_nodes) if !discovered_nodes.is_empty() => {
                                info!(
                                    discovered_count = discovered_nodes.len(),
                                    leader = ?metrics.leader,
                                    "discovered cluster peers via GetClusterState"
                                );

                                // Add each discovered node to our nodes map
                                for node_desc in discovered_nodes {
                                    // Create a NodeInfo from the discovered node descriptor
                                    let node_info = NodeInfo {
                                        node_id: node_desc.node_id,
                                        status: NodeStatus::Healthy, // Assume healthy for discovered nodes
                                        is_leader: node_desc.is_leader,
                                        last_applied_index: None, // Will be updated on next metrics call
                                        current_term: Some(metrics.term),
                                        uptime_secs: None,
                                        addr: format!("iroh://{}", node_desc.endpoint_addr),
                                    };
                                    new_nodes.insert(node_desc.node_id, node_info);
                                }
                            }
                            Ok(_) => {
                                // No discovered nodes (single-node client or HTTP)
                                if metrics.node_count > 1 {
                                    info!(
                                        total_nodes = metrics.node_count,
                                        leader = ?metrics.leader,
                                        "cluster has multiple nodes but discovery not available"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to discover peers");
                            }
                        }
                    }
                }

                // For Iroh MultiNode client, we already got the cluster state from get_metrics()
                // The ClusterMetrics now includes the actual discovered node count
            }
            Err(e) => {
                warn!(error = %e, "failed to get node info");
                self.set_status(&format!("Failed to refresh: {}", e));
            }
        }

        self.nodes = new_nodes;
        self.last_refresh = Some(Instant::now());
        self.refreshing = false;

        // Also refresh vaults if we're on the vaults view
        if self.active_view == ActiveView::Vaults {
            self.refresh_vaults().await;
        }
    }

    /// Refresh the list of vaults.
    async fn refresh_vaults(&mut self) {
        match self.client.list_vaults().await {
            Ok(vaults) => {
                self.vaults = vaults;
                // Reset selection if out of bounds
                if self.selected_vault >= self.vaults.len() && !self.vaults.is_empty() {
                    self.selected_vault = self.vaults.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to list vaults");
                self.set_status(&format!("Failed to list vaults: {}", e));
            }
        }
    }

    /// Refresh keys for a specific vault.
    async fn refresh_vault_keys(&mut self, vault: &str) {
        match self.client.list_vault_keys(vault).await {
            Ok(keys) => {
                self.vault_keys = keys;
                // Reset selection if out of bounds
                if self.selected_vault_key >= self.vault_keys.len() && !self.vault_keys.is_empty() {
                    self.selected_vault_key = self.vault_keys.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, vault = vault, "failed to list vault keys");
                self.set_status(&format!("Failed to list vault keys: {}", e));
            }
        }
    }

    /// Refresh the jobs list from the cluster.
    async fn refresh_jobs(&mut self) {
        let status_filter = self.jobs_state.status_filter.to_rpc_filter();
        let limit = Some(MAX_DISPLAYED_JOBS as u32);

        match self.client.list_jobs(status_filter, limit).await {
            Ok(mut jobs) => {
                // Apply priority filter client-side (API may not support it)
                if let Some(priority) = self.jobs_state.priority_filter.to_priority() {
                    jobs.retain(|j| j.priority == priority);
                }
                self.jobs_state.jobs = jobs;
                // Reset selection if out of bounds
                if self.jobs_state.selected_job >= self.jobs_state.jobs.len() && !self.jobs_state.jobs.is_empty() {
                    self.jobs_state.selected_job = self.jobs_state.jobs.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to list jobs");
                self.set_status(&format!("Failed to list jobs: {}", e));
            }
        }

        // Also refresh queue stats
        match self.client.get_queue_stats().await {
            Ok(stats) => {
                self.jobs_state.queue_stats = stats;
            }
            Err(e) => {
                warn!(error = %e, "failed to get queue stats");
            }
        }
    }

    /// Refresh the workers list from the cluster.
    async fn refresh_workers(&mut self) {
        match self.client.get_worker_status().await {
            Ok(pool_info) => {
                self.workers_state.pool_info = pool_info;
                // Reset selection if out of bounds
                if self.workers_state.selected_worker >= self.workers_state.pool_info.workers.len()
                    && !self.workers_state.pool_info.workers.is_empty()
                {
                    self.workers_state.selected_worker = self.workers_state.pool_info.workers.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to get worker status");
                self.set_status(&format!("Failed to get worker status: {}", e));
            }
        }
    }

    /// Cancel the currently selected job.
    async fn cancel_selected_job(&mut self) {
        if self.jobs_state.jobs.is_empty() {
            self.set_status("No job selected");
            return;
        }

        let Some(job) = self.jobs_state.jobs.get(self.jobs_state.selected_job) else {
            self.set_status("No job selected");
            return;
        };

        let job_id = job.job_id.clone();

        match self.client.cancel_job(&job_id, Some("Cancelled from TUI".to_string())).await {
            Ok(()) => {
                self.set_status(&format!("Job {} cancelled", job_id));
                // Refresh to show updated status
                self.refresh_jobs().await;
            }
            Err(e) => {
                self.set_status(&format!("Failed to cancel job: {}", e));
            }
        }
    }

    /// Initialize the cluster.
    async fn init_cluster(&mut self) {
        match self.client.init_cluster().await {
            Ok(_) => {
                self.set_status("Cluster initialized successfully");
                info!("cluster initialized");
                self.refresh_cluster_state().await;
            }
            Err(e) => {
                self.set_status(&format!("Init failed: {}", e));
                warn!(error = %e, "cluster init failed");
            }
        }
    }

    /// Execute key-value operation based on input.
    async fn execute_kv_operation(&mut self) {
        let input = self.input_buffer.trim().to_string();

        if let Some(key) = input.strip_prefix("get ") {
            self.read_key(key).await;
        } else if let Some(rest) = input.strip_prefix("set ") {
            if let Some((key, value)) = rest.split_once(' ') {
                self.write_key(key, value.as_bytes().to_vec()).await;
            } else {
                self.set_status("Usage: set <key> <value>");
            }
        } else {
            self.set_status("Commands: get <key> | set <key> <value>");
        }

        self.input_buffer.clear();
    }

    /// Read a key from the cluster.
    async fn read_key(&mut self, key: &str) {
        match self.client.read(key.to_string()).await {
            Ok(value) => {
                self.last_read_result = Some((key.to_string(), value));
                self.set_status(&format!("Read key '{}'", key));
            }
            Err(e) => {
                self.set_status(&format!("Read failed: {}", e));
            }
        }
    }

    /// Write a key-value pair to the cluster.
    async fn write_key(&mut self, key: &str, value: Vec<u8>) {
        match self.client.write(key.to_string(), value).await {
            Ok(_) => {
                self.set_status(&format!("Written key '{}'", key));
            }
            Err(e) => {
                self.set_status(&format!("Write failed: {}", e));
            }
        }
    }

    /// Execute SQL query against the cluster.
    async fn execute_sql_query(&mut self) {
        let query = self.sql_state.query_buffer.trim().to_string();

        // Validate query is not empty
        if query.is_empty() {
            self.set_status("No query to execute (press Enter to edit query first)");
            return;
        }

        // Validate query size
        if query.len() > MAX_SQL_QUERY_SIZE {
            self.set_status(&format!("Query too large ({} bytes, max {})", query.len(), MAX_SQL_QUERY_SIZE));
            return;
        }

        // Add to history and save
        self.sql_state.add_to_history(query.clone());
        save_sql_history(&self.sql_state.history);

        // Reset result state
        self.sql_state.selected_row = 0;
        self.sql_state.result_scroll_col = 0;

        self.set_status("Executing query...");

        // Execute query
        match self
            .client
            .execute_sql(
                query,
                self.sql_state.consistency.as_str().to_string(),
                Some(1000), // Default limit
                Some(5000), // Default timeout
            )
            .await
        {
            Ok(result) => {
                if result.success {
                    let row_count = result.row_count.unwrap_or(0);
                    let exec_time = result.execution_time_ms.unwrap_or(0);
                    let is_truncated = result.is_truncated.unwrap_or(false);

                    self.sql_state.last_result = Some(SqlQueryResult::from_response(
                        result.columns.unwrap_or_default(),
                        result.rows.unwrap_or_default(),
                        row_count,
                        is_truncated,
                        exec_time,
                    ));

                    let truncated_msg = if is_truncated { " (truncated)" } else { "" };
                    self.set_status(&format!("{} rows in {}ms{}", row_count, exec_time, truncated_msg));
                } else {
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    self.sql_state.last_result = Some(SqlQueryResult::error(error_msg.clone()));
                    self.set_status(&format!("Query failed: {}", error_msg));
                }
            }
            Err(e) => {
                self.sql_state.last_result = Some(SqlQueryResult::error(format!("{}", e)));
                self.set_status(&format!("Query failed: {}", e));
            }
        }
    }

    /// Connect via Iroh ticket.
    async fn connect_iroh_ticket(&mut self, ticket: &str) {
        let ticket = ticket.trim();

        if ticket.is_empty() {
            self.set_status("No ticket provided");
            return;
        }

        // Parse the ticket to get all endpoint addresses
        match parse_cluster_ticket(ticket) {
            Ok(endpoint_addrs) => {
                info!("Ticket contains {} bootstrap peers", endpoint_addrs.len());

                // Create MultiNodeClient with all bootstrap endpoints
                match MultiNodeClient::new(endpoint_addrs).await {
                    Ok(multi_client) => {
                        let new_client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::MultiNode(multi_client));

                        // Test the connection
                        match new_client.get_node_info().await {
                            Ok(node_info) => {
                                // Connection successful, replace the client
                                self.client = new_client;
                                self.set_status(&format!("Connected to cluster via Iroh (node {})", node_info.node_id));
                                // Immediately refresh to populate all discovered nodes
                                self.refresh_cluster_state().await;
                            }
                            Err(e) => {
                                self.set_status(&format!("Failed to connect via ticket: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.set_status(&format!("Failed to create Iroh client: {:#}", e));
                    }
                }
            }
            Err(e) => {
                self.set_status(&format!("Invalid ticket format: {:#}", e));
            }
        }
    }

    /// Set status message.
    fn set_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), Instant::now()));
    }

    /// Get the currently selected node info.
    pub fn selected_node_info(&self) -> Option<&NodeInfo> {
        self.nodes.values().nth(self.selected_node)
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
