//! Application-owned failure vocabulary for external fabric capabilities.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the port vocabulary keeps explicit domain-qualified infrastructure failure names"
)]

use crate::error::Failure;
use crate::error::FailureClass;

/// Infrastructure failures that can cross a maintained fabric port boundary.
// r[impl molten.modularity.fabric_boundary.errors]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricPortError {
    CapabilityUnavailable { message: String },
    MalformedObservation { message: String },
    Timeout { message: String },
    Cancelled { message: String },
    StorageFailure { message: String },
    TransportFailure { message: String },
    UncertainExternalOutcome { message: String },
}

impl FabricPortError {
    pub fn capability(message: impl Into<String>) -> Self {
        Self::CapabilityUnavailable {
            message: message.into(),
        }
    }

    pub fn malformed(message: impl Into<String>) -> Self {
        Self::MalformedObservation {
            message: message.into(),
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::StorageFailure {
            message: message.into(),
        }
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self::TransportFailure {
            message: message.into(),
        }
    }

    pub fn uncertain(message: impl Into<String>) -> Self {
        Self::UncertainExternalOutcome {
            message: message.into(),
        }
    }

    pub fn class(&self) -> FailureClass {
        match self {
            Self::CapabilityUnavailable { .. } => FailureClass::CapabilityUnavailable,
            Self::MalformedObservation { .. } => FailureClass::MalformedObservation,
            Self::Timeout { .. } => FailureClass::Timeout,
            Self::Cancelled { .. } => FailureClass::Cancelled,
            Self::StorageFailure { .. } => FailureClass::StorageFailure,
            Self::TransportFailure { .. } => FailureClass::TransportFailure,
            Self::UncertainExternalOutcome { .. } => FailureClass::UncertainExternalOutcome,
        }
    }

    fn from_class(class: FailureClass, message: String) -> Self {
        match class {
            FailureClass::CapabilityUnavailable => Self::CapabilityUnavailable { message },
            FailureClass::MalformedObservation => Self::MalformedObservation { message },
            FailureClass::Timeout => Self::Timeout { message },
            FailureClass::Cancelled => Self::Cancelled { message },
            FailureClass::StorageFailure => Self::StorageFailure { message },
            FailureClass::TransportFailure => Self::TransportFailure { message },
            FailureClass::UncertainExternalOutcome => Self::UncertainExternalOutcome { message },
        }
    }

    pub fn contains(&self, pattern: &str) -> bool {
        self.message().contains(pattern)
    }

    pub fn message(&self) -> &str {
        match self {
            Self::CapabilityUnavailable { message }
            | Self::MalformedObservation { message }
            | Self::Timeout { message }
            | Self::Cancelled { message }
            | Self::StorageFailure { message }
            | Self::TransportFailure { message }
            | Self::UncertainExternalOutcome { message } => message,
        }
    }
}

impl std::fmt::Display for FabricPortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.class().as_str(), self.message())
    }
}

impl std::error::Error for FabricPortError {}

impl From<Failure> for FabricPortError {
    fn from(error: Failure) -> Self {
        match error {
            Failure::Classified { class, message } => Self::from_class(class, message),
            other => Self::capability(other.to_string()),
        }
    }
}

// r[impl molten.modularity.fabric_boundary.errors]
impl From<FabricPortError> for Failure {
    /// Keeps the port's class, including uncertainty, instead of flattening it into harness text.
    fn from(error: FabricPortError) -> Self {
        let class = error.class();
        match error {
            FabricPortError::CapabilityUnavailable { message }
            | FabricPortError::MalformedObservation { message }
            | FabricPortError::Timeout { message }
            | FabricPortError::Cancelled { message }
            | FabricPortError::StorageFailure { message }
            | FabricPortError::TransportFailure { message }
            | FabricPortError::UncertainExternalOutcome { message } => Failure::classified(class, message),
        }
    }
}

pub type FabricPortResult<T> = std::result::Result<T, FabricPortError>;

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.modularity.fabric_boundary.adapters.transport_error]
    #[test]
    fn typed_failures_retain_infrastructure_ownership() {
        let storage = FabricPortError::storage("commit failed");
        let transport = FabricPortError::transport("peer disconnected after submission");
        let uncertain = FabricPortError::uncertain("effect may have happened");

        assert!(matches!(storage, FabricPortError::StorageFailure { .. }));
        assert!(matches!(transport, FabricPortError::TransportFailure { .. }));
        assert!(matches!(uncertain, FabricPortError::UncertainExternalOutcome { .. }));
        assert!(transport.to_string().contains("transport failure"));
    }

    #[test]
    fn malformed_observation_does_not_become_policy_denial() {
        let error = FabricPortError::malformed("clock moved backwards");
        let molten = Failure::from(error.clone());

        assert!(matches!(error, FabricPortError::MalformedObservation { .. }));
        assert!(molten.to_string().contains("malformed observation"));
        assert!(!molten.to_string().contains("policy denied"));
    }

    // r[verify molten.modularity.fabric_boundary.errors]
    #[test]
    fn every_port_class_survives_conversion_both_ways() {
        for error in [
            FabricPortError::capability("capability"),
            FabricPortError::malformed("malformed"),
            FabricPortError::Timeout {
                message: "timeout".to_string(),
            },
            FabricPortError::Cancelled {
                message: "cancelled".to_string(),
            },
            FabricPortError::storage("storage"),
            FabricPortError::transport("transport"),
            FabricPortError::uncertain("uncertain"),
        ] {
            let failure = Failure::from(error.clone());
            assert_eq!(failure.class(), Some(error.class()));
            assert!(!matches!(failure, Failure::InvalidHarness(_)));
            assert_eq!(failure.to_string(), error.to_string());
            assert_eq!(FabricPortError::from(failure), error);
        }
    }

    #[test]
    fn unclassified_failures_stay_unclassified() {
        let failure = Failure::invalid_harness("uncertain external outcome: looks classified");
        assert_eq!(failure.class(), None);
        assert!(matches!(FabricPortError::from(failure), FabricPortError::CapabilityUnavailable { .. }));
    }
}
