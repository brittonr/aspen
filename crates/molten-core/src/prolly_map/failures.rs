//! Phase-aware classification of prolly map port failures.
//!
//! A port failure is not one thing. A failure that precedes any mutation has a
//! known outcome: the operation did not apply. A commit failure may already
//! have become durable, so its outcome is unknown until a readback reconciles
//! it. The classifier is pure: it reads typed phases and typed failure kinds
//! and returns the outcome class. The adapter owns the I/O facts and calls this
//! function instead of hand-assigning a boolean.

// r[impl molten.prolly.publication.classification]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProllyPortPhase {
    /// Admission and validation, before any store access mutates state.
    Validation,
    /// Staging content blocks into the store.
    Staging,
    /// Advancing the published root of a map.
    Publication,
    /// Reading the published root back for reconciliation.
    Readback,
    /// Deleting unreachable blocks.
    Deletion,
}

/// The kind of failure a port observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProllyPortFailureKind {
    /// A read of the store failed.
    Read,
    /// Encoding or decoding a value failed.
    Codec,
    /// The request did not satisfy the admitted contract.
    Validation,
    /// A write inside an uncommitted transaction failed.
    Write,
    /// The write transaction commit failed.
    Commit,
}

/// The outcome class of a port failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProllyPortFailureOutcome {
    /// The request was rejected and no state changed.
    DefiniteRejection,
    /// The operation failed before any mutation, so it did not apply.
    NotApplied,
    /// The operation may already have become durable, so the outcome is unknown.
    Unknown,
}

impl ProllyPortFailureOutcome {
    /// Whether the outcome of the operation is unknown.
    pub const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Classify a port failure over its phase and kind.
///
/// A commit failure in the staging, publication, or deletion phase is unknown:
/// the transaction may already have become durable. Every other failure
/// precedes any mutation, so the outcome is known.
pub fn classify_port_failure(phase: ProllyPortPhase, kind: ProllyPortFailureKind) -> ProllyPortFailureOutcome {
    match kind {
        ProllyPortFailureKind::Validation => ProllyPortFailureOutcome::DefiniteRejection,
        ProllyPortFailureKind::Read | ProllyPortFailureKind::Codec | ProllyPortFailureKind::Write => {
            ProllyPortFailureOutcome::NotApplied
        }
        ProllyPortFailureKind::Commit => match phase {
            ProllyPortPhase::Staging | ProllyPortPhase::Publication | ProllyPortPhase::Deletion => {
                ProllyPortFailureOutcome::Unknown
            }
            ProllyPortPhase::Validation | ProllyPortPhase::Readback => ProllyPortFailureOutcome::NotApplied,
        },
    }
}
