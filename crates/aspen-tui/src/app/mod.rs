//! Application state and logic for the Aspen TUI.
//!
//! Implements The Elm Architecture (TEA) pattern with:
//! - Model: `App` struct holding all application state
//! - Update: `App::handle_event()` for state transitions
//! - View: Handled by `ui` module

mod ci_ops;
mod cluster_ops;
mod events;
mod job_ops;
mod kv_ops;
mod navigation;
mod sql_ops;
mod state;
mod types;
mod vault_ops;

pub use state::App;
pub use types::ActiveView;
pub use types::ClusterOperation;
pub use types::InputMode;
