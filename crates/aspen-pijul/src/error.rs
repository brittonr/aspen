//! Error types for the Pijul module.
//!
//! Uses `snafu` for structured error handling with context.

use aspen_core::KeyValueStoreError;
use snafu::Snafu;

/// Errors that can occur in Pijul operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum PijulError {
    // ========================================================================
    // Change Errors
    // ========================================================================
    /// Change not found in storage.
    #[snafu(display("change not found: {hash}"))]
    ChangeNotFound { hash: String },

    /// Change size exceeds maximum allowed.
    #[snafu(display("change size {size} exceeds maximum {max}"))]
    ChangeTooLarge { size: u64, max: u64 },

    /// Invalid change format.
    #[snafu(display("invalid change: {message}"))]
    InvalidChange { message: String },

    /// Too many hunks in a change.
    #[snafu(display("change has {count} hunks, maximum is {max}"))]
    TooManyHunks { count: u32, max: u32 },

    /// Too many dependencies.
    #[snafu(display("change has {count} dependencies, maximum is {max}"))]
    TooManyDependencies { count: u32, max: u32 },

    /// Change application failed.
    #[snafu(display("failed to apply change: {message}"))]
    ApplyFailed { message: String },

    /// Change unrecord failed.
    #[snafu(display("failed to unrecord change: {message}"))]
    UnrecordFailed { message: String },

    /// Change recording failed.
    #[snafu(display("failed to record change: {message}"))]
    RecordFailed { message: String },

    /// Working directory output failed.
    #[snafu(display("failed to output working directory: {message}"))]
    OutputFailed { message: String },

    // ========================================================================
    // Channel Errors
    // ========================================================================
    /// Channel not found.
    #[snafu(display("channel not found: {channel}"))]
    ChannelNotFound { channel: String },

    /// Channel already exists.
    #[snafu(display("channel already exists: {channel}"))]
    ChannelAlreadyExists { channel: String },

    /// Invalid channel name.
    #[snafu(display("invalid channel name: {channel}"))]
    InvalidChannelName { channel: String },

    /// Too many channels.
    #[snafu(display("too many channels: {count} > {max}"))]
    TooManyChannels { count: u32, max: u32 },

    /// Channel update conflict (CAS failure).
    #[snafu(display("channel update conflict: expected {expected:?}, found {found:?}"))]
    ChannelConflict {
        expected: Option<String>,
        found: Option<String>,
    },

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

    // ========================================================================
    // Pristine Errors
    // ========================================================================
    /// Pristine storage error.
    #[snafu(display("pristine storage error: {message}"))]
    PristineStorage { message: String },

    /// Pristine not initialized.
    #[snafu(display("pristine not initialized for repository: {repo_id}"))]
    PristineNotInitialized { repo_id: String },

    /// Pristine size limit exceeded.
    #[snafu(display("pristine size {size} exceeds maximum {max}"))]
    PristineTooLarge { size: u64, max: u64 },

    // ========================================================================
    // Sync Errors
    // ========================================================================
    /// Fetch failed.
    #[snafu(display("failed to fetch change {hash}: {message}"))]
    FetchFailed { hash: String, message: String },

    /// Fetch timeout.
    #[snafu(display("fetch timeout for change {hash}"))]
    FetchTimeout { hash: String },

    /// No peers available to fetch from.
    #[snafu(display("no peers available for repository {repo_id}"))]
    NoPeersAvailable { repo_id: String },

    /// Sync operation failed.
    #[snafu(display("sync failed: {message}"))]
    SyncFailed { message: String },

    /// Too many pending sync requests.
    ///
    /// Tiger Style: Prevents memory exhaustion from unbounded pending maps.
    #[snafu(display("too many pending changes: {count} > {max}"))]
    TooManyPendingChanges {
        /// Current count of pending changes.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// Too many channel updates waiting for a single change.
    ///
    /// Tiger Style: Prevents memory exhaustion from unbounded awaiting list.
    #[snafu(display("too many updates waiting for change {hash}: {count} > {max}"))]
    TooManyAwaitingUpdates {
        /// Change hash being waited on.
        hash: String,
        /// Current count of waiting updates.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

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

    /// Deserialization error.
    #[snafu(display("deserialization error: {message}"))]
    Deserialization { message: String },

    /// IO error.
    #[snafu(display("IO error: {message}"))]
    Io { message: String },

    // ========================================================================
    // Working Directory Errors
    // ========================================================================
    /// Working directory already initialized.
    #[snafu(display("working directory already initialized: {path}"))]
    WorkingDirAlreadyInitialized {
        /// Path to the working directory.
        path: String,
    },

    /// Working directory not initialized.
    #[snafu(display("working directory not initialized: {path}"))]
    WorkingDirNotInitialized {
        /// Path to the working directory.
        path: String,
    },

    /// Path is outside the working directory.
    #[snafu(display("path is outside working directory: {path}"))]
    WorkingDirPathOutside {
        /// Path that was outside.
        path: String,
    },

    /// Too many staged files.
    #[snafu(display("too many staged files: {count} > {max}"))]
    TooManyStagedFiles {
        /// Current count of staged files.
        count: u32,
        /// Maximum allowed.
        max: u32,
    },

    // ========================================================================
    // libpijul Errors
    // ========================================================================
    /// Error from libpijul.
    #[snafu(display("libpijul error: {message}"))]
    LibPijul { message: String },
}

impl From<anyhow::Error> for PijulError {
    fn from(e: anyhow::Error) -> Self {
        PijulError::BlobStorage { message: e.to_string() }
    }
}

impl From<KeyValueStoreError> for PijulError {
    fn from(e: KeyValueStoreError) -> Self {
        PijulError::KvStorage { message: e.to_string() }
    }
}

impl From<postcard::Error> for PijulError {
    fn from(e: postcard::Error) -> Self {
        PijulError::Serialization { message: e.to_string() }
    }
}

impl From<std::io::Error> for PijulError {
    fn from(e: std::io::Error) -> Self {
        PijulError::Io { message: e.to_string() }
    }
}

/// Result type alias for Pijul operations.
pub type PijulResult<T> = Result<T, PijulError>;
