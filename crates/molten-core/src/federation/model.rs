pub const MAX_FEDERATION_ENTRIES: usize = 4096;
pub const MAX_FEDERATION_PEERS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordPurpose {
    Announcement,
    Inventory,
    Delegate,
    Request,
    Response,
    Receipt,
}
impl RecordPurpose {
    pub const fn domain(self) -> &'static str {
        match self {
            Self::Announcement => "molten.federation.announcement.v1",
            Self::Inventory => "molten.federation.inventory.v1",
            Self::Delegate => "molten.federation.delegate.v1",
            Self::Request => "molten.federation.request.v1",
            Self::Response => "molten.federation.response.v1",
            Self::Receipt => "molten.federation.receipt.v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorSource {
    Static,
    Endpoint,
    Gossip,
    Tracker,
    Dht,
    Pkarr,
    Probe,
    Catalog,
}

/// Observations locate candidates only; none are admission evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHint {
    pub peer_ref: String,
    pub signer_ref: String,
    pub source: LocatorSource,
    pub observed_at: u64,
    pub expires_at: u64,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub name: String,
    pub head_ref: String,
    pub ancestry_ref: String,
    pub policy_ref: String,
    pub bytes: u64,
    pub dag_nodes: u64,
    pub chunks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullBounds {
    pub inventory_entries: usize,
    pub inventory_bytes: u64,
    pub fetch_bytes: u64,
    pub dag_nodes: u64,
    pub content_chunks: u64,
    pub concurrent_sessions: usize,
    pub query_interval: u64,
    pub freshness: u64,
    pub session_ticks: u64,
    pub retries: u32,
    pub retained_status: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryObservation {
    pub peer_ref: String,
    pub signer_ref: String,
    pub generation: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub encoded_bytes: u64,
    pub entries: Vec<InventoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullInput<'a> {
    pub bounds: &'a PullBounds,
    pub configured: &'a [PeerHint],
    pub inventory: &'a InventoryObservation,
    pub local: &'a [InventoryEntry],
    pub generation: u64,
    pub now: u64,
    pub last_query: Option<u64>,
    pub resource_history: &'a [ResourceHistory],
    pub active_sessions: usize,
    pub receiver_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub name: String,
    pub local_ref: String,
    pub remote: InventoryEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullPlan {
    pub peer_ref: String,
    pub signer_ref: String,
    pub inventory_issued_at: u64,
    pub inventory_expires_at: u64,
    pub generation: u64,
    pub deadline: u64,
    pub missing: Vec<InventoryEntry>,
    pub conflicts: Vec<Conflict>,
    pub bytes: u64,
    pub dag_nodes: u64,
    pub chunks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullIssue {
    Bounds,
    Unsolicited,
    UnknownPeer,
    Unavailable,
    Stale,
    Generation,
    RateLimited,
    Sessions,
    Inventory,
    DuplicateName,
    Resource,
    Clock,
    ResourceRateLimited,
    ResourceRetry,
    ResourceHistory,
}

/// Each gate must be obtained locally for this exact head and session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPrerequisites {
    pub schema_ref: String,
    pub policy_ref: String,
    pub capability_ref: String,
    pub provenance_ref: String,
    pub resource_ref: String,
    pub retention_ref: String,
    pub import_ref: String,
}

/// Receiver-owned bounded reservations, keyed by content identity rather than peer or session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHistory {
    pub head_ref: String,
    pub window_started: u64,
    pub last_query: u64,
    pub attempts: u32,
    pub bytes: u64,
    pub dag_nodes: u64,
    pub chunks: u64,
}
