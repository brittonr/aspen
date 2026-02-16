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
    /// Blob storage error during add.
    #[snafu(display("failed to add blob: {message}"))]
    AddBlob { message: String },

    /// Blob storage error during get.
    #[snafu(display("failed to get blob {hash}: {message}"))]
    GetBlob { hash: String, message: String },

    /// Blob storage error during existence check.
    #[snafu(display("failed to check blob existence for {hash}: {message}"))]
    CheckBlobExists { hash: String, message: String },

    /// Blob storage error during protect.
    #[snafu(display("failed to protect blob {tag}: {message}"))]
    ProtectBlob { tag: String, message: String },

    /// Blob storage error during unprotect.
    #[snafu(display("failed to unprotect blob {tag}: {message}"))]
    UnprotectBlob { tag: String, message: String },

    /// Blob storage error.
    #[snafu(display("blob storage error: {message}"))]
    BlobStorage { message: String },

    /// KV storage error.
    #[snafu(display("KV storage error: {message}"))]
    KvStorage { message: String },

    /// System time error (clock before Unix epoch).
    #[snafu(display("system time error: {message}"))]
    SystemTime { message: String },

    /// Serialization error.
    #[snafu(display("serialization error: {message}"))]
    Serialization { message: String },

    /// Deserialization error.
    #[snafu(display("deserialization error: {message}"))]
    Deserialization { message: String },

    /// IO error.
    #[snafu(display("IO error: {message}"))]
    Io { message: String },

    /// Failed to create directory.
    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to read file.
    #[snafu(display("failed to read file {}: {source}", path.display()))]
    ReadFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to write file.
    #[snafu(display("failed to write file {}: {source}", path.display()))]
    WriteFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to read directory entries.
    #[snafu(display("failed to read directory {}: {source}", path.display()))]
    ReadDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to read directory entry.
    #[snafu(display("failed to read directory entry in {}: {source}", path.display()))]
    ReadDirEntry {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to stat file.
    #[snafu(display("failed to stat {}: {source}", path.display()))]
    StatFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to read file to string.
    #[snafu(display("failed to read file to string {}: {source}", path.display()))]
    ReadFileToString {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to remove directory.
    #[snafu(display("failed to remove directory {}: {source}", path.display()))]
    RemoveDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to begin pristine transaction.
    #[snafu(display("failed to begin {txn_kind} transaction for {repo_id}: {message}"))]
    BeginTransaction {
        repo_id: String,
        txn_kind: String,
        message: String,
    },

    /// Failed to open or create pristine database.
    #[snafu(display("failed to open pristine at {}: {message}", path.display()))]
    OpenPristine { path: std::path::PathBuf, message: String },

    /// Failed to load channel from pristine.
    #[snafu(display("failed to load channel '{channel}': {message}"))]
    LoadChannel { channel: String, message: String },

    /// Failed to list channels.
    #[snafu(display("failed to list channels: {message}"))]
    ListChannels { message: String },

    /// Failed to open or create channel.
    #[snafu(display("failed to open/create channel '{channel}': {message}"))]
    OpenOrCreateChannel { channel: String, message: String },

    /// Failed to fork channel.
    #[snafu(display("failed to fork channel '{source_channel}' to '{dest_channel}': {message}"))]
    ForkChannel {
        source_channel: String,
        dest_channel: String,
        message: String,
    },

    /// Failed to rename channel.
    #[snafu(display("failed to rename channel '{old_name}' to '{new_name}': {message}"))]
    RenameChannel {
        old_name: String,
        new_name: String,
        message: String,
    },

    /// Failed to drop channel.
    #[snafu(display("failed to drop channel '{channel}': {message}"))]
    DropChannel { channel: String, message: String },

    /// Failed to commit pristine transaction.
    #[snafu(display("failed to commit transaction: {message}"))]
    CommitTransaction { message: String },

    /// Failed to read channel log.
    #[snafu(display("failed to read channel log for '{channel}': {message}"))]
    ReadChannelLog { channel: String, message: String },

    /// Failed to deserialize change file.
    #[snafu(display("failed to deserialize change at {}: {message}", path.display()))]
    DeserializeChange { path: std::path::PathBuf, message: String },

    /// Failed to compute change hash.
    #[snafu(display("failed to compute pijul hash: {message}"))]
    ComputeChangeHash { message: String },

    /// Failed to save change in libpijul format.
    #[snafu(display("failed to save change in libpijul format: {message}"))]
    SaveLibpijulChange { message: String },

    /// Failed to parse config TOML.
    #[snafu(display("failed to parse config at {}: {message}", path.display()))]
    ParseConfig { path: std::path::PathBuf, message: String },

    /// Failed to serialize config TOML.
    #[snafu(display("failed to serialize config: {message}"))]
    SerializeConfig { message: String },

    /// Failed to output repository to working directory.
    #[snafu(display("failed to output channel '{channel}' to working directory: {message}"))]
    OutputChannel { channel: String, message: String },

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

    // ========================================================================
    // Resolution Errors
    // ========================================================================
    /// Conflict not found for resolution.
    #[snafu(display("conflict not found at path: {path}"))]
    ConflictNotFound {
        /// Path where conflict was expected.
        path: String,
    },

    /// Resolution failed.
    #[snafu(display("resolution failed: {message}"))]
    ResolutionFailed {
        /// Reason for failure.
        message: String,
    },

    /// Invalid resolution strategy for conflict kind.
    #[snafu(display("strategy '{strategy}' not applicable to '{conflict_kind}' conflict"))]
    InvalidResolutionStrategy {
        /// The strategy that was attempted.
        strategy: String,
        /// The kind of conflict.
        conflict_kind: String,
    },

    /// Resolution state is stale (channel has changed).
    #[snafu(display("resolution state is stale: expected head {expected}, found {actual}"))]
    StaleResolutionState {
        /// Expected channel head.
        expected: String,
        /// Actual channel head.
        actual: String,
    },
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
