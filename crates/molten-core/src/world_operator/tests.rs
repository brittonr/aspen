mod support;

use support::*;

use super::*;

#[test]
fn complete_logical_workflow_is_stable_under_input_permutation() {
    let request = logical_request();
    let first = plan_world_workflow(&request).expect("logical workflow plan");
    let mut permuted = request;
    permuted.operations.reverse();
    permuted.profiles.reverse();
    permuted.observations.reverse();
    let second = plan_world_workflow(&permuted).expect("permuted logical workflow plan");

    assert_eq!(first.plan_ref, second.plan_ref);
    assert_eq!(first.operations, second.operations);
    assert!(first.first_blocker.is_none());
    assert_eq!(first.operations.len(), WorldOperationKind::all_for_tests().len());
}

#[test]
fn opaque_replay_is_admitted_but_semantic_diff_is_blocked() {
    let mut request = single_operation_request(WorldOperationKind::Replay, WorldProfileKind::Opaque);
    let world_ref = request.world_ref.as_str().to_string();
    add_observation(&mut request, WorldExpectedObservationKind::CapsuleClosure, &world_ref, true);
    let replay = plan_world_workflow(&request).expect("opaque replay plan");
    assert!(replay.first_blocker.is_none());

    request.operations[0].kind = WorldOperationKind::Diff;
    let diff = plan_world_workflow(&request).expect("opaque diff plan");
    assert_eq!(
        diff.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::OpaqueSemanticOperation)
    );
}

#[test]
fn unavailable_profiles_and_denied_observations_fail_closed() {
    let mut request = single_operation_request(WorldOperationKind::Checkpoint, WorldProfileKind::Logical);
    request.profiles[0].status = WorldProfileStatus::Unavailable;
    let unavailable = plan_world_workflow(&request).expect("unavailable profile plan");
    assert_eq!(
        unavailable.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ProfileUnavailable)
    );

    request.profiles[0].status = WorldProfileStatus::Admitted;
    let authority_position = request
        .observations
        .iter()
        .position(|observation| observation.kind == WorldExpectedObservationKind::Authority)
        .expect("authority observation");
    request.observations[authority_position].admitted = false;
    let denied = plan_world_workflow(&request).expect("denied authority plan");
    assert_eq!(
        denied.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::AuthorityObservationDenied)
    );

    request.observations[authority_position].admitted = true;
    request.observations[authority_position].observation_ref = reference("crossed-authority-observation");
    let crossed = plan_world_workflow(&request).expect("crossed authority plan");
    assert_eq!(
        crossed.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::AuthorityObservationDenied)
    );
}

#[test]
fn operation_specific_observations_and_missing_profiles_block_exactly() {
    let mut replay = single_operation_request(WorldOperationKind::Replay, WorldProfileKind::Logical);
    let capsule = replay
        .observations
        .iter_mut()
        .find(|observation| observation.kind == WorldExpectedObservationKind::CapsuleClosure)
        .expect("capsule observation");
    capsule.admitted = false;
    let replay_plan = plan_world_workflow(&replay).expect("incomplete capsule plan");
    assert_eq!(
        replay_plan.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::CapsuleIncomplete)
    );

    let mut promotion = single_operation_request(WorldOperationKind::Promote, WorldProfileKind::Logical);
    let conflict = promotion
        .observations
        .iter_mut()
        .find(|observation| observation.kind == WorldExpectedObservationKind::Conflict)
        .expect("conflict observation");
    conflict.admitted = false;
    let promotion_plan = plan_world_workflow(&promotion).expect("unresolved conflict plan");
    assert_eq!(
        promotion_plan.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ConflictUnresolved)
    );

    let mut missing_profile = single_operation_request(WorldOperationKind::Inspect, WorldProfileKind::Logical);
    missing_profile.profiles.clear();
    let missing_plan = plan_world_workflow(&missing_profile).expect("missing profile plan");
    assert_eq!(
        missing_plan.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ProfileUnavailable)
    );
}

#[test]
fn dependency_cycles_missing_dependencies_and_duplicate_ids_are_rejected() {
    let mut cycle = logical_request();
    let first_id = cycle.operations.first().expect("first operation").operation_id.clone();
    let last_id = cycle.operations.last().expect("last operation").operation_id.clone();
    cycle.operations.first_mut().expect("first operation").dependencies.push(last_id);
    let cycle_issues = plan_world_workflow(&cycle).expect_err("dependency cycle denied");
    assert!(cycle_issues.contains(&WorldWorkflowIssue::DependencyCycle));

    let mut missing = logical_request();
    let missing_ref = reference("missing-operation");
    missing.operations.first_mut().expect("first operation").dependencies.push(missing_ref.clone());
    let missing_issues = plan_world_workflow(&missing).expect_err("missing dependency denied");
    assert!(missing_issues.contains(&WorldWorkflowIssue::MissingDependency(missing_ref)));

    let mut duplicate = logical_request();
    duplicate.operations.last_mut().expect("last operation").operation_id = first_id.clone();
    let duplicate_issues = plan_world_workflow(&duplicate).expect_err("duplicate operation denied");
    assert!(duplicate_issues.contains(&WorldWorkflowIssue::DuplicateOperation(first_id)));
}

