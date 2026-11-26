//! Count-bound denials, rendered exactly as the `molten` shell rendered them.

/// Why a bounded collection or count conversion was denied. Every variant is a
/// [`crate::fault::FailureKind::InvalidHarness`] denial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundError {
    /// `{label} count {count} exceeds maximum {maximum}`.
    CountExceeded {
        label: alloc::string::String,
        count: u64,
        maximum: u64,
    },
    /// `{label} count overflow`: the count itself does not fit.
    CountOverflow { label: alloc::string::String },
    /// `{label} {value} does not fit usize`.
    UsizeOverflow { label: alloc::string::String, value: u64 },
    /// `{label} {value} does not fit u64`; `value` is the host count in decimal.
    U64Overflow {
        label: alloc::string::String,
        value: alloc::string::String,
    },
}

impl core::fmt::Display for BoundError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CountExceeded { label, count, maximum } => {
                write!(formatter, "{label} count {count} exceeds maximum {maximum}")
            }
            Self::CountOverflow { label } => write!(formatter, "{label} count overflow"),
            Self::UsizeOverflow { label, value } => write!(formatter, "{label} {value} does not fit usize"),
            Self::U64Overflow { label, value } => write!(formatter, "{label} {value} does not fit u64"),
        }
    }
}

impl core::error::Error for BoundError {}

impl crate::fault::Classified for BoundError {
    fn failure_kind(&self) -> crate::fault::FailureKind {
        crate::fault::FailureKind::InvalidHarness
    }
}
