//! Tiger Style constants for capability-based authorization.
//!
//! These constants define fixed limits to prevent resource exhaustion
//! and ensure predictable token sizes.

/// Maximum number of capabilities per token (32).
///
/// Tiger Style: Bounded to prevent token bloat and DoS.
pub const MAX_CAPABILITIES_PER_TOKEN: u32 = 32;

/// Maximum delegation chain depth (8 levels).
///
/// Tiger Style: Bounded to prevent unbounded proof chains.
/// Root -> Service -> User -> ... max 8 levels
pub const MAX_DELEGATION_DEPTH: u8 = 8;

/// Maximum token size in bytes (8 KB).
///
/// Tiger Style: Bounded to prevent oversized tokens.
/// Typical token with 10 capabilities is ~500 bytes.
pub const MAX_TOKEN_SIZE: u32 = 8 * 1024;

/// Maximum revocation list size (10,000 entries).
///
/// Tiger Style: Bounded to prevent unbounded memory growth.
/// Old revocations can be pruned after token expiry.
pub const MAX_REVOCATION_LIST_SIZE: u32 = 10_000;

/// Token clock skew tolerance (60 seconds).
///
/// Tiger Style: Fixed tolerance for clock drift between nodes.
pub const TOKEN_CLOCK_SKEW_SECS: u64 = 60;

/// Signed token fact required for explicit cross-cluster proxy bearer tokens.
pub const FEDERATION_PROXY_FACT_KEY: &str = "aspen:federation-proxy";

/// Value for [`FEDERATION_PROXY_FACT_KEY`] in v1 proxy bearer tokens.
pub const FEDERATION_PROXY_FACT_VALUE: &[u8] = b"v1";

/// Maximum lifetime for explicit federation proxy bearer delegations: 15 minutes.
pub const MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS: u64 = 15 * 60;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Capability limits must be positive
const _: () = assert!(MAX_CAPABILITIES_PER_TOKEN > 0);
const _: () = assert!(MAX_DELEGATION_DEPTH > 0);
const _: () = assert!(MAX_TOKEN_SIZE > 0);
const _: () = assert!(MAX_REVOCATION_LIST_SIZE > 0);
const _: () = assert!(TOKEN_CLOCK_SKEW_SECS > 0);
const _: () = assert!(MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS > 0);
