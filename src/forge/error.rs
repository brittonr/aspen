//! Error types for the Forge module.
//!
//! Uses `snafu` for structured error handling with context.

use snafu::Snafu;

use crate::api::KeyValueStoreError;

/// Errors that can occur in Forge operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ForgeError {
    // ========================================================================
    // Git Object Errors
    // ========================================================================
    /// Object not found in storage.
    #[snafu(display("object not found: {hash}"))]
    ObjectNotFound { hash: String },

    /// Object size exceeds maximum allowed.
    #[snafu(display("object size {size} exceeds maximum {max}"))]
    ObjectTooLarge { size: u64, max: u64 },

    /// Invalid object format.
    #[snafu(display("invalid object format: {message}"))]
    InvalidObject { message: String },

    /// Too many tree entries.
    #[snafu(display("tree has {count} entries, maximum is {max}"))]
    TooManyTreeEntries { count: u32, max: u32 },

    /// Too many commit parents.
    #[snafu(display("commit has {count} parents, maximum is {max}"))]
    TooManyParents { count: u32, max: u32 },

    // ========================================================================
    // COB Errors
    // ========================================================================
    /// COB not found.
    #[snafu(display("COB not found: {cob_type}:{cob_id}"))]
    CobNotFound { cob_type: String, cob_id: String },

    /// COB change validation failed.
    #[snafu(display("invalid COB change: {message}"))]
    InvalidCobChange { message: String },

    /// COB resolution failed (e.g., cycle detected).
    #[snafu(display("COB resolution failed: {message}"))]
    CobResolutionFailed { message: String },

    /// Too many changes to resolve.
    #[snafu(display("COB has too many changes to resolve: {count} > {max}"))]
    TooManyChanges { count: u32, max: u32 },

    // ========================================================================
    // Repository Errors
    // ========================================================================
    /// Repository not found.
    #[snafu(display("repository not found: {repo_id}"))]
    RepoNotFound { repo_id: String },

    /// Repository already exists.
    #[snafu(display("repository already exists: {repo_id}"))]
    RepoAlreadyExists { repo_id: String },

    /// Invalid repository identity.
    #[snafu(display("invalid repository identity: {message}"))]
    InvalidRepoIdentity { message: String },

    /// Too many delegates.
    #[snafu(display("too many delegates: {count} > {max}"))]
    TooManyDelegates { count: u32, max: u32 },

    /// Invalid threshold.
    #[snafu(display("invalid threshold {threshold}: must be 1..={delegates}"))]
    InvalidThreshold { threshold: u32, delegates: u32 },

    // ========================================================================
    // Ref Errors
    // ========================================================================
    /// Ref not found.
    #[snafu(display("ref not found: {ref_name}"))]
    RefNotFound { ref_name: String },

    /// Ref update conflict (CAS failure).
    #[snafu(display("ref update conflict: expected {expected:?}, found {found:?}"))]
    RefConflict {
        expected: Option<String>,
        found: Option<String>,
    },

    /// Invalid ref name.
    #[snafu(display("invalid ref name: {ref_name}"))]
    InvalidRefName { ref_name: String },

    /// Too many refs.
    #[snafu(display("too many refs in repository: {count} > {max}"))]
    TooManyRefs { count: u32, max: u32 },

    // ========================================================================
    // Authorization Errors
    // ========================================================================
    /// Not authorized to perform operation.
    #[snafu(display("not authorized: {signer} cannot {action}"))]
    NotAuthorized { signer: String, action: String },

    /// Invalid signature.
    #[snafu(display("invalid signature: {message}"))]
    InvalidSignature { message: String },

    /// Threshold not met.
    #[snafu(display("signature threshold not met: {count} < {threshold}"))]
    ThresholdNotMet { count: u32, threshold: u32 },

    // ========================================================================
    // Sync Errors
    // ========================================================================
    /// Fetch failed.
    #[snafu(display("failed to fetch object {hash}: {message}"))]
    FetchFailed { hash: String, message: String },

    /// Fetch timeout.
    #[snafu(display("fetch timeout for object {hash}"))]
    FetchTimeout { hash: String },

    /// No peers available to fetch from.
    #[snafu(display("no peers available for repository {repo_id}"))]
    NoPeersAvailable { repo_id: String },

    // ========================================================================
    // Gossip Errors
    // ========================================================================
    /// Generic gossip operation error.
    #[snafu(display("gossip error: {message}"))]
    GossipError { message: String },

    /// Rate limit exceeded for gossip messages.
    #[snafu(display("gossip rate limit exceeded: {reason}"))]
    GossipRateLimited { reason: String },

    /// Failed to subscribe to gossip topic.
    #[snafu(display("failed to subscribe to topic {topic}: {message}"))]
    GossipTopicError { topic: String, message: String },

    /// Maximum subscribed repositories exceeded.
    #[snafu(display("maximum subscribed repos exceeded: {count} > {max}"))]
    TooManySubscriptions { count: u32, max: u32 },

    /// Gossip service not initialized.
    #[snafu(display("gossip service not initialized"))]
    GossipNotInitialized,

    // ========================================================================
    // Storage Errors
    // ========================================================================
    /// Blob storage error.
    #[snafu(display("blob storage error: {message}"))]
    BlobStorage { message: String },

    /// KV storage error.
    #[snafu(display("KV storage error: {message}"))]
    KvStorage { message: String },

    /// Serialization error.
    #[snafu(display("serialization error: {message}"))]
    Serialization { message: String },

    // ========================================================================
    // Federation Errors
    // ========================================================================
    /// Federation operation error.
    #[snafu(display("federation error: {message}"))]
    FederationError { message: String },
}

impl From<anyhow::Error> for ForgeError {
    fn from(e: anyhow::Error) -> Self {
        ForgeError::BlobStorage {
            message: e.to_string(),
        }
    }
}

impl From<KeyValueStoreError> for ForgeError {
    fn from(e: KeyValueStoreError) -> Self {
        ForgeError::KvStorage {
            message: e.to_string(),
        }
    }
}

impl From<postcard::Error> for ForgeError {
    fn from(e: postcard::Error) -> Self {
        ForgeError::Serialization {
            message: e.to_string(),
        }
    }
}

/// Result type alias for Forge operations.
pub type ForgeResult<T> = Result<T, ForgeError>;
