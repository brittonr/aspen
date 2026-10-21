//! Pure ingress publication reconciliation before adapter effects.
//!
//! Every observation belongs to one exact delivery operation and control request.
//! Decisions neither reconstruct historical receipts nor authorize dispatch.

mod model;
mod transition;
mod validation;

pub use model::DedupObservation;
pub use model::PublicationDecision;
pub use model::PublicationIntentObservation;
pub use model::PublicationObservation;
pub use model::PublicationPhase;
pub use model::ReconcileInput;
pub use transition::reconcile;

#[cfg(test)]
mod tests;
