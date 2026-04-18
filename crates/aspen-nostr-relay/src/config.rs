//! Nostr relay configuration.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::constants;

/// Controls whether the relay requires NIP-42 authentication before
/// accepting EVENT submissions over the WebSocket connection.
///
/// This policy only affects external writes from WebSocket clients.
/// Internal writes via `NostrRelayService::publish()` (bridges, plugins)
/// always bypass the policy. Reads (REQ/CLOSE) are unaffected.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WritePolicy {
    /// Any client can submit events (current behavior). No NIP-42 required.
    #[default]
    Open,
    /// Clients must complete NIP-42 authentication before EVENT is accepted.
    AuthRequired,
    /// No external writes allowed. Only `publish()` API works.
    ReadOnly,
}

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

    /// Write policy controlling EVENT acceptance from WebSocket clients.
    ///
    /// - `open`: Any client can write (default, backward-compatible).
    /// - `auth_required`: Must complete NIP-42 before EVENT is accepted.
    /// - `read_only`: No external writes; only `publish()` API works.
    #[serde(default = "WritePolicy::default")]
    pub write_policy: WritePolicy,

    /// Public relay URL used for NIP-42 challenge verification.
    ///
    /// The kind 22242 auth event must contain a `relay` tag matching this
    /// URL. If `None`, the relay URL check is skipped.
    #[serde(default = "no_relay_url")]
    pub relay_url: Option<String>,

    /// Maximum EVENT submissions per second per source IP. Set to 0 to disable.
    #[serde(default = "default_events_per_second_per_ip")]
    pub events_per_second_per_ip: u32,

    /// Maximum burst of EVENT submissions per source IP.
    #[serde(default = "default_events_burst_per_ip")]
    pub events_burst_per_ip: u32,

    /// Maximum EVENT submissions per second per author pubkey. Set to 0 to disable.
    #[serde(default = "default_events_per_second_per_pubkey")]
    pub events_per_second_per_pubkey: u32,

    /// Maximum burst of EVENT submissions per author pubkey.
    #[serde(default = "default_events_burst_per_pubkey")]
    pub events_burst_per_pubkey: u32,
}

fn no_relay_url() -> Option<String> {
    None
}

fn default_events_per_second_per_ip() -> u32 {
    constants::MAX_EVENTS_PER_SECOND_PER_IP
}
fn default_events_burst_per_ip() -> u32 {
    constants::MAX_EVENTS_BURST_PER_IP
}
fn default_events_per_second_per_pubkey() -> u32 {
    constants::MAX_EVENTS_PER_SECOND_PER_PUBKEY
}
fn default_events_burst_per_pubkey() -> u32 {
    constants::MAX_EVENTS_BURST_PER_PUBKEY
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
            write_policy: WritePolicy::default(),
            relay_url: None,
            events_per_second_per_ip: constants::MAX_EVENTS_PER_SECOND_PER_IP,
            events_burst_per_ip: constants::MAX_EVENTS_BURST_PER_IP,
            events_per_second_per_pubkey: constants::MAX_EVENTS_PER_SECOND_PER_PUBKEY,
            events_burst_per_pubkey: constants::MAX_EVENTS_BURST_PER_PUBKEY,
        }
    }
}
