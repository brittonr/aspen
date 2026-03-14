//! Nostr relay configuration.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::constants;

/// Configuration for the Nostr relay service.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NostrRelayConfig {
    /// Whether the relay is enabled.
    pub enabled: bool,

    /// Bind address for the WebSocket listener.
    pub bind_addr: String,

    /// TCP port for the WebSocket listener.
    pub bind_port: u16,

    /// Maximum concurrent WebSocket connections.
    pub max_connections: u32,

    /// Maximum subscriptions per connection.
    pub max_subscriptions_per_connection: u32,

    /// Maximum event size in bytes.
    pub max_event_size_bytes: u32,
}

impl Default for NostrRelayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_addr: constants::DEFAULT_NOSTR_BIND_ADDR.to_string(),
            bind_port: constants::DEFAULT_NOSTR_PORT,
            max_connections: constants::MAX_NOSTR_CONNECTIONS,
            max_subscriptions_per_connection: constants::MAX_SUBSCRIPTIONS_PER_CONNECTION,
            max_event_size_bytes: constants::MAX_EVENT_SIZE,
        }
    }
}
