//! API modules for the application

pub mod tofu_handlers;

// Re-export AppState from state module for convenience in handlers
pub use crate::state::AppState;