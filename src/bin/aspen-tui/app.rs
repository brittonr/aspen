//! Application state and logic for the Aspen TUI.
//!
//! Implements The Elm Architecture (TEA) pattern with:
//! - Model: `App` struct holding all application state
//! - Update: `App::handle_event()` for state transitions
//! - View: Handled by `ui` module

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use tracing::{info, warn};

use crate::client::{AspenClient, ClusterMetrics, NodeInfo, NodeStatus};
use crate::client_trait::{ClientImpl, ClusterClient};
use crate::event::Event;
use crate::iroh_client::{MultiNodeClient, parse_cluster_ticket};

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
    /// Log viewer.
    Logs,
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
            Self::Vaults => Self::Logs,
            Self::Logs => Self::Help,
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
            Self::Logs => Self::Vaults,
            Self::Help => Self::Logs,
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cluster => "Cluster",
            Self::Metrics => "Metrics",
            Self::KeyValue => "Key-Value",
            Self::Vaults => "Vaults",
            Self::Logs => "Logs",
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
}

impl App {
    /// Create new application state with HTTP client.
    ///
    /// Tiger Style: Explicit initialization of all fields.
    pub fn new(node_urls: Vec<String>, debug_mode: bool, max_display_nodes: usize) -> Self {
        // Use the first node URL for now
        let url = node_urls
            .into_iter()
            .next()
            .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

        let client = AspenClient::new(url);
        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Http(client));

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
        }
    }

    /// Create new application state with Iroh multi-node client.
    ///
    /// Uses the MultiNodeClient to automatically discover and connect to all
    /// nodes in the cluster.
    pub async fn new_with_iroh(
        ticket: String,
        debug_mode: bool,
        max_display_nodes: usize,
    ) -> Result<Self> {
        // Parse the ticket to get all endpoint addresses
        let endpoint_addrs =
            parse_cluster_ticket(&ticket).map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;

        info!(
            "Ticket contains {} bootstrap peers, creating multi-node client",
            endpoint_addrs.len()
        );

        // Create MultiNodeClient with all bootstrap endpoints
        let multi_client = MultiNodeClient::new(endpoint_addrs)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;
        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::MultiNode(multi_client));

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
        })
    }

    /// Create new application state without any initial connection.
    ///
    /// Starts in disconnected mode, allowing user to connect later via commands.
    pub fn new_disconnected(debug_mode: bool, max_display_nodes: usize) -> Self {
        use crate::client_trait::DisconnectedClient;

        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Disconnected(DisconnectedClient));

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
    async fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match self.input_mode {
            InputMode::Normal => match key.code {
                // Quit (but Esc goes back first in vault view)
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    // In vault view with active vault, Esc goes back
                    if self.active_view == ActiveView::Vaults && self.active_vault.is_some() {
                        self.active_vault = None;
                        self.vault_keys.clear();
                        self.selected_vault_key = 0;
                    } else {
                        self.should_quit = true;
                    }
                }

                // View navigation
                KeyCode::Tab => {
                    let prev_view = self.active_view;
                    self.active_view = self.active_view.next();
                    // Auto-refresh vaults when switching to Vaults view
                    if self.active_view == ActiveView::Vaults && prev_view != ActiveView::Vaults {
                        self.refresh_vaults().await;
                    }
                }
                KeyCode::BackTab => {
                    let prev_view = self.active_view;
                    self.active_view = self.active_view.prev();
                    // Auto-refresh vaults when switching to Vaults view
                    if self.active_view == ActiveView::Vaults && prev_view != ActiveView::Vaults {
                        self.refresh_vaults().await;
                    }
                }
                KeyCode::Char('1') => self.active_view = ActiveView::Cluster,
                KeyCode::Char('2') => self.active_view = ActiveView::Metrics,
                KeyCode::Char('3') => self.active_view = ActiveView::KeyValue,
                KeyCode::Char('4') => {
                    let prev_view = self.active_view;
                    self.active_view = ActiveView::Vaults;
                    // Auto-refresh vaults when switching to Vaults view
                    if prev_view != ActiveView::Vaults {
                        self.refresh_vaults().await;
                    }
                }
                KeyCode::Char('5') => self.active_view = ActiveView::Logs,
                KeyCode::Char('?') => self.active_view = ActiveView::Help,

                // List navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    match self.active_view {
                        ActiveView::Vaults => {
                            if self.active_vault.is_some() {
                                // Navigating vault keys
                                if self.selected_vault_key > 0 {
                                    self.selected_vault_key -= 1;
                                }
                            } else {
                                // Navigating vault list
                                if self.selected_vault > 0 {
                                    self.selected_vault -= 1;
                                }
                            }
                        }
                        _ => {
                            if self.selected_node > 0 {
                                self.selected_node -= 1;
                            }
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    match self.active_view {
                        ActiveView::Vaults => {
                            if self.active_vault.is_some() {
                                // Navigating vault keys
                                let max = self.vault_keys.len().saturating_sub(1);
                                if self.selected_vault_key < max {
                                    self.selected_vault_key += 1;
                                }
                            } else {
                                // Navigating vault list
                                let max = self.vaults.len().saturating_sub(1);
                                if self.selected_vault < max {
                                    self.selected_vault += 1;
                                }
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

                // Refresh
                KeyCode::Char('r') => {
                    self.refresh_cluster_state().await;
                }

                // Cluster operations
                KeyCode::Char('i') => {
                    self.init_cluster().await;
                }

                // Connection commands
                KeyCode::Char('c') => {
                    // Connect to HTTP nodes - enter edit mode to get addresses
                    self.input_mode = InputMode::Editing;
                    self.input_buffer = "http://127.0.0.1:21001".to_string(); // Default suggestion
                    self.set_status(
                        "Enter HTTP node address(es) separated by spaces, then press Enter",
                    );
                }
                KeyCode::Char('t') => {
                    // Connect via Iroh ticket - enter edit mode to get ticket
                    self.input_mode = InputMode::Editing;
                    self.input_buffer.clear();
                    self.set_status("Paste Iroh cluster ticket, then press Enter");
                }

                // Enter editing mode for KV operations
                KeyCode::Enter if self.active_view == ActiveView::KeyValue => {
                    self.input_mode = InputMode::Editing;
                }

                // Vault navigation - Enter to drill into vault, Backspace to go back
                KeyCode::Enter if self.active_view == ActiveView::Vaults => {
                    if self.active_vault.is_none() && !self.vaults.is_empty() {
                        // Enter the selected vault
                        if let Some(vault) = self.vaults.get(self.selected_vault) {
                            let vault_name = vault.name.clone();
                            self.active_vault = Some(vault_name.clone());
                            self.selected_vault_key = 0;
                            self.refresh_vault_keys(&vault_name).await;
                        }
                    }
                }
                KeyCode::Backspace if self.active_view == ActiveView::Vaults => {
                    // Go back to vault list
                    if self.active_vault.is_some() {
                        self.active_vault = None;
                        self.vault_keys.clear();
                        self.selected_vault_key = 0;
                    }
                }

                // Log scroll
                KeyCode::PageUp if self.active_view == ActiveView::Logs => {
                    self.log_scroll = self.log_scroll.saturating_sub(10);
                }
                KeyCode::PageDown if self.active_view == ActiveView::Logs => {
                    self.log_scroll = self.log_scroll.saturating_add(10);
                }

                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                }
                KeyCode::Enter => {
                    // Check if we're doing a connection operation based on status message
                    let is_http_connect = self
                        .status_message
                        .as_ref()
                        .map(|(msg, _)| msg.contains("Enter HTTP node"))
                        .unwrap_or(false);
                    let is_ticket_connect = self
                        .status_message
                        .as_ref()
                        .map(|(msg, _)| msg.contains("Paste Iroh cluster ticket"))
                        .unwrap_or(false);

                    if is_http_connect {
                        // Connect to HTTP nodes
                        self.connect_http_nodes(&self.input_buffer.clone()).await;
                    } else if is_ticket_connect {
                        // Connect via Iroh ticket
                        self.connect_iroh_ticket(&self.input_buffer.clone()).await;
                    } else {
                        // Regular KV operation
                        self.execute_kv_operation().await;
                    }
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                KeyCode::Tab => {
                    // Switch between key and value input
                    std::mem::swap(&mut self.key_buffer, &mut self.input_buffer);
                }
                _ => {}
            },
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
                let connected_node_id = info.node_id;
                let is_http = info.http_addr.starts_with("http://");
                let is_iroh = info.http_addr.starts_with("iroh://");
                new_nodes.insert(info.node_id, info);

                // Try to get metrics - this also triggers discovery for MultiNode client
                if let Ok(metrics) = self.client.get_metrics().await {
                    self.cluster_metrics = Some(metrics.clone());

                    // For MultiNode (Iroh) client, populate all discovered nodes
                    if is_iroh {
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
                                        http_addr: format!("iroh://{}", node_desc.endpoint_addr),
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

                // For HTTP client, try to query other nodes using standard ports
                // This is a temporary solution until we have proper node discovery
                if is_http {
                    // Try standard cluster ports (21001, 21002, 21003)
                    for port in 21001..=21003 {
                        let node_id = port - 21000; // Assume node IDs 1, 2, 3
                        if node_id != connected_node_id {
                            // Create a temporary client for this node
                            let temp_client =
                                AspenClient::new(format!("http://127.0.0.1:{}", port));
                            if let Ok(node_info) = temp_client.get_node_info().await {
                                info!(node_id = node_info.node_id, "discovered node");
                                new_nodes.insert(node_info.node_id, node_info);
                            }
                        }
                    }
                    info!(total_nodes = new_nodes.len(), "nodes discovered via HTTP");
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

        if input.starts_with("get ") {
            let key = input.strip_prefix("get ").unwrap().to_string();
            self.read_key(&key).await;
        } else if input.starts_with("set ") {
            let rest = input.strip_prefix("set ").unwrap();
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

    /// Connect to HTTP nodes.
    async fn connect_http_nodes(&mut self, input: &str) {
        // Parse input - could be single node or space-separated list
        let urls: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();

        if urls.is_empty() {
            self.set_status("No nodes specified");
            return;
        }

        // For now, use the first URL
        // TODO: Support multiple HTTP nodes later
        let url = urls[0].clone();

        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.set_status("Invalid URL format - must start with http:// or https://");
            return;
        }

        // Create new HTTP client
        let client = AspenClient::new(url.clone());
        let new_client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Http(client));

        // Test the connection
        match new_client.get_node_info().await {
            Ok(node_info) => {
                // Connection successful, replace the client
                self.client = new_client;
                self.set_status(&format!(
                    "Connected to node {} at {}",
                    node_info.node_id, url
                ));
                // Immediately refresh to populate nodes
                self.refresh_cluster_state().await;
            }
            Err(e) => {
                self.set_status(&format!("Failed to connect: {}", e));
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
                        let new_client: Arc<dyn ClusterClient> =
                            Arc::new(ClientImpl::MultiNode(multi_client));

                        // Test the connection
                        match new_client.get_node_info().await {
                            Ok(node_info) => {
                                // Connection successful, replace the client
                                self.client = new_client;
                                self.set_status(&format!(
                                    "Connected to cluster via Iroh (node {})",
                                    node_info.node_id
                                ));
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
