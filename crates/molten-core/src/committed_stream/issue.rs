//! The issues a committed-change stream operation can refuse.

/// Why a committed-change stream operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommittedStreamIssue {
    /// The stream epoch is zero, so no cursor can identify a position in it.
    MissingStreamEpoch,
    /// The service identity is empty.
    MissingServiceId,
    /// The payload reference is not a canonical content reference.
    InvalidPayloadRef,
    /// The state reference is not a canonical content reference.
    InvalidStateRef,
    /// The event count is zero or above the bound.
    InvalidEventCount,
    /// The revision overflowed.
    RevisionOverflow,
    /// The ordinal overflowed.
    OrdinalOverflow,
    /// The stream retains more events than the bound allows.
    RetentionBoundExceeded,
    /// The buffer bound is zero.
    InvalidBufferBound,
}

impl CommittedStreamIssue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingStreamEpoch => "missing-stream-epoch",
            Self::MissingServiceId => "missing-service-id",
            Self::InvalidPayloadRef => "invalid-payload-ref",
            Self::InvalidStateRef => "invalid-state-ref",
            Self::InvalidEventCount => "invalid-event-count",
            Self::RevisionOverflow => "revision-overflow",
            Self::OrdinalOverflow => "ordinal-overflow",
            Self::RetentionBoundExceeded => "retention-bound-exceeded",
            Self::InvalidBufferBound => "invalid-buffer-bound",
        }
    }
}
