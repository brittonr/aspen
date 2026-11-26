//! Conversions from the typed errors of the `no_std` kernels back into [`super::Failure`].
//!
//! A kernel error names its shell class through [`molten_preserves_core::fault::Classified`] and
//! renders the shell message through `Display`; the conversion keeps both, so `Failure::class()`,
//! `Display` and equality are the same as before the code moved into a kernel. See
//! `molten_preserves_core::fault` for the convention.

use molten_preserves_core::fault::Classified;

impl super::Failure {
    /// The failure a classified kernel error maps back to: `InvalidHarness` or `Preserves`, with
    /// the kernel error's message.
    pub fn from_kernel<E: Classified>(error: E) -> Self {
        let message = error.to_string();
        match error.failure_kind() {
            molten_preserves_core::fault::FailureKind::InvalidHarness => Self::InvalidHarness(message),
            molten_preserves_core::fault::FailureKind::Preserves => Self::Preserves(message),
        }
    }
}

impl From<molten_preserves_core::view::ViewError> for super::Failure {
    fn from(error: molten_preserves_core::view::ViewError) -> Self {
        Self::from_kernel(error)
    }
}

impl From<molten_preserves_core::bounded::BoundError> for super::Failure {
    fn from(error: molten_preserves_core::bounded::BoundError) -> Self {
        Self::from_kernel(error)
    }
}

#[cfg(test)]
mod tests {
    type Failure = super::super::Failure;

    /// A codec denial, to cover the `Preserves` class.
    #[derive(Debug)]
    struct CodecDenial;

    impl core::fmt::Display for CodecDenial {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str("malformed packed Preserves input: truncated")
        }
    }

    impl core::error::Error for CodecDenial {}

    impl molten_preserves_core::fault::Classified for CodecDenial {
        fn failure_kind(&self) -> molten_preserves_core::fault::FailureKind {
            molten_preserves_core::fault::FailureKind::Preserves
        }
    }

    #[test]
    fn view_denials_become_invalid_harness_failures_with_the_same_message() {
        let error = molten_preserves_core::view::ViewError::ExpectedString { field: "name".into() };
        assert_eq!(Failure::from(error), Failure::InvalidHarness("expected string for name".into()));
    }

    #[test]
    fn bound_denials_become_invalid_harness_failures_with_the_same_message() {
        let error = molten_preserves_core::bounded::BoundError::CountExceeded {
            label: "pins".into(),
            count: 3,
            maximum: 2,
        };
        let failure = Failure::from(error);
        assert_eq!(failure, Failure::invalid_harness("pins count 3 exceeds maximum 2"));
        assert_eq!(failure.to_string(), "invalid harness artifact: pins count 3 exceeds maximum 2");
        assert_eq!(failure.class(), None);
    }

    #[test]
    fn codec_class_denials_stay_codec_failures() {
        let failure = Failure::from_kernel(CodecDenial);
        assert_eq!(failure, Failure::Preserves("malformed packed Preserves input: truncated".into()));
        assert_ne!(failure, Failure::invalid_harness(CodecDenial.to_string()));
    }
}
