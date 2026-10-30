use super::super::domain;
use super::super::validation;
use super::support;
use crate::world_head;

/// One input substitution and the issue validation must report for it.
type SubstitutionCase = (fn(&mut domain::CurrentnessInput), domain::WitnessIssue);

// r[verify molten.world_witness.verification]
#[test]
fn admitted_checkpoint_advances_and_repeated_current_read_is_bounded() {
    let mut input = support::input();
    let admitted = validation::validate_committed_currentness(&input).unwrap();
    let highest = admitted.highest.unwrap();
    assert_eq!(highest.subject, input.subject);
    input.retained = domain::RetainedWitnessState::Present(highest.clone());
    support::observation(&mut input).consistency.predecessor = Some(highest.checkpoint.checkpoint_id);
    assert_eq!(validation::validate_currentness(&input).unwrap().profile, input.selected_profile);
    input.now = 111;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::StaleCheckpoint));
}

// r[verify molten.world_witness.currentness]
#[test]
fn provider_domain_generation_and_operation_substitution_are_denied() {
    let cases: [SubstitutionCase; 5] = [
        (
            |i| support::observation(i).contract.source_revision = "substitute".into(),
            domain::WitnessIssue::ProviderSubstitution,
        ),
        (
            |i| support::observation(i).append.subject.branch_id = world_head::WorldBranchId::new("other").unwrap(),
            domain::WitnessIssue::DomainSubstitution,
        ),
        (
            |i| support::observation(i).inclusion.subject.generation += 1,
            domain::WitnessIssue::DomainSubstitution,
        ),
        (
            |i| {
                support::observation(i).current_head.subject.claim_ref =
                    world_head::WorldHeadClaimRef::new(support::reference("other")).unwrap()
            },
            domain::WitnessIssue::DomainSubstitution,
        ),
        (
            |i| support::observation(i).append.operation_id = support::reference("other-operation"),
            domain::WitnessIssue::OperationSubstitution,
        ),
    ];
    for (mutate, expected) in cases {
        let mut input = support::input();
        mutate(&mut input);
        assert_eq!(validation::validate_currentness(&input), Err(expected));
    }
}

// r[verify molten.world_witness.verification]
#[test]
fn invalid_proofs_quorum_currentness_and_checkpoint_are_denied() {
    let cases: [SubstitutionCase; 7] = [
        (|i| support::observation(i).inclusion.verified = false, domain::WitnessIssue::InvalidInclusion),
        (|i| support::observation(i).consistency.verified = false, domain::WitnessIssue::FailedConsistency),
        (
            |i| support::observation(i).checkpoint.signature_verified = false,
            domain::WitnessIssue::InvalidCheckpoint,
        ),
        (|i| support::observation(i).quorum.witnesses.clear(), domain::WitnessIssue::InsufficientQuorum),
        (
            |i| {
                support::observation(i).quorum.witnesses.insert("other".into());
            },
            domain::WitnessIssue::InsufficientQuorum,
        ),
        (|i| support::observation(i).current_head.verified = false, domain::WitnessIssue::MissingCurrentness),
        (|i| support::observation(i).checkpoint.observed_at = 106, domain::WitnessIssue::StaleCheckpoint),
    ];
    for (mutate, expected) in cases {
        let mut input = support::input();
        mutate(&mut input);
        assert_eq!(validation::validate_currentness(&input), Err(expected));
    }
}

// r[verify molten.world_witness.profiles]
#[test]
fn strongest_retained_assurance_cannot_be_reset_or_downgraded() {
    let mut input = support::input();
    let highest = validation::validate_currentness(&input).unwrap().highest.unwrap();
    input.retained = domain::RetainedWitnessState::Missing;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::MissingHighestState));
    input.retained = domain::RetainedWitnessState::Present(highest);
    input.selected_profile = domain::AssuranceProfile::LocalGenerationV1;
    input.policy.required_profile = domain::AssuranceProfile::LocalGenerationV1;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::ProfileDowngrade));
    input.retained = domain::RetainedWitnessState::Empty;
    input.observation = None;
    input.local_generation = 3;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::StaleGeneration));
    input.local_generation = 2;
    assert_eq!(validation::validate_currentness(&input).unwrap().highest, None);
    input.policy.required_profile = domain::AssuranceProfile::IndependentWitnessV1;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::ProfileDowngrade));
}

// r[verify molten.world_witness.verification]
#[test]
fn whole_store_rollback_and_checkpoint_fork_are_not_hidden_by_valid_signatures() {
    let mut input = support::input();
    let mut highest = validation::validate_currentness(&input).unwrap().highest.unwrap();
    highest.subject.generation = 3;
    highest.checkpoint.revision = 4;
    input.retained = domain::RetainedWitnessState::Present(highest.clone());
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::StaleCheckpoint));
    // A rolled-back local store cannot suppress a fresh provider current-head read.
    input.retained = domain::RetainedWitnessState::Empty;
    support::observation(&mut input).current_head.subject.generation = 3;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::DomainSubstitution));
    support::observation(&mut input).current_head.subject.generation = 2;
    highest.subject.generation = 2;
    highest.checkpoint.revision = 3;
    highest.checkpoint.checkpoint_id = support::reference("fork");
    input.retained = domain::RetainedWitnessState::Present(highest);
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::Fork));
}

// r[verify molten.world_witness.reconciliation]
#[test]
fn unavailable_fork_and_reserved_observations_keep_distinct_outcomes() {
    let mut input = support::input();
    support::observation(&mut input).current_head.phase = domain::CurrentHeadPhase::Reserved;
    assert!(validation::validate_currentness(&input).is_ok());
    assert_eq!(validation::validate_committed_currentness(&input), Err(domain::WitnessIssue::MissingCurrentness));
    input.observation = Some(domain::WitnessObservation::Unavailable {
        append_may_have_occurred: true,
    });
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::UncertainAppend));
    input.observation = Some(domain::WitnessObservation::Unavailable {
        append_may_have_occurred: false,
    });
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::Unavailable));
    input.observation = Some(domain::WitnessObservation::Fork {
        provider_id: "reviewed-provider".into(),
        checkpoint_ids: Default::default(),
    });
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::Fork));
}