#[test]
fn apply_requires_exact_plan_and_fresh_mutable_observations() {
    let request = single_operation_request(WorldOperationKind::Checkpoint, WorldProfileKind::Logical);
    let plan = plan_world_workflow(&request).expect("checkpoint plan");
    let operation = plan.operations.first().expect("checkpoint operation");
    let facts = current_facts(&plan, operation);
    let admission = admit_world_operation_apply(&plan, &facts).expect("fresh apply admitted");
    assert!(admission.admitted);

    let mut stale = facts.clone();
    stale.observed_generation = STALE_GENERATION;
    let stale_issues = admit_world_operation_apply(&plan, &stale).expect_err("stale generation denied");
    assert!(stale_issues.contains(&WorldWorkflowIssue::ApplyGenerationMismatch));

    let mut crossed = facts;
    crossed.plan_ref = reference("crossed-plan");
    let crossed_issues = admit_world_operation_apply(&plan, &crossed).expect_err("crossed plan denied");
    assert!(crossed_issues.contains(&WorldWorkflowIssue::ApplyPlanMismatch));
}

#[test]
fn aggregate_receipt_preserves_order_nonclaims_and_complete_links() {
    let plan = plan_world_workflow(&logical_request()).expect("logical workflow plan");
    let links = plan
        .operations
        .iter()
        .map(|operation| completed_link(operation, WorldReceiptRole::ComponentReceipt))
        .collect();
    let receipt = build_world_workflow_receipt(&plan, links, None).expect("complete receipt");
    let summary = summarize_world_workflow(&plan, &receipt).expect("workflow summary");
    let rendered = render_world_workflow_summary(&summary).expect("render summary");

    assert_eq!(receipt.completion, WorldWorkflowCompletionState::Complete);
    assert_eq!(receipt.non_claims, world_operator_non_claims());
    assert!(rendered.contains("authority_granted=false"));
    assert!(rendered.contains("deletion_authorized=false"));
}

#[test]
fn aggregate_receipt_stops_at_unknown_and_rejects_later_success() {
    let plan = plan_world_workflow(&logical_request()).expect("logical workflow plan");
    let replay_position = plan
        .operations
        .iter()
        .position(|operation| operation.kind == WorldOperationKind::Replay)
        .expect("replay operation");
    let mut links = plan.operations[..replay_position]
        .iter()
        .map(|operation| completed_link(operation, WorldReceiptRole::ComponentReceipt))
        .collect::<Vec<_>>();
    let replay = &plan.operations[replay_position];
    let mut unknown = completed_link(replay, WorldReceiptRole::Reconciliation);
    unknown.state = WorldComponentCompletionState::Unknown;
    links.push(unknown);
    let receipt = build_world_workflow_receipt(&plan, links.clone(), None).expect("unknown receipt");
    assert_eq!(receipt.completion, WorldWorkflowCompletionState::Unknown);
    assert_eq!(
        receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ComponentOutcomeUnknown)
    );

    let later = plan.operations.get(replay_position + 1).expect("later operation");
    links.push(completed_link(later, WorldReceiptRole::ComponentReceipt));
    let issues = build_world_workflow_receipt(&plan, links, None).expect_err("later success denied");
    assert!(issues.contains(&WorldWorkflowIssue::ReceiptAfterBlocker));
}

#[test]
fn aggregate_receipt_rejects_authority_deletion_and_sensitive_overclaims() {
    let plan = plan_world_workflow(&single_operation_request(
        WorldOperationKind::GarbageCollectionPlan,
        WorldProfileKind::Logical,
    ))
    .expect("gc plan");
    let operation = plan.operations.first().expect("gc operation");

    let mut crossed_owner = completed_link(operation, WorldReceiptRole::ComponentReceipt);
    crossed_owner.owner = WorldComponentOwner::WorldHead;
    assert_receipt_issue(&plan, crossed_owner, WorldWorkflowIssue::ReceiptOwnerMismatch);

    let mut authority = completed_link(operation, WorldReceiptRole::ComponentReceipt);
    authority.claims_authority = true;
    assert_receipt_issue(&plan, authority, WorldWorkflowIssue::ReceiptOverclaimsAuthority);

    let mut deletion = completed_link(operation, WorldReceiptRole::ComponentReceipt);
    deletion.claims_deletion_authority = true;
    assert_receipt_issue(&plan, deletion, WorldWorkflowIssue::ReceiptOverclaimsDeletionAuthority);

    let mut sensitive = completed_link(operation, WorldReceiptRole::ComponentReceipt);
    sensitive.sensitive_material_present = true;
    assert_receipt_issue(&plan, sensitive, WorldWorkflowIssue::ReceiptContainsSensitiveMaterial);
}
