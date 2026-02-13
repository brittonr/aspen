//! Key validation for reserved prefixes.
//!
//! This module re-exports vault validation from aspen-vault for backward compatibility.

// Re-export all vault types from aspen-vault
pub use aspen_vault::SYSTEM_PREFIX;
pub use aspen_vault::VaultError;
pub use aspen_vault::is_system_key;
pub use aspen_vault::validate_client_key;
