//! Application-owned failure vocabulary for external fabric capabilities.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the port vocabulary keeps explicit domain-qualified infrastructure failure names"
)]

use crate::error::MoltenError;

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
        let class = match self {
            Self::CapabilityUnavailable { .. } => "capability unavailable",
            Self::MalformedObservation { .. } => "malformed observation",
            Self::Timeout { .. } => "timeout",
            Self::Cancelled { .. } => "cancelled",
            Self::StorageFailure { .. } => "storage failure",
            Self::TransportFailure { .. } => "transport failure",
            Self::UncertainExternalOutcome { .. } => "uncertain external outcome",
        };
        write!(formatter, "{class}: {}", self.message())
    }
}

impl std::error::Error for FabricPortError {}

impl From<MoltenError> for FabricPortError {
    fn from(error: MoltenError) -> Self {
        Self::capability(error.to_string())
    }
}

impl From<FabricPortError> for MoltenError {
    fn from(error: FabricPortError) -> Self {
        Self::invalid_harness(error.to_string())
    }
}

pub type FabricPortResult<T> = std::result::Result<T, FabricPortError>;

#[cfg(test)]
mod tests {
    use super::*;

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
        let molten = MoltenError::from(error.clone());

        assert!(matches!(error, FabricPortError::MalformedObservation { .. }));
        assert!(molten.to_string().contains("malformed observation"));
        assert!(!molten.to_string().contains("policy denied"));
    }
}
