use super::super::domain;
use super::super::planning;
use super::support;

// r[verify molten.world_witness.finalization]
#[test]
fn stage_precedes_request_and_exact_witnessed_finalization_is_admitted() {
    let input = support::finalization();
    let stage = planning::StageInput {
        staged: input.staged.clone(),
        authenticated: true,
        authority: input.authority.clone(),
    };
    let request = planning::plan_stage(&stage).unwrap();
    assert_eq!(request.subject, input.currentness.subject);
    assert_eq!(request.operation_id, input.staged.operation_id);
    let plan = planning::plan_finalization(&input).unwrap();
    assert_eq!(plan.transition.after, input.staged.transition.after);
    assert_eq!(plan.currentness.highest.unwrap().subject, input.currentness.subject);
}

// r[verify molten.world_witness.provider_boundary]
#[test]
fn witnessed_claim_never_overrides_denied_or_stale_authority() {
    let mut input = support::finalization();
    input.authority.admitted = false;
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::AuthorityDenied));
    input.authority.admitted = true;
    input.authority.observed_generation = 0;
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::AuthorityDenied));
    input.authority.observed_generation = 1;
    input.authority.policy_ref = crate::world_head::WorldHeadPolicyRef::new(support::reference("other")).unwrap();
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::AuthorityDenied));
}

// r[verify molten.world_witness.finalization]
#[test]
fn witnessed_orphan_cannot_move_head_after_local_contention_or_stage_replacement() {
    let mut input = support::finalization();
    input.current_head = Some(input.staged.transition.after.clone());
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::LocalConflict));
    input.current_head = input.staged.transition.before.clone();
    input.durable_stage = None;
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::InvalidStage));
    input.durable_stage = Some(input.staged.clone());
    input.currentness.operation_id = support::reference("substituted");
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::InvalidStage));
}

// r[verify molten.world_witness.profiles]
#[test]
fn local_only_evidence_cannot_publish_strong_policy_head() {
    let mut input = support::finalization();
    input.currentness.observation = None;
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::MissingCurrentness));
    input.currentness.selected_profile = domain::AssuranceProfile::LocalGenerationV1;
    input.staged.selected_profile = domain::AssuranceProfile::LocalGenerationV1;
    input.durable_stage = Some(input.staged.clone());
    assert_eq!(planning::plan_finalization(&input), Err(domain::WitnessIssue::ProfileDowngrade));
}

// r[verify molten.world_witness.finalization]
#[test]
fn unauthenticated_and_non_successor_claims_cannot_be_staged() {
    let input = support::finalization();
    let mut stage = planning::StageInput {
        staged: input.staged,
        authenticated: false,
        authority: input.authority,
    };
    assert_eq!(planning::plan_stage(&stage), Err(domain::WitnessIssue::InvalidStage));
    stage.authenticated = true;
    stage.staged.transition.after.generation = 4;
    assert_eq!(planning::plan_stage(&stage), Err(domain::WitnessIssue::InvalidStage));
}
