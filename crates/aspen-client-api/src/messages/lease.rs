//! Lease operation types.
//!
//! Request/response types for time-based resource management with leases.

use serde::Deserialize;
use serde::Serialize;

/// Lease domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaseRequest {
    /// Grant a new lease with specified TTL.
    LeaseGrant { ttl_seconds: u32, lease_id: Option<u64> },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke { lease_id: u64 },
    /// Refresh a lease's TTL (keepalive).
    LeaseKeepalive { lease_id: u64 },
    /// Get lease information including TTL and attached keys.
    LeaseTimeToLive { lease_id: u64, include_keys: bool },
    /// List all active leases.
    LeaseList,
}

impl LeaseRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::LeaseGrant { .. } | Self::LeaseRevoke { .. } | Self::LeaseKeepalive { .. } => {
                Some(Operation::Write {
                    key: "_lease:".to_string(),
                    value: vec![],
                })
            }
            Self::LeaseTimeToLive { .. } | Self::LeaseList => Some(Operation::Read {
                key: "_lease:".to_string(),
            }),
        }
    }
}

/// Lease grant result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGrantResultResponse {
    /// Whether the lease was granted.
    pub success: bool,
    /// Unique lease ID.
    pub lease_id: Option<u64>,
    /// Granted TTL in seconds.
    pub ttl_seconds: Option<u32>,
    /// Error message if grant failed.
    pub error: Option<String>,
}

/// Lease revoke result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRevokeResultResponse {
    /// Whether the lease was revoked.
    pub success: bool,
    /// Number of keys deleted with the lease.
    pub keys_deleted: Option<u32>,
    /// Error message if revoke failed.
    pub error: Option<String>,
}

/// Lease keepalive result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseKeepaliveResultResponse {
    /// Whether the keepalive succeeded.
    pub success: bool,
    /// Lease ID that was refreshed.
    pub lease_id: Option<u64>,
    /// New TTL in seconds.
    pub ttl_seconds: Option<u32>,
    /// Error message if keepalive failed.
    pub error: Option<String>,
}

/// Lease time-to-live result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTimeToLiveResultResponse {
    /// Whether the query succeeded.
    pub success: bool,
    /// Lease ID queried.
    pub lease_id: Option<u64>,
    /// Original TTL in seconds.
    pub granted_ttl_seconds: Option<u32>,
    /// Remaining TTL in seconds.
    pub remaining_ttl_seconds: Option<u32>,
    /// Keys attached to the lease.
    pub keys: Option<Vec<String>>,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Lease list result response.
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
