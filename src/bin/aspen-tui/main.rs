//! Aspen TUI - Terminal User Interface for cluster management.
//!
//! Interactive terminal application for monitoring and managing Aspen clusters.
//! Uses ratatui for the UI framework with crossterm backend.
//!
//! # Architecture
//!
//! Follows The Elm Architecture (TEA) pattern:
//! - Model: Application state in `App` struct
//! - Messages: User events and async updates via `Message` enum
//! - Update: State transitions in `App::update()`
//! - View: Rendering in `App::draw()`
//!
//! # Features
//!
//! - Real-time cluster status monitoring
//! - Node health and metrics visualization
//! - Cluster initialization and membership changes
//! - Key-value store operations
//! - Log viewing with tracing integration
//!
//! # Tiger Style
//!
//! - Fixed-size buffers for event channels (bounded queues)
//! - Explicit error handling with color-eyre
//! - Graceful degradation on network failures
//! - Clean shutdown with resource cleanup

mod app;
mod client;
mod client_trait;
mod event;
mod iroh_client;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tracing::info;

use app::App;
use event::EventHandler;

/// Default refresh interval for metrics polling (milliseconds).
const DEFAULT_REFRESH_INTERVAL_MS: u64 = 1000;

/// Maximum number of nodes to display in the cluster view.
const MAX_DISPLAY_NODES: usize = 50;

#[derive(Parser, Debug)]
#[command(name = "aspen-tui")]
#[command(about = "Terminal UI for Aspen cluster management")]
struct Args {
    /// HTTP API addresses of Aspen nodes to connect to.
    /// Can be specified multiple times.
    /// If not specified, TUI starts disconnected and can connect later.
    #[arg(short, long)]
    nodes: Vec<String>,

    /// Aspen cluster ticket for Iroh P2P connection.
    /// Format: "aspen{base32-encoded-data}"
    #[arg(short, long)]
    ticket: Option<String>,

    /// Refresh interval in milliseconds for metrics polling.
    #[arg(short, long, default_value_t = DEFAULT_REFRESH_INTERVAL_MS)]
    refresh: u64,

    /// Enable debug mode with additional logging panel.
    #[arg(short, long)]
    debug: bool,
}

/// Initialize the terminal for TUI rendering.
///
/// Sets up raw mode, alternate screen, and mouse capture.
/// Returns the terminal instance.
///
/// Tiger Style: Focused function for terminal setup.
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal state.
///
/// Disables raw mode, leaves alternate screen, and disables mouse capture.
///
/// Tiger Style: Focused function for terminal cleanup.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Initialize tracing subscriber with TUI-compatible output.
///
/// Routes logs to tui-logger for display in the TUI log panel.
fn init_tracing() {
    use tracing_subscriber::prelude::*;

    // Initialize tui-logger
    tui_logger::init_logger(log::LevelFilter::Debug).ok();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // Set up tracing subscriber to route to tui-logger
    let tui_layer = tui_logger::tracing_subscriber_layer();

    tracing_subscriber::registry().with(tui_layer).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre panic and error hooks
    color_eyre::install()?;

    let args = Args::parse();

    // Initialize tracing for TUI
    init_tracing();

    info!(
        nodes = ?args.nodes,
        ticket = ?args.ticket,
        refresh_ms = args.refresh,
        "starting aspen-tui"
    );

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create application state
    let app = if let Some(ticket) = args.ticket {
        // Use Iroh P2P connection
        App::new_with_iroh(ticket, args.debug, MAX_DISPLAY_NODES).await?
    } else if !args.nodes.is_empty() {
        // Use HTTP connections
        App::new(args.nodes, args.debug, MAX_DISPLAY_NODES)
    } else {
        // Start disconnected - can connect later
        info!("starting in disconnected mode - use 'c' to connect");
        App::new_disconnected(args.debug, MAX_DISPLAY_NODES)
    };

    // Create event handler with tick interval
    let tick_rate = Duration::from_millis(args.refresh);
    let event_handler = EventHandler::new(tick_rate);

    // Run the main loop
    let result = run(&mut terminal, app, event_handler).await;

    // Restore terminal
    restore_terminal(&mut terminal)?;

    result
}

/// Main application loop.
///
/// Handles events and renders UI until quit is requested.
///
/// Tiger Style: Main control flow in a single function.
async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut event_handler: EventHandler,
) -> Result<()> {
    while !app.should_quit {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Handle events
        if let Some(event) = event_handler.next().await {
            app.handle_event(event).await?;
        }
    }

    Ok(())
}
