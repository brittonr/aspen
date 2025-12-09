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

use crate::client::{AspenClient, ClusterMetrics, NodeInfo};
use crate::client_trait::{ClientImpl, ClusterClient};
use crate::event::Event;
use crate::iroh_client::{IrohClient, parse_cluster_ticket};

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
            Self::KeyValue => Self::Logs,
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
            Self::Logs => Self::KeyValue,
            Self::Help => Self::Logs,
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cluster => "Cluster",
            Self::Metrics => "Metrics",
            Self::KeyValue => "Key-Value",
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
        }
    }

    /// Create new application state with Iroh client.
    pub async fn new_with_iroh(
        ticket: String,
        debug_mode: bool,
        max_display_nodes: usize,
    ) -> Result<Self> {
        // Parse the ticket to get endpoint address
        let endpoint_addr =
            parse_cluster_ticket(&ticket).map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;

        // Create Iroh client
        let iroh_client = IrohClient::new(endpoint_addr)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("{:#}", e))?;
        let client: Arc<dyn ClusterClient> = Arc::new(ClientImpl::Iroh(iroh_client));

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
        })
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
            self.refresh_cluster_state().await;
        }

        // Clear old status messages (after 5 seconds)
        if let Some((_, timestamp)) = &self.status_message {
            if timestamp.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }
    }

    /// Handle key event.
    async fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match self.input_mode {
            InputMode::Normal => match key.code {
                // Quit
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.should_quit = true;
                }

                // View navigation
                KeyCode::Tab => {
                    self.active_view = self.active_view.next();
                }
                KeyCode::BackTab => {
                    self.active_view = self.active_view.prev();
                }
                KeyCode::Char('1') => self.active_view = ActiveView::Cluster,
                KeyCode::Char('2') => self.active_view = ActiveView::Metrics,
                KeyCode::Char('3') => self.active_view = ActiveView::KeyValue,
                KeyCode::Char('4') => self.active_view = ActiveView::Logs,
                KeyCode::Char('?') => self.active_view = ActiveView::Help,

                // List navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_node > 0 {
                        self.selected_node -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self.nodes.len().saturating_sub(1);
                    if self.selected_node < max {
                        self.selected_node += 1;
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

                // Enter editing mode for KV operations
                KeyCode::Enter if self.active_view == ActiveView::KeyValue => {
                    self.input_mode = InputMode::Editing;
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
                    self.execute_kv_operation().await;
                    self.input_mode = InputMode::Normal;
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
    async fn refresh_cluster_state(&mut self) {
        self.refreshing = true;
        let mut new_nodes = BTreeMap::new();

        // First, get info from the connected node
        match self.client.get_node_info().await {
            Ok(info) => {
                let connected_node_id = info.node_id;
                let is_http = info.http_addr.starts_with("http://");
                new_nodes.insert(info.node_id, info);

                // Try to get metrics to discover other nodes
                if let Ok(metrics) = self.client.get_metrics().await {
                    self.cluster_metrics = Some(metrics);
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
                    info!(total_nodes = new_nodes.len(), "nodes discovered");
                }

                // For Iroh client, we currently only show the connected node
                // TODO: Implement multi-node discovery for Iroh P2P connections
            }
            Err(e) => {
                warn!(error = %e, "failed to get node info");
                self.set_status(&format!("Failed to refresh: {}", e));
            }
        }

        self.nodes = new_nodes;
        self.last_refresh = Some(Instant::now());
        self.refreshing = false;
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
