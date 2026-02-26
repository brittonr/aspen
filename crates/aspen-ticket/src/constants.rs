//! Version constants and protocol parameters for Aspen cluster tickets.

/// Current signed ticket protocol version.
///
/// Version history:
/// - v1: Initial signed ticket format with Ed25519 signatures
pub(crate) const SIGNED_TICKET_VERSION: u8 = 1;

/// Default ticket validity duration (24 hours).
pub(crate) const DEFAULT_TICKET_VALIDITY_SECS: u64 = 24 * 3600;

/// Clock skew tolerance for timestamp validation (5 minutes).
pub(crate) const CLOCK_SKEW_TOLERANCE_SECS: u64 = 300;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Protocol version must be positive
const _: () = assert!(SIGNED_TICKET_VERSION > 0);

// Ticket validity must be positive and reasonable
const _: () = assert!(DEFAULT_TICKET_VALIDITY_SECS > 0);
const _: () = assert!(DEFAULT_TICKET_VALIDITY_SECS <= 7 * 24 * 3600); // max 1 week (sanity check)

// Clock skew tolerance must be positive and reasonable
const _: () = assert!(CLOCK_SKEW_TOLERANCE_SECS > 0);
const _: () = assert!(CLOCK_SKEW_TOLERANCE_SECS < DEFAULT_TICKET_VALIDITY_SECS);
