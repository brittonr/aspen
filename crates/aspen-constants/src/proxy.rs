//! Cross-cluster proxy constants.
//!
//! Tiger Style: Bounded limits for proxied requests to prevent abuse.

/// Maximum number of proxy hops before rejecting a request.
///
/// Prevents infinite forwarding loops (A → B → C → A).
pub const MAX_PROXY_HOPS: u8 = 3;

/// Timeout for each proxy attempt in seconds.
///
/// Each cluster gets this much time to respond before we try the next one.
pub const PROXY_TIMEOUT_SECS: u64 = 15;

/// Maximum number of target clusters to try when proxying.
///
/// Prevents cascading failures from trying every cluster in the federation.
pub const MAX_PROXY_TARGETS: usize = 3;
