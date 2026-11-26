//! What a step plans: typed effects for the shell to execute, in order, and the next state.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalDisposition {
    Committed,
    Retryable,
    Denied,
    Cancelled,
    Uncertain,
}

impl ProposalDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Retryable => "retryable",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDisposition {
    Current,
    Local,
    Retryable,
    Denied,
}

impl ReadDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Local => "local",
            Self::Retryable => "retryable",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaEffect {
    PersistHardState {
        term: u64,
        voted_for: Option<String>,
        /// Persisted atomically with term and vote; cleared only by peer admission.
        is_recovery_required: bool,
    },
    PersistEntries {
        truncate_from: Option<u64>,
        entries: Vec<ReplicatedEntry>,
    },
    FlushLog {
        through_index: u64,
    },
    PersistCommit {
        through_index: u64,
    },
    PersistSnapshot {
        snapshot: ReplicaSnapshot,
        is_suffix_retained: bool,
    },
    Send {
        envelope: ReplicaMessageEnvelope,
    },
    ArmElectionTimer {
        timer_ref: String,
    },
    ArmHeartbeatTimer,
    RestoreApplicationSnapshot {
        snapshot: ReplicaSnapshot,
    },
    ApplyCommitted {
        entries: Vec<ReplicatedEntry>,
    },
    ProposalOutcome {
        request_ref: String,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    },
    ReadOutcome {
        request_ref: String,
        mode: ReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    },
    LifecycleChanged {
        lifecycle: ReplicaLifecycle,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaTransition {
    pub next: ReplicaState,
    pub effects: Vec<ReplicaEffect>,
}
