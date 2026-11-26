//! Typed reasons a replica step, start, or peer envelope is refused.
//!
//! Every refusal is deterministic: it depends only on the inputs, and the step that produced it
//! planned no effects. [`ReplicaError::class`] names the failure class the `molten` shell keeps
//! when it converts the error at its boundary.

pub mod refs;

pub type Result<T, E = ReplicaError> = core::result::Result<T, E>;

/// The failure class a [`ReplicaError`] keeps at the shell boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaErrorClass {
    /// The state, event, start input, or peer message breaks an admitted invariant.
    Invalid,
    /// Committed recovery evidence reported by peers contradicts itself or a snapshot.
    MalformedObservation,
    /// The canonical identity port could not encode an identity record.
    Encoding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaError {
    /// The input breaks a protocol, bound, or binding invariant.
    Denied(&'static str),
    /// A term, index, or sequence would leave its integer domain.
    Overflow(&'static str),
    /// A field that must hold a canonical content ref does not; `label` names the field.
    ContentRef {
        label: Option<&'static str>,
        issue: refs::ContentRefIssue,
    },
    /// An identifier is empty, too long, or uses unsupported characters.
    Identifier {
        label: &'static str,
        issue: refs::IdentifierIssue,
    },
    /// The static membership does not hold exactly the required voter count.
    VoterCount { required: usize },
    /// Committed recovery evidence contradicts the snapshot recovery would install.
    RecoveryEvidence {
        snapshot_ref: String,
        index: u64,
        term: u64,
    },
    /// The identity port failed to derive a canonical ref.
    Identity(IdentityError),
}

impl ReplicaError {
    pub const fn class(&self) -> ReplicaErrorClass {
        match self {
            Self::Denied(_)
            | Self::Overflow(_)
            | Self::ContentRef { .. }
            | Self::Identifier { .. }
            | Self::VoterCount { .. } => ReplicaErrorClass::Invalid,
            Self::RecoveryEvidence { .. } => ReplicaErrorClass::MalformedObservation,
            Self::Identity(_) => ReplicaErrorClass::Encoding,
        }
    }
}

impl core::fmt::Display for ReplicaError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Denied(message) | Self::Overflow(message) => formatter.write_str(message),
            Self::ContentRef { label: None, issue } => write!(formatter, "{issue}"),
            Self::ContentRef {
                label: Some(label),
                issue,
            } => write!(formatter, "invalid {label}: {issue}"),
            Self::Identifier { label, issue } => write!(formatter, "{label} {issue}"),
            Self::VoterCount { required } => {
                write!(formatter, "initial live Raft profile requires exactly {required} voters")
            }
            Self::RecoveryEvidence {
                snapshot_ref,
                index,
                term,
            } => write!(
                formatter,
                "Raft recovery evidence diverges from snapshot {snapshot_ref} at index {index} (term {term})"
            ),
            Self::Identity(error) => write!(formatter, "{error}"),
        }
    }
}

impl core::error::Error for ReplicaError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ContentRef { issue, .. } => Some(issue),
            Self::Identifier { issue, .. } => Some(issue),
            Self::Identity(error) => Some(error),
            Self::Denied(_) | Self::Overflow(_) | Self::VoterCount { .. } | Self::RecoveryEvidence { .. } => None,
        }
    }
}

impl From<IdentityError> for ReplicaError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

/// The identity port could not encode a canonical identity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityError {
    detail: String,
}

impl IdentityError {
    pub fn new(detail: impl Into<String>) -> Self {
        Self { detail: detail.into() }
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl core::fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl core::error::Error for IdentityError {}
