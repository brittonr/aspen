//! World replay, runtime-spine restore and branch-activation producers.
//!
//! Every scenario drives the production shell of its requirement: the world
//! replay service (`run_world_replay`), the runtime-spine logical snapshot
//! restore (`restore_local_logical_snapshot`) and the branch-authority shell
//! (`execute_world_branch_authority`). The shells own their decisions and
//! receipts; this module supplies only the caller-owned ports those shells
//! require, each backed by a real store or check: the world-commit capture shell
//! over `LocalWorldCommitStore`, the committed `RuntimeState`, capability
//! admission (`admit_capability`) over Ed25519-signed tokens, and durable files
//! under one scratch node root.
//!
//! Fixture trust: the runtime-spine restore, replay authority and branch
//! capability require independently signed admission material. The producer
//! derives fixed Ed25519 keys from labelled BLAKE3 key derivation, so every
//! re-execution signs identical bytes. No production check is weakened.
//!
//! Observations exclude every value bound to the executing build (snapshot
//! cohort, descriptor and restore receipt refs measure `/proc/self/exe`); the
//! executable is bound by the qualification subject instead. World-commit roots
//! are derived from the committed runtime state, so replay receipts are
//! identical for the CLI and the unit-test binary.

mod activation;
mod replay;
mod restore;
mod runtime;
#[cfg(test)]
mod tests;

pub(super) fn run(scenario: super::super::Scenario) -> crate::error::Result<super::Evidence> {
    match scenario {
        super::super::Scenario::WorldReplayAdmitted => replay::admitted(),
        super::super::Scenario::WorldReplayAuthorityDenied => replay::authority_denied(true),
        super::super::Scenario::WorldRestoreCurrentAdmission => restore::current_admission(),
        super::super::Scenario::WorldRestoreStaleAdmissionDenied => restore::stale_admission_denied(true),
        super::super::Scenario::WorldActivationCurrentFacts => activation::current_facts(),
        super::super::Scenario::WorldActivationRevokedDenied => activation::revoked_denied(true),
        _ => Err(invalid("scenario is not a world replay, restore or activation scenario")),
    }
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    super::super::invalid(message)
}
