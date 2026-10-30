use std::collections;

use super::super::domain;
use super::super::planning;
use super::super::validation;
use super::support;

fn witnesses(count: usize) -> collections::BTreeSet<String> {
    (0..count).map(|index| format!("fixture-witness-{index}")).collect()
}

// r[verify molten.world_witness.verification]
#[test]
fn maximum_witness_quorum_is_admitted_but_policy_or_proof_overflow_is_denied() {
    let mut input = support::input();
    let maximum = witnesses(domain::MAX_WITNESSES);
    input.policy.allowed_witnesses = maximum.clone();
    input.policy.minimum_quorum = domain::MAX_WITNESSES;
    support::observation(&mut input).quorum.witnesses = maximum.clone();
    assert_eq!(validation::validate_currentness(&input).unwrap().highest.unwrap().quorum, maximum);
    input.policy.allowed_witnesses = witnesses(domain::MAX_WITNESSES + 1);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
    input.policy.allowed_witnesses = maximum;
    support::observation(&mut input).quorum.witnesses = witnesses(domain::MAX_WITNESSES + 1);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
}

// r[verify molten.world_witness.currentness]
#[test]
fn retained_quorum_and_fork_evidence_cannot_bypass_collection_bounds() {
    let mut input = support::input();
    let mut highest = validation::validate_currentness(&input).unwrap().highest.unwrap();
    highest.quorum = witnesses(domain::MAX_WITNESSES + 1);
    input.retained = domain::RetainedWitnessState::Present(highest);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
    input.retained = domain::RetainedWitnessState::Empty;
    input.observation = Some(domain::WitnessObservation::Fork {
        provider_id: input.policy.provider.provider_id.clone(),
        checkpoint_ids: witnesses(domain::MAX_WITNESSES + 1),
    });
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
}

// r[verify molten.world_witness.provider_boundary]
#[test]
fn exact_contract_byte_boundary_is_preserved_without_truncation() {
    let mut input = support::input();
    input.policy.provider.contract_id = "c".repeat(domain::MAX_WITNESS_CONTRACT_BYTES);
    let contract = input.policy.provider.clone();
    support::observation(&mut input).contract = contract;
    assert_eq!(validation::validate_currentness(&input).unwrap().profile, input.selected_profile);
    input.policy.provider.contract_id.push('c');
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
}

// r[verify molten.world_witness.currentness]
#[test]
fn checkpoint_and_operation_labels_reject_overlong_inputs_before_domain_comparison() {
    let mut input = support::input();
    let checkpoint = "x".repeat(domain::MAX_WITNESS_LABEL_BYTES);
    let observation = support::observation(&mut input);
    observation.checkpoint.checkpoint_id = checkpoint.clone();
    observation.inclusion.checkpoint_id = checkpoint.clone();
    observation.consistency.checkpoint_id = checkpoint.clone();
    observation.quorum.checkpoint_id = checkpoint.clone();
    observation.current_head.checkpoint_id = checkpoint;
    assert_eq!(validation::validate_currentness(&input).unwrap().profile, input.selected_profile);
    support::observation(&mut input).checkpoint.checkpoint_id.push('x');
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
    input = support::input();
    input.operation_id = "o".repeat(domain::MAX_WITNESS_LABEL_BYTES + 1);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
}

// r[verify molten.world_witness.profiles]
#[test]
fn local_profile_and_staging_do_not_bypass_bounds() {
    let mut input = support::input();
    input.selected_profile = domain::AssuranceProfile::LocalGenerationV1;
    input.policy.required_profile = domain::AssuranceProfile::LocalGenerationV1;
    input.policy.allowed_witnesses = witnesses(domain::MAX_WITNESSES + 1);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::BoundsExceeded));
    let mut input = support::finalization();
    input.staged.operation_id = "o".repeat(domain::MAX_WITNESS_LABEL_BYTES + 1);
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::BoundsExceeded));
    let stage = planning::StageInput {
        staged: input.staged,
        authenticated: true,
        authority: input.authority,
    };
    assert_eq!(planning::plan_stage(&stage), Err(domain::WitnessIssue::BoundsExceeded));
}
