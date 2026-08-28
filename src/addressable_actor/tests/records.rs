use molten_core::addressable_actor::*;

use super::super::*;
use super::support::*;

#[test]
fn canonical_receipt_is_deterministic_and_authority_neutral() {
    let receipt = ActorCommitReceipt {
        actor_key_ref: reference("actor-key"),
        request_ref: reference("request"),
        operation_ref: reference("operation"),
        before_state_ref: reference("before"),
        planned_state_ref: reference("planned"),
        final_state_ref: reference("final"),
        revision: 1,
        status: ActorServiceStatus::Applied,
        currentness: ActorCommitCurrentness::Linearizable,
        durability: ActorDurabilityOutcome::Durable,
        engine_epoch: ENGINE_EPOCH,
        effect_observations: vec![ActorEffectObservation {
            effect_ref: reference("effect"),
            admission_ref: reference("admission"),
            disposition: ActorEffectDisposition::Succeeded,
            outcome_ref: Some(reference("outcome")),
        }],
        status_ref: Some(reference("status")),
        issue: None,
        authorizes_future_mutation: false,
        authorizes_effects: false,
        authorizes_retry: false,
        claims_exactly_once: false,
        claims_runtime_survival: false,
        non_claims: required_addressable_actor_non_claims(),
    };
    let first = canonical_actor_commit_receipt(&receipt).expect("first receipt");
    let second = canonical_actor_commit_receipt(&receipt).expect("second receipt");
    assert_eq!(first.receipt_ref, second.receipt_ref);
    assert_eq!(first.bytes, second.bytes);

    let mut overclaim = receipt;
    overclaim.authorizes_effects = true;
    assert!(canonical_actor_commit_receipt(&overclaim).is_err());
}

#[test]
fn status_identity_rejects_mutation_authority() {
    let profile = profile();
    let actor_key = actor_key();
    let state = initial_state(&profile, &actor_key);
    let mut status = project_actor_status(&state, ActorStatusProjectionInput {
        maximum_events: 1,
        evidence_refs: &[reference("evidence")],
    })
    .expect("status");
    let first = identify_canonical_actor_status(&status).expect("status identity");
    let second = identify_canonical_actor_status(&status).expect("status identity again");
    assert_eq!(first, second);

    status.authorizes_mutation = true;
    assert!(identify_canonical_actor_status(&status).is_err());
}
