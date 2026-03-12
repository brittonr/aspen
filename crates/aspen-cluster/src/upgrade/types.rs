//! Types and error definitions for node upgrades.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use snafu::Snafu;
use tokio::sync::Notify;

/// Errors that can occur during node upgrade.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum NodeUpgradeError {
    /// Nix store path is not available locally and could not be fetched.
    #[snafu(display("store path unavailable: {path}"))]
    StorePathUnavailable { path: String },

    /// Binary validation failed (--version returned non-zero).
    #[snafu(display("binary validation failed: {reason}"))]
    BinaryValidationFailed { reason: String },

    /// Artifact download failed.
    #[snafu(display("artifact fetch failed: {reason}"))]
    ArtifactFetchFailed { reason: String },

    /// Nix profile switch failed.
    #[snafu(display("nix profile switch failed: {reason}"))]
    NixProfileSwitchFailed { reason: String },

    /// Binary replacement failed (rename/copy).
    #[snafu(display("binary replacement failed: {reason}"))]
    BinaryReplacementFailed { reason: String },

    /// Process restart failed.
    #[snafu(display("restart failed: {reason}"))]
    RestartFailed { reason: String },

    /// Status reporting to KV failed.
    #[snafu(display("status report failed: {reason}"))]
    StatusReportFailed { reason: String },

    /// Drain timeout exceeded with cancelled operations.
    #[snafu(display("drain timeout: {cancelled_ops} operations cancelled"))]
    DrainTimeout { cancelled_ops: u64 },

    /// Nix rollback failed.
    #[snafu(display("nix rollback failed: {reason}"))]
    NixRollbackFailed { reason: String },

    /// Binary rollback failed (no .bak file).
    #[snafu(display("binary rollback failed: {reason}"))]
    BinaryRollbackFailed { reason: String },
}

pub type Result<T> = std::result::Result<T, NodeUpgradeError>;

/// How the binary gets replaced.
#[derive(Debug, Clone)]
pub enum UpgradeMethod {
    /// Nix profile switch: `nix-env --profile <path> --set <store-path>`.
    Nix {
        /// Profile path (e.g., `/nix/var/nix/profiles/aspen-node`).
        profile_path: PathBuf,
    },
    /// Direct binary replacement via blob download.
    Blob {
        /// Path to the current running binary.
        binary_path: PathBuf,
        /// Staging directory for downloads.
        staging_dir: PathBuf,
    },
}

/// How the process gets restarted after binary replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartMethod {
    /// Restart via systemd: `systemctl restart <unit>`.
    Systemd {
        /// Systemd unit name (e.g., `aspen-node`).
        unit_name: String,
    },
    /// Restart via execve: replace current process with new binary.
    Execve,
}

/// Configuration for the node upgrade executor.
#[derive(Debug, Clone)]
pub struct NodeUpgradeConfig {
    /// Node ID for status reporting.
    pub node_id: u64,
    /// How to replace the binary.
    pub upgrade_method: UpgradeMethod,
    /// How to restart after replacement.
    pub restart_method: RestartMethod,
    /// Drain timeout in seconds (default: DRAIN_TIMEOUT_SECS).
    pub drain_timeout_secs: u64,
    /// Binary to validate inside a Nix store path.
    ///
    /// - `Some("bin/aspen-node")` (default): checks that `bin/aspen-node` exists in the store path
    /// - `Some("bin/cowsay")`: checks for a custom binary path
    /// - `None`: skip binary existence check, only verify the store path is available
    pub expected_binary: Option<String>,
}

/// Shared drain state for coordinating graceful shutdown of client RPCs.
///
/// The drain flag is checked by RPC handlers to reject new requests.
/// The in-flight counter tracks operations that started before drain.
pub struct DrainState {
    /// When set, new client RPCs should be rejected with NOT_LEADER.
    pub(super) is_draining: std::sync::atomic::AtomicBool,
    /// Count of in-flight operations (started before drain).
    pub(super) in_flight_ops: AtomicU64,
    /// Notified when in_flight_ops reaches 0.
    pub(super) drained: Notify,
}

impl DrainState {
    /// Create a new drain state (not draining, zero in-flight).
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            is_draining: std::sync::atomic::AtomicBool::new(false),
            in_flight_ops: AtomicU64::new(0),
            drained: Notify::new(),
        })
    }

    /// Check if the node is currently draining.
    #[inline]
    pub fn is_draining(&self) -> bool {
        self.is_draining.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Register a new in-flight operation. Returns false if draining.
    ///
    /// Callers should check the return value: if false, reject the request.
    #[inline]
    pub fn try_start_op(&self) -> bool {
        if self.is_draining() {
            return false;
        }
        self.in_flight_ops.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        // Double-check after incrementing (prevents race with drain start).
        if self.is_draining() {
            self.finish_op();
            return false;
        }
        true
    }

    /// Mark an in-flight operation as complete.
    #[inline]
    pub fn finish_op(&self) {
        let prev = self.in_flight_ops.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        if prev == 1 && self.is_draining() {
            self.drained.notify_waiters();
        }
    }

    /// Get the current in-flight operation count.
    #[inline]
    pub fn in_flight_count(&self) -> u64 {
        self.in_flight_ops.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Default for DrainState {
    fn default() -> Self {
        Self {
            is_draining: std::sync::atomic::AtomicBool::new(false),
            in_flight_ops: AtomicU64::new(0),
            drained: Notify::new(),
        }
    }
}
