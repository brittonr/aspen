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
