//! Application state struct and constructors.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use color_eyre::Result;
use tracing::info;

use super::types::ActiveView;
use super::types::InputMode;
use crate::client_trait::ClientImpl;
use crate::client_trait::ClusterClient;
use crate::iroh_client::MultiNodeClient;
use crate::iroh_client::parse_cluster_ticket;
use crate::types::CiState;
use crate::types::ClusterMetrics;
use crate::types::JobsState;
use crate::types::NodeInfo;
use crate::types::SqlState;
use crate::types::WorkersState;
use crate::types::load_sql_history;

/// Maximum input buffer size (8KB).
/// Tiger Style: Prevents memory issues from large paste operations.
pub(crate) const MAX_INPUT_SIZE: usize = 8192;

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
    pub is_debug_mode: bool,

    /// Maximum nodes to display.
    pub max_display_nodes: usize,

    /// Client for cluster communication (HTTP or Iroh).
    pub(crate) client: Arc<dyn ClusterClient>,

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

    /// CI view state.
    pub ci_state: CiState,
}

impl App {
    /// Create new application state with HTTP client.
    ///
    /// Tiger Style: Explicit initialization of all fields.
    pub fn new(_node_urls: Vec<String>, is_debug_mode: bool, max_display_nodes: usize) -> Self {
        // Start with disconnected client - use connect command or ticket to connect
        let client: Arc<dyn ClusterClient> =
            Arc::new(ClientImpl::Disconnected(crate::client_trait::DisconnectedClient));

        // Load SQL history from disk
        let mut sql_state = SqlState::default();
        sql_state.history = load_sql_history();
        sql_state.history_index = sql_state.history.len() as u32;

        Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            is_debug_mode,
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
            ci_state: CiState::default(),
        }
    }

    /// Create new application state with Iroh multi-node client.
    ///
    /// Uses the MultiNodeClient to automatically discover and connect to all
    /// nodes in the cluster.
    pub async fn new_with_iroh(ticket: String, is_debug_mode: bool, max_display_nodes: usize) -> Result<Self> {
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
        sql_state.history_index = sql_state.history.len() as u32;

        Ok(Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            is_debug_mode,
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
            ci_state: CiState::default(),
        })
    }

    /// Create new application state without any initial connection.
    ///
    /// Starts in disconnected mode, allowing user to connect later via commands.
    pub fn new_disconnected(is_debug_mode: bool, max_display_nodes: usize) -> Self {
        use crate::client_trait::DisconnectedClient;

        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Disconnected(DisconnectedClient));

        // Load SQL history from disk
        let mut sql_state = SqlState::default();
        sql_state.history = load_sql_history();
        sql_state.history_index = sql_state.history.len() as u32;

        Self {
            should_quit: false,
            active_view: ActiveView::default(),
            input_mode: InputMode::default(),
            is_debug_mode,
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
            ci_state: CiState::default(),
        }
    }

    /// Set status message.
    pub(crate) fn set_status(&mut self, message: &str) {
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
