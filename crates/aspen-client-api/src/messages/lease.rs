//! Lease operation response types.
//!
//! Response types for time-based resource management with leases.

use serde::Deserialize;
use serde::Serialize;

/// Lease grant result response.
///
/// Returned when a new lease is granted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGrantResultResponse {
    /// Whether the lease was granted.
    pub success: bool,
    /// Unique lease ID (client-provided or server-generated).
    pub lease_id: Option<u64>,
    /// Granted TTL in seconds.
    pub ttl_seconds: Option<u32>,
    /// Error message if grant failed.
    pub error: Option<String>,
}

/// Lease revoke result response.
///
/// Returned when a lease is revoked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRevokeResultResponse {
    /// Whether the lease was revoked.
    pub success: bool,
    /// Number of keys deleted with the lease.
    pub keys_deleted: Option<u32>,
    /// Error message if revoke failed (e.g., lease not found).
    pub error: Option<String>,
}

/// Lease keepalive result response.
///
/// Returned when a lease is refreshed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseKeepaliveResultResponse {
    /// Whether the keepalive succeeded.
    pub success: bool,
    /// Lease ID that was refreshed.
    pub lease_id: Option<u64>,
    /// New TTL in seconds (reset to original TTL).
    pub ttl_seconds: Option<u32>,
    /// Error message if keepalive failed (e.g., lease not found or expired).
    pub error: Option<String>,
}

/// Lease time-to-live result response.
///
/// Returns lease metadata including remaining TTL and attached keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTimeToLiveResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Lease ID queried.
    pub lease_id: Option<u64>,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: Option<u32>,
    /// Remaining TTL in seconds (0 if expired).
    pub remaining_ttl_seconds: Option<u32>,
    /// Keys attached to the lease (if include_keys was true).
    pub keys: Option<Vec<String>>,
    /// Error message if query failed (e.g., lease not found).
    pub error: Option<String>,
}

/// Lease list result response.
///
/// Returns all active leases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseListResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// List of active leases.
    pub leases: Option<Vec<LeaseInfo>>,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Information about an active lease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    /// Unique lease ID.
    pub lease_id: u64,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: u32,
    /// Remaining TTL in seconds.
    pub remaining_ttl_seconds: u32,
    /// Number of keys attached to this lease.
    pub attached_keys: u32,
}
