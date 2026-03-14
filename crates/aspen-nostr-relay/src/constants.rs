//! Tiger Style resource bounds for the Nostr relay.

/// Maximum concurrent WebSocket connections to the relay.
pub const MAX_NOSTR_CONNECTIONS: u32 = 256;

/// Maximum subscriptions a single connection can hold.
pub const MAX_SUBSCRIPTIONS_PER_CONNECTION: u32 = 16;

/// Maximum filters within a single REQ subscription.
pub const MAX_FILTERS_PER_SUBSCRIPTION: u32 = 8;

/// Maximum event JSON size in bytes (64 KB).
pub const MAX_EVENT_SIZE: u32 = 64 * 1024;

/// Capacity of the broadcast channel for real-time event fan-out.
pub const BROADCAST_CHANNEL_CAPACITY: u32 = 4096;

/// Maximum stored events before eviction kicks in.
pub const MAX_STORED_EVENTS: u32 = 100_000;

/// Default TCP port for the Nostr relay WebSocket listener.
pub const DEFAULT_NOSTR_PORT: u16 = 4869;

/// Default bind address for the relay listener.
pub const DEFAULT_NOSTR_BIND_ADDR: &str = "127.0.0.1";

/// KV prefix for event data: `nostr:ev:{event_id}` → event JSON.
pub const KV_PREFIX_EVENT: &str = "nostr:ev:";

/// KV prefix for kind index: `nostr:ki:{kind}:{created_at_be}:{event_id}`.
pub const KV_PREFIX_KIND: &str = "nostr:ki:";

/// KV prefix for author index: `nostr:au:{author_hex}:{created_at_be}:{event_id}`.
pub const KV_PREFIX_AUTHOR: &str = "nostr:au:";

/// KV prefix for tag index: `nostr:tg:{tag_name}:{tag_value}:{event_id}`.
pub const KV_PREFIX_TAG: &str = "nostr:tg:";

/// KV key for the stored event counter.
pub const KV_EVENT_COUNT: &str = "nostr:meta:count";
