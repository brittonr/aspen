pub const CONTENT_ADAPTER_PROFILE_SCHEMA: &str = "molten.content-store-adapter.profile.v1";
pub const CONTENT_COMMAND_SCHEMA: &str = "molten.content-store-adapter.command.v1";
pub const CONTENT_EVENT_SCHEMA: &str = "molten.content-store-adapter.event.v1";
pub const CONTENT_PARTIAL_STATE_SCHEMA: &str = "molten.content-store-adapter.partial-state.v1";
pub const CONTENT_STATUS_SCHEMA: &str = "molten.content-store-adapter.status.v1";
pub const CONTENT_PROTECTION_AUTHORITY_SCHEMA: &str = "molten.content-store-adapter.protection-authority.v1";

pub(crate) const CONTENT_ADJACENT_PAIR_WIDTH: usize = 2;
const REQUIRED_NON_CLAIM_COUNT: usize = 7;

pub const REQUIRED_CONTENT_NON_CLAIMS: [ContentNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    ContentNonClaim::BackendHintIsNotIdentity,
    ContentNonClaim::TransportIsNotImportAuthority,
    ContentNonClaim::ProtectionIsNotRetentionAuthority,
    ContentNonClaim::PinIsNotReadAuthority,
    ContentNonClaim::UnprotectIsNotDeleteAuthority,
    ContentNonClaim::AvailabilityIsNotExecutionAuthority,
    ContentNonClaim::VerificationIsNotProvenance,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentNonClaim {
    BackendHintIsNotIdentity,
    TransportIsNotImportAuthority,
    ProtectionIsNotRetentionAuthority,
    PinIsNotReadAuthority,
    UnprotectIsNotDeleteAuthority,
    AvailabilityIsNotExecutionAuthority,
    VerificationIsNotProvenance,
}

impl ContentNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BackendHintIsNotIdentity => "backend-hint-is-not-identity",
            Self::TransportIsNotImportAuthority => "transport-is-not-import-authority",
            Self::ProtectionIsNotRetentionAuthority => "protection-is-not-retention-authority",
            Self::PinIsNotReadAuthority => "pin-is-not-read-authority",
            Self::UnprotectIsNotDeleteAuthority => "unprotect-is-not-delete-authority",
            Self::AvailabilityIsNotExecutionAuthority => "availability-is-not-execution-authority",
            Self::VerificationIsNotProvenance => "verification-is-not-provenance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentAdapterClass {
    CapabilityLocal,
    RedbIndexed,
    IrohBlobs,
    DeterministicSimulation,
}

impl ContentAdapterClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityLocal => "capability-local",
            Self::RedbIndexed => "redb-indexed",
            Self::IrohBlobs => "iroh-blobs",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentCapability {
    StreamingPut,
    StreamingGet,
    VerifiedRange,
    Availability,
    Import,
    Export,
    Protection,
    DurableCompletion,
}

