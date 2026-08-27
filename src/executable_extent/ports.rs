//! Application-owned executable-extent ports.

/// Bounded capability-relative source failure.
#[derive(Debug)]
pub enum SourceError {
    /// A requested leaf is not one normal relative component.
    InvalidLeaf,
    /// A capability-relative open or read failed.
    Io(std::io::Error),
    /// Input exceeded its explicit byte bound.
    BoundExceeded,
}

/// Reads exact immutable members relative to an already authorized root.
pub trait BundleSource {
    /// Reads one normal member leaf with an inclusive byte bound.
    ///
    /// # Errors
    ///
    /// Returns a leaf, I/O, or bound failure.
    fn read_leaf(&self, leaf: &str, maximum_bytes: usize) -> Result<Vec<u8>, SourceError>;
}

/// Current admission observation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionPortError {
    /// Current authority or policy could not be observed conclusively.
    ObservationUnavailable,
}

/// Observes current Molten-owned admission facts.
pub trait CurrentAdmissionPort {
    /// Returns current artifact, runtime, resource, policy, and execution facts.
    ///
    /// # Errors
    ///
    /// Returns an unavailable observation instead of guessing.
    fn observe(
        &self,
        profile: &molten_core::executable_extent::ExtentCodeRootProfile,
    ) -> Result<molten_core::executable_extent::ActivationFacts, AdmissionPortError>;
}
