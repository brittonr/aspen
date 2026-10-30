use super::super::domain;
use super::super::validation;
use super::support;

// r[verify molten.world_witness.currentness]
#[test]
fn admitted_quorum_can_grow_but_retained_witness_strength_cannot_shrink() {
    let mut input = support::input();
    input.policy.allowed_witnesses.insert("independent-b".into());
    input.policy.minimum_quorum = 2;
    support::observation(&mut input).quorum.witnesses.insert("independent-b".into());
    let highest = validation::validate_currentness(&input).unwrap().highest.unwrap();
    input.retained = domain::RetainedWitnessState::Present(highest.clone());
    support::observation(&mut input).consistency.predecessor = Some(highest.checkpoint.checkpoint_id);
    input.policy.minimum_quorum = 1;
    support::observation(&mut input).quorum.witnesses.remove("independent-b");
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::InsufficientQuorum));
}

// r[verify molten.world_witness.verification]
#[test]
fn advance_must_extend_exact_highest_checkpoint_and_cannot_substitute_same_generation() {
    let mut input = support::input();
    let highest = validation::validate_currentness(&input).unwrap().highest.unwrap();
    input.retained = domain::RetainedWitnessState::Present(highest.clone());
    support::observation(&mut input).checkpoint.revision += 1;
    support::observation(&mut input).checkpoint.checkpoint_id = support::reference("next-checkpoint");
    let next_checkpoint = support::reference("next-checkpoint");
    support::observation(&mut input).inclusion.checkpoint_id = next_checkpoint.clone();
    support::observation(&mut input).consistency.checkpoint_id = next_checkpoint.clone();
    support::observation(&mut input).quorum.checkpoint_id = next_checkpoint.clone();
    support::observation(&mut input).current_head.checkpoint_id = next_checkpoint;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::FailedConsistency));
    support::observation(&mut input).consistency.predecessor = Some(highest.checkpoint.checkpoint_id);
    input.subject.claim_ref = crate::world_head::WorldHeadClaimRef::new(support::reference("conflicting")).unwrap();
    let subject = input.subject.clone();
    support::observation(&mut input).append.subject = subject.clone();
    support::observation(&mut input).inclusion.subject = subject.clone();
    support::observation(&mut input).current_head.subject = subject;
    assert_eq!(validation::validate_currentness(&input), Err(domain::WitnessIssue::Fork));
    input.subject.generation += 1;
    let subject = input.subject.clone();
    support::observation(&mut input).append.subject = subject.clone();
    support::observation(&mut input).inclusion.subject = subject.clone();
    support::observation(&mut input).current_head.subject = subject;
    assert_eq!(validation::validate_currentness(&input).unwrap().highest.unwrap().subject, input.subject);
}
