//! API modules for the application

#[cfg(feature = "tofu-support")]
pub mod tofu_handlers;

// Re-export AppState from state module for convenience in handlers
pub use crate::state::AppState;