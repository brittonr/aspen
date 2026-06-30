#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeErrorCategory {
    InvalidInput,
    DeniedOperation,
    AdapterUnavailable,
    PersistenceFailure,
}

#[derive(Debug, snafu::Snafu)]
pub enum RuntimeBoundaryError {
    #[snafu(display("invalid runtime input at {boundary}: {message}"))]
    InvalidInput { boundary: &'static str, message: String },

    #[snafu(display("runtime operation denied at {boundary}: {message}"))]
    DeniedOperation { boundary: &'static str, message: String },

    #[snafu(display("runtime adapter unavailable at {boundary}: {message}"))]
    AdapterUnavailable { boundary: &'static str, message: String },

    #[snafu(display("runtime persistence failure at {boundary}: {message}"))]
    PersistenceFailure { boundary: &'static str, message: String },
}

impl RuntimeBoundaryError {
    pub fn invalid_input(boundary: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            boundary,
            message: message.into(),
        }
    }

    pub fn denied_operation(boundary: &'static str, message: impl Into<String>) -> Self {
        Self::DeniedOperation {
            boundary,
            message: message.into(),
        }
    }

    pub fn adapter_unavailable(boundary: &'static str, message: impl Into<String>) -> Self {
        Self::AdapterUnavailable {
            boundary,
            message: message.into(),
        }
    }

    pub fn persistence_failure(boundary: &'static str, message: impl Into<String>) -> Self {
        Self::PersistenceFailure {
            boundary,
            message: message.into(),
        }
    }

    pub fn category(&self) -> RuntimeErrorCategory {
        match self {
            Self::InvalidInput { .. } => RuntimeErrorCategory::InvalidInput,
            Self::DeniedOperation { .. } => RuntimeErrorCategory::DeniedOperation,
            Self::AdapterUnavailable { .. } => RuntimeErrorCategory::AdapterUnavailable,
            Self::PersistenceFailure { .. } => RuntimeErrorCategory::PersistenceFailure,
        }
    }

    pub fn boundary(&self) -> &'static str {
        match self {
            Self::InvalidInput { boundary, .. }
            | Self::DeniedOperation { boundary, .. }
            | Self::AdapterUnavailable { boundary, .. }
            | Self::PersistenceFailure { boundary, .. } => boundary,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_boundary_errors_are_structured_by_category() {
        let invalid = super::RuntimeBoundaryError::invalid_input("envelope", "bad subject");
        let denied = super::RuntimeBoundaryError::denied_operation("policy", "missing send capability");
        let adapter = super::RuntimeBoundaryError::adapter_unavailable("iroh-gossip", "not connected");
        let persistence = super::RuntimeBoundaryError::persistence_failure("redb", "write failed");

        assert_eq!(invalid.category(), super::RuntimeErrorCategory::InvalidInput);
        assert_eq!(denied.category(), super::RuntimeErrorCategory::DeniedOperation);
        assert_eq!(adapter.category(), super::RuntimeErrorCategory::AdapterUnavailable);
        assert_eq!(persistence.category(), super::RuntimeErrorCategory::PersistenceFailure);
        assert_eq!(invalid.boundary(), "envelope");
        assert!(denied.to_string().contains("runtime operation denied"));
    }
}
