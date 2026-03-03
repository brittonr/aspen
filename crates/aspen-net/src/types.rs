//! Service mesh types.
//!
//! Core data structures for the aspen-net service registry.

use serde::Deserialize;
use serde::Serialize;

/// A published service in the mesh.
///
/// Services are registered by name and discovered via the registry.
/// Each entry maps a human-readable name to an iroh endpoint + port.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceEntry {
    /// Unique service name (e.g., "mydb", "web-frontend").
    pub name: String,
    /// Iroh endpoint ID of the node hosting this service.
    pub endpoint_id: String,
    /// Local port the service listens on.
    pub port: u16,
    /// Protocol (e.g., "tcp").
    pub proto: String,
    /// Tags for filtering and grouping.
    pub tags: Vec<String>,
    /// Optional hostname for DNS resolution.
    pub hostname: Option<String>,
    /// Timestamp when this service was published (unix ms).
    pub published_at_ms: u64,
}

/// A node with published services.
///
/// Tracks which services a given iroh endpoint is hosting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeEntry {
    /// Iroh endpoint ID.
    pub endpoint_id: String,
    /// Node hostname.
    pub hostname: String,
    /// Node-level tags.
    pub tags: Vec<String>,
    /// Names of services published by this node.
    pub services: Vec<String>,
    /// Last seen timestamp (unix ms).
    pub last_seen_ms: u64,
}
