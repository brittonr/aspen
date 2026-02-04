//! Constants for the Aspen client library.

// Re-export constants from aspen-client-rpc
pub use aspen_client_api::CLIENT_ALPN;
pub use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;

// Additional client-specific constants
pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_MS: u64 = 100;

// Re-export overlay constants
pub use crate::overlay_constants::*;
