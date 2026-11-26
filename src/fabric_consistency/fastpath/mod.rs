//! Shell over the no_std fast-path design model (`molten_fastpath_core`).
//!
//! The model is re-exported at its historical facade path. Each typed core rejection becomes a
//! classified `Failure` here, at the outer boundary, so callers branch on the class and keep the
//! typed reason in the message.

pub use molten_fastpath_core::Rejection;
pub use molten_fastpath_core::commute::*;
pub use molten_fastpath_core::envelope::*;
pub use molten_fastpath_core::hazard::*;
pub use molten_fastpath_core::quorate::*;
pub use molten_fastpath_core::succession::*;
pub use molten_fastpath_core::witness::*;

/// A rejected fast-path model input is a malformed observation: the core refused a transition,
/// retry, profile, contract, or scenario as given, and resubmitting the same input fails again.
// r[impl molten.consensus.no_std_raft_and_reliability_kernels]
pub fn failure(rejection: impl Into<Rejection>) -> crate::error::Failure {
    let rejection = rejection.into();
    crate::error::Failure::classified(
        crate::error::FailureClass::MalformedObservation,
        format!("fast-path model rejected input: {rejection}"),
    )
}