impl ContentCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StreamingPut => "streaming-put",
            Self::StreamingGet => "streaming-get",
            Self::VerifiedRange => "verified-range",
            Self::Availability => "availability",
            Self::Import => "import",
            Self::Export => "export",
            Self::Protection => "protection",
            Self::DurableCompletion => "durable-completion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentResourceBounds {
    pub max_total_bytes: u64,
    pub max_chunk_count: usize,
    pub max_chunk_bytes: u64,
    pub max_range_bytes: u64,
    pub max_concurrent_operations: usize,
    pub max_queued_bytes: u64,
    pub max_memory_bytes: u64,
    pub max_deadline_ticks: u64,
    pub max_retries: u32,
    pub max_events: usize,
    pub max_status_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAdapterProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub class: ContentAdapterClass,
    pub capabilities: Vec<ContentCapability>,
    pub bounds: ContentResourceBounds,
    pub supported_transforms: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ContentNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentChunkDescriptor {
    pub chunk_ref: String,
    pub length: u64,
    pub position: usize,
    pub transform: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentManifestDescriptor {
    pub manifest_ref: String,
    pub total_length: u64,
    pub chunker: String,
    pub chunk_size: u64,
    pub metadata_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub chunks: Vec<ContentChunkDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentOperation {
    Put,
    Get,
    RangeRead,
    Availability,
    Import,
    Export,
    Protect,
    Unprotect,
    Cancel,
}

impl ContentOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Put => "put",
            Self::Get => "get",
            Self::RangeRead => "range-read",
            Self::Availability => "availability",
            Self::Import => "import",
            Self::Export => "export",
            Self::Protect => "protect",
            Self::Unprotect => "unprotect",
            Self::Cancel => "cancel",
        }
    }

    pub const fn required_capability(self) -> Option<ContentCapability> {
        match self {
            Self::Put => Some(ContentCapability::StreamingPut),
            Self::Get => Some(ContentCapability::StreamingGet),
            Self::RangeRead => Some(ContentCapability::VerifiedRange),
            Self::Availability => Some(ContentCapability::Availability),
            Self::Import => Some(ContentCapability::Import),
            Self::Export => Some(ContentCapability::Export),
            Self::Protect | Self::Unprotect => Some(ContentCapability::Protection),
            Self::Cancel => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCommand {
    pub schema: String,
    pub operation_ref: String,
    pub adapter_ref: String,
    pub operation: ContentOperation,
    pub manifest_ref: String,
    pub range: Option<ContentRange>,
    pub expected_bytes: u64,
    pub expected_chunks: usize,
    pub submitted_tick: u64,
    pub deadline_tick: u64,
    pub retry_count: u32,
    pub cancelled: bool,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentTerminal {
    Accepted,
    Streaming,
    Verified,
    Durable,
    Cancelled,
    Retryable,
    Failed,
    Uncertain,
    Denied,
}

impl ContentTerminal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Streaming => "streaming",
            Self::Verified => "verified",
            Self::Durable => "durable",
            Self::Cancelled => "cancelled",
            Self::Retryable => "retryable",
            Self::Failed => "failed",
            Self::Uncertain => "uncertain",
            Self::Denied => "denied",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Verified
                | Self::Durable
                | Self::Cancelled
                | Self::Retryable
                | Self::Failed
                | Self::Uncertain
                | Self::Denied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentFailure {
    CorruptChunk,
    TruncatedChunk,
    ReorderedChunk,
    UnexpectedChunk,
    StaleTicket,
    UnsupportedTransform,
    RootEscape,
    Overload,
    PermissionDenied,
    Timeout,
    TransportDisconnected,
    AdapterFailure,
}

impl ContentFailure {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CorruptChunk => "corrupt-chunk",
            Self::TruncatedChunk => "truncated-chunk",
            Self::ReorderedChunk => "reordered-chunk",
            Self::UnexpectedChunk => "unexpected-chunk",
            Self::StaleTicket => "stale-ticket",
            Self::UnsupportedTransform => "unsupported-transform",
            Self::RootEscape => "root-escape",
            Self::Overload => "overload",
            Self::PermissionDenied => "permission-denied",
            Self::Timeout => "timeout",
            Self::TransportDisconnected => "transport-disconnected",
            Self::AdapterFailure => "adapter-failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPartialState {
    pub schema: String,
    pub operation_ref: String,
    pub manifest_ref: String,
    pub profile_ref: String,
    pub generation: u64,
    pub terminal: ContentTerminal,
    pub verified_chunk_refs: Vec<String>,
    pub missing_chunk_refs: Vec<String>,
    pub verified_bytes: u64,
    pub event_count: usize,
    pub last_sequence: Option<u64>,
    pub failure: Option<ContentFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentChunkObservation {
    pub operation_ref: String,
    pub manifest_ref: String,
    pub sequence: u64,
    pub chunk_ref: String,
    pub position: usize,
    pub observed_content_ref: String,
    pub observed_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEvent {
    pub schema: String,
    pub operation_ref: String,
    pub manifest_ref: String,
    pub sequence: u64,
    pub terminal: ContentTerminal,
    pub chunk_ref: Option<String>,
    pub observed_bytes: u64,
    pub failure: Option<ContentFailure>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ContentNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPreflight {
    pub terminal: ContentTerminal,
    pub required_chunk_refs: Vec<String>,
    pub issues: Vec<ContentIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAdapterStatus {
    pub schema: String,
    pub profile_ref: String,
    pub class: ContentAdapterClass,
    pub generation: u64,
    pub active_operations: usize,
    pub queued_bytes: u64,
    pub terminal_counts: Vec<(ContentTerminal, u64)>,
    pub backend_hint_ref: Option<String>,
    pub issues: Vec<ContentIssue>,
    pub non_claims: Vec<ContentNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentProtectionAuthority {
    pub schema: String,
    pub operation_ref: String,
    pub manifest_ref: String,
    pub retention_policy_ref: String,
    pub canonical_pin_ref: Option<String>,
    pub read_authority_ref: Option<String>,
    pub deletion_gate_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAuthorityDecision {
    Admit,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentIssue {
    SchemaMismatch(&'static str),
    EmptyField(&'static str),
    MalformedToken(&'static str),
    MalformedRef(&'static str),
    ZeroBound(&'static str),
    DuplicateValue(&'static str),
    MissingNonClaim(&'static str),
    ProfileMismatch,
    AdapterMismatch,
    ManifestMismatch,
    UnsupportedCapability,
    UnsupportedTransform(String),
    TotalBytesExceeded,
    ChunkCountExceeded,
    ChunkBytesExceeded,
    RangeExceeded,
    ConcurrencyExceeded,
    QueueExceeded,
    MemoryExceeded,
    DeadlineExceeded,
    RetryExceeded,
    EventLimitExceeded,
    ArithmeticOverflow,
    Cancelled,
    PartialStateMismatch,
    UnexpectedChunk(String),
    ReorderedChunk(String),
    CorruptChunk(String),
    TruncatedChunk(String),
    DuplicateChunk(String),
    BackendHintCannotReplaceIdentity,
    ProtectionCannotGrantAuthority,
}
