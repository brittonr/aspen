pub type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessDivergence {
    pub kind: String,
    pub step: Option<u64>,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

impl HarnessDivergence {
    pub fn new(
        kind: impl Into<String>,
        step: Option<u64>,
        expected: impl Into<String>,
        actual: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            step,
            expected: expected.into(),
            actual: actual.into(),
            detail: detail.into(),
        }
    }
}

/// Infrastructure failure classes that survive conversion from a port boundary, so shells can
/// branch on the class instead of matching message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    CapabilityUnavailable,
    MalformedObservation,
    Timeout,
    Cancelled,
    StorageFailure,
    TransportFailure,
    /// The operation may or may not have taken effect; retrying it can duplicate the effect.
    UncertainExternalOutcome,
}

impl FailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityUnavailable => "capability unavailable",
            Self::MalformedObservation => "malformed observation",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::StorageFailure => "storage failure",
            Self::TransportFailure => "transport failure",
            Self::UncertainExternalOutcome => "uncertain external outcome",
        }
    }

    /// Stable machine-readable identifier for receipts and reports.
    pub const fn code(self) -> &'static str {
        match self {
            Self::CapabilityUnavailable => "capability-unavailable",
            Self::MalformedObservation => "malformed-observation",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::StorageFailure => "storage-failure",
            Self::TransportFailure => "transport-failure",
            Self::UncertainExternalOutcome => "uncertain-external-outcome",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Io(String),
    Preserves(String),
    InvalidHarness(String),
    HarnessDivergence(HarnessDivergence),
    /// A classified infrastructure failure, for example one converted from a fabric port error.
    Classified {
        class: FailureClass,
        message: String,
    },
}

impl Failure {
    pub fn invalid_harness(message: impl Into<String>) -> Self {
        Self::InvalidHarness(message.into())
    }

    pub fn harness_divergence(divergence: HarnessDivergence) -> Self {
        Self::HarnessDivergence(divergence)
    }

    pub fn classified(class: FailureClass, message: impl Into<String>) -> Self {
        Self::Classified {
            class,
            message: message.into(),
        }
    }

    /// The infrastructure class, when this failure carries one.
    pub fn class(&self) -> Option<FailureClass> {
        match self {
            Self::Classified { class, .. } => Some(*class),
            Self::Io(_) | Self::Preserves(_) | Self::InvalidHarness(_) | Self::HarnessDivergence(_) => None,
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Io(message) => write!(f, "io error: {message}"),
            Failure::Preserves(message) => write!(f, "preserves error: {message}"),
            Failure::InvalidHarness(message) => write!(f, "invalid harness artifact: {message}"),
            Failure::Classified { class, message } => write!(f, "{}: {message}", class.as_str()),
            Failure::HarnessDivergence(divergence) => {
                if let Some(step) = divergence.step {
                    write!(
                        f,
                        "divergence kind={} step={} expected={} actual={} detail={}",
                        divergence.kind, step, divergence.expected, divergence.actual, divergence.detail
                    )
                } else {
                    write!(
                        f,
                        "divergence kind={} expected={} actual={} detail={}",
                        divergence.kind, divergence.expected, divergence.actual, divergence.detail
                    )
                }
            }
        }
    }
}

impl std::error::Error for Failure {}

impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classified_failures_keep_their_class_and_name_it() {
        let uncertain = Failure::classified(FailureClass::UncertainExternalOutcome, "effect may have happened");
        assert_eq!(uncertain.class(), Some(FailureClass::UncertainExternalOutcome));
        assert_eq!(uncertain.to_string(), "uncertain external outcome: effect may have happened");
        assert!(!uncertain.to_string().contains("invalid harness"));
        assert_eq!(Failure::invalid_harness("effect may have happened").class(), None);
    }
}
