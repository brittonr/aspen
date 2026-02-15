//! Fatal error types for wire protocol communication.
//!
//! Provides a simplified, serializable representation of unrecoverable
//! Raft errors that can be sent over the wire.

use serde::Deserialize;
use serde::Serialize;

/// Classification of fatal Raft errors for wire protocol.
///
/// This is a simplified representation of `openraft::error::Fatal` that can be
/// serialized and sent over the wire. It allows clients to understand why the
/// server cannot process requests and take appropriate action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftFatalErrorKind {
    /// RaftCore panicked - indicates a programming error.
    /// The node should be restarted, and clients should retry with other nodes.
    Panicked,
    /// RaftCore was explicitly stopped via shutdown.
    /// This is a normal shutdown, clients should retry with other nodes.
    Stopped,
    /// Unrecoverable storage error (log or state machine).
    /// Data may be corrupted, requires operator intervention.
    StorageError,
}

impl RaftFatalErrorKind {
    /// Convert from an openraft Fatal error to the wire protocol representation.
    pub fn from_fatal<C: openraft::RaftTypeConfig>(fatal: &openraft::error::Fatal<C>) -> Self {
        match fatal {
            openraft::error::Fatal::Panicked => Self::Panicked,
            openraft::error::Fatal::Stopped => Self::Stopped,
            openraft::error::Fatal::StorageError(_) => Self::StorageError,
        }
    }
}

impl std::fmt::Display for RaftFatalErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panicked => write!(f, "RaftCore panicked"),
            Self::Stopped => write!(f, "RaftCore stopped"),
            Self::StorageError => write!(f, "storage error"),
        }
    }
}
