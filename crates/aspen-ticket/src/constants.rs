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
