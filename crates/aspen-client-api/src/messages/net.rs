//! Net service mesh RPC message types.

use serde::Deserialize;
use serde::Serialize;

/// Response for service publish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPublishResponse {
    /// Whether the publish succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response for service unpublish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetUnpublishResponse {
    /// Whether the unpublish succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response for service lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetLookupResponse {
    /// The found service entry, if any.
    pub entry: Option<NetServiceInfo>,
    /// Error message if lookup failed.
    pub error: Option<String>,
}

/// Response for service listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetListResponse {
    /// List of matching services.
    pub services: Vec<NetServiceInfo>,
    /// Error message if listing failed.
    pub error: Option<String>,
}

/// Service info returned in responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetServiceInfo {
    /// Service name.
    pub name: String,
    /// Endpoint ID.
    pub endpoint_id: String,
    /// Port.
    pub port: u16,
    /// Protocol.
    pub proto: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Hostname.
    pub hostname: Option<String>,
    /// Published timestamp (unix ms).
    pub published_at_ms: u64,
}
