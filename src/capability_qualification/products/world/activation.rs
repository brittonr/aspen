//! Branch activation through the production branch-authority shell. The
//! runtime observes the current Basalt policy document, capability admission
//! of an Ed25519-signed branch token against the current revocation ledger,
//! durable activation records (replay) and scope at every call, so activation
//! rechecks the facts that hold when it runs, not those the plan saw.

mod ports;
mod state;

use crate::world_branch_authority::CurrentBranchPolicyPort;

const POLICY_PATH: &str = "storage/world-branch/policy.json";
const REVOCATION_PATH: &str = "storage/world-branch/revocations.preserves";
const GRANT_DIRECTORY: &str = "storage/world-branch/grants";
const ACTIVATION_DIRECTORY: &str = "storage/world-branch/activations";
const RECEIPT_DIRECTORY: &str = "storage/world-branch/receipts";
const REVOCATION_RECORD: &str = "aspen-qualification-branch-revocations-v1";
const TOKEN_KIND: &str = "world-branch-authority";
const BRANCH_ABILITY: &str = "world/branch/create";
const SCOPE_RESOURCE: &str = "world/branch/qualification";
const SCOPE_LIMIT: u64 = 64;
const TOKEN_EXPIRY_TICK: u64 = 64;
/// The logical tick at which the revocation lands: after planning (tick 1)
/// and before the activation recheck (tick 2).
const REVOCATION_TICK: u64 = 2;
const MAX_STATE_BYTES: u64 = 65_536;

fn source_branch_ref() -> String {
    super::runtime::reference("aspen qualification source world branch")
}

fn branch_authority_ref() -> String {
    super::runtime::reference("aspen qualification world branch capability")
}

fn facts(
    runtime: &mut state::BranchRuntime,
) -> crate::error::Result<molten_core::world_branch_authority::WorldBranchAuthorityFacts> {
    let scope = molten_core::world_branch_authority::NormalizedCapabilityScope::new(
        SCOPE_RESOURCE,
        ["read".to_string(), "write".to_string()],
        Some(SCOPE_LIMIT),
    )
    .map_err(|diagnostic| super::invalid(format!("branch scope: {diagnostic:?}")))?;
    Ok(molten_core::world_branch_authority::WorldBranchAuthorityFacts {
        capability_kind: molten_core::world_branch_authority::CapabilityKind::PublicArtifact,
        action: molten_core::world_branch_authority::WorldBranchAction::Create,
        source_branch_ref: source_branch_ref(),
        destination_branch_ref: super::runtime::reference("aspen qualification destination world branch"),
        capability_ref: branch_authority_ref(),
        source_scope: scope.clone(),
        destination_scope: scope,
        policy_generation: runtime.observe_policy()?.generation,
        mapping_lossless: true,
    })
}

// r[impl molten.world_branch_authority.activation]
pub(super) fn current_facts() -> crate::error::Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-activation")?;
    let mut evidence = super::super::Evidence::default();
    let mut runtime = state::BranchRuntime::open(scratch.path(), None)?;
    let facts = facts(&mut runtime)?;
    let execution = crate::world_branch_authority::execute_world_branch_authority(&facts, &mut runtime)?;
    let activation = execution.activation.as_ref();
    evidence.check("plan-admitted-by-current-policy", execution.plan.allowed);
    evidence.check(
        "authority-rechecked-at-activation",
        runtime.observations.len() == 2
            && runtime
                .observations
                .iter()
                .all(molten_core::world_branch_authority::CurrentAuthorityFacts::all_current)
            && runtime.observations[0].observation_ref != runtime.observations[1].observation_ref,
    );
    evidence.check(
        "activation-admitted-on-current-facts",
        activation.is_some_and(|decision| {
            decision.allowed
                && decision.diagnostic == molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic::Admitted
        }),
    );
    evidence.check(
        "activation-performed-once",
        execution.activation_outcome == Some(crate::world_branch_authority::ActivationOutcome::Activated)
            && runtime.activation_calls == 1,
    );
    evidence.check(
        "activation-durably-recorded",
        activation.is_some_and(|decision| {
            runtime
                .activation_record(&facts.destination_branch_ref)
                .is_ok_and(|record| record.as_deref() == Some(decision.decision_ref.as_str()))
        }),
    );
    evidence.check(
        "plan-activation-outcome-receipts-published",
        execution.receipt_refs.len() == 3 && runtime.receipts.len() == 3,
    );
    for receipt in runtime.receipts.clone() {
        evidence.runtime(receipt);
    }
    for admission in runtime.admissions.clone() {
        evidence.runtime(admission);
    }

    let replayed = crate::world_branch_authority::execute_world_branch_authority(&facts, &mut runtime)?;
    evidence.check(
        "replayed-activation-not-admitted",
        !replayed.activation.as_ref().is_some_and(|decision| decision.allowed)
            && replayed.activation_outcome.is_none()
            && runtime.activation_calls == 1
            && runtime.observations.iter().skip(2).any(|current| !current.replay_current),
    );
    Ok(evidence)
}

/// A revocation that lands after planning denies activation on the recheck;
/// `revoke = false` is the control that must activate instead.
// r[impl molten.world_branch_authority.activation]
pub(super) fn revoked_denied(revoke: bool) -> crate::error::Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-activation-revoked")?;
    let mut evidence = super::super::Evidence::default();
    let mut runtime = state::BranchRuntime::open(scratch.path(), revoke.then_some(REVOCATION_TICK))?;
    let facts = facts(&mut runtime)?;
    let execution = crate::world_branch_authority::execute_world_branch_authority(&facts, &mut runtime)?;
    evidence.check("plan-admitted-on-earlier-facts", execution.plan.allowed);
    evidence.check(
        "revocation-observed-at-activation",
        runtime.observations.len() == 2
            && runtime.observations[0].all_current()
            && !runtime.observations[1].revocation_current
            && !runtime.observations[1].capability_current,
    );
    evidence.check(
        "activation-denied-on-current-facts",
        execution.activation.as_ref().is_some_and(|decision| {
            !decision.allowed
                && decision.diagnostic
                    == molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic::CurrentnessDenied
        }),
    );
    evidence.check("no-activation-attempted", runtime.activation_calls == 0 && execution.activation_outcome.is_none());
    evidence.check("no-activation-recorded", runtime.activation_record(&facts.destination_branch_ref)?.is_none());
    evidence.check("denial-receipt-published", execution.receipt_refs.len() == 2 && runtime.receipts.len() == 2);
    for receipt in runtime.receipts {
        evidence.runtime(receipt);
    }
    for admission in runtime.admissions {
        evidence.runtime(admission);
    }
    Ok(evidence)
}
