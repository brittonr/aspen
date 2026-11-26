//! Shell for the public register history: projection and audit live in
//! `molten-reliability-model-core`; this module binds the `serde_json` publication size port.

pub use molten_reliability_model_core::history::AttemptObservation;
pub use molten_reliability_model_core::history::CheckerBounds;
pub use molten_reliability_model_core::history::Completeness;
pub use molten_reliability_model_core::history::Document;
pub use molten_reliability_model_core::history::Event;
pub use molten_reliability_model_core::history::Input;
pub use molten_reliability_model_core::history::Issue;
pub use molten_reliability_model_core::history::Outcome;
pub use molten_reliability_model_core::history::Profile;
pub use molten_reliability_model_core::history::PublicOutcome;
pub use molten_reliability_model_core::history::attempt_id;
pub use molten_reliability_model_core::history::project_attempt;

/// Replays `history` against the admitted plan, bounding its published `serde_json` byte size.
pub fn validate(
    admitted: &crate::live_reliability::profile::Admitted,
    plan: &crate::live_reliability::generator::OperationPlan,
    history: &Document,
) -> Result<(), Issue> {
    molten_reliability_model_core::history::validate(admitted, plan, history, encoded_len)
}

fn encoded_len(history: &Document) -> Result<u64, Issue> {
    let bytes = serde_json::to_vec(history).map_err(|_| Issue::Bounds)?;
    u64::try_from(bytes.len()).map_err(|_| Issue::Bounds)
}
