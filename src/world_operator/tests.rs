mod support;

use molten_core::world_operator::*;
use support::*;

use super::*;

// r[impl molten.world_operator.dogfood]
// r[verify molten.world_operator.dogfood]
#[test]
fn logical_dogfood_preview_and_apply_link_every_component_in_order() {
    let request = logical_request();
    let mut preview_handlers = fixture_handlers(None);
    let mut preview_refs = handler_refs(&mut preview_handlers);
    let preview = preview_world_operator_with_handlers(&request, &mut preview_refs).expect("logical preview");
    assert_eq!(preview.receipt.completion, WorldWorkflowCompletionState::Planned);
    assert_eq!(preview.receipt.links.len(), preview.plan.operations.len());

    let mut apply_handlers = fixture_handlers(None);
    let mut apply_refs = handler_refs(&mut apply_handlers);
    let mut facts = FixtureFacts {
        stale: false,
        observations: 0,
    };
    let apply = apply_world_operator_with_handlers(&request, &preview.plan.plan_ref, &mut apply_refs, &mut facts)
        .expect("logical apply");

    assert_eq!(apply.receipt.completion, WorldWorkflowCompletionState::Complete);
    assert_eq!(apply.receipt.links.len(), apply.plan.operations.len() + apply.plan.operations.len());
    assert_eq!(facts.observations, mutating_operation_count(&apply.plan));
    assert!(apply.rendered_summary.contains("release_eligible=false"));
    assert!(apply.rendered_summary.contains("deletion_authorized=false"));
}

#[test]
fn stale_plan_and_stale_generation_stop_before_the_owned_mutation() {
    let request = logical_request();
    let plan = plan_world_operator_request(&request).expect("workflow plan");
    let mut handlers = fixture_handlers(None);
    let mut refs = handler_refs(&mut handlers);
    let mut facts = FixtureFacts {
        stale: false,
        observations: 0,
    };
    let stale_plan = apply_world_operator_with_handlers(&request, &reference("stale-plan"), &mut refs, &mut facts)
        .expect("stale plan receipt");
    assert_eq!(stale_plan.receipt.completion, WorldWorkflowCompletionState::Blocked);
    assert_eq!(
        stale_plan.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::StalePlan)
    );
    assert!(handlers.iter().all(|handler| handler.execute_calls == 0));

    let mut handlers = fixture_handlers(None);
    let mut refs = handler_refs(&mut handlers);
    let mut facts = FixtureFacts {
        stale: true,
        observations: 0,
    };
    let stale_generation = apply_world_operator_with_handlers(&request, &plan.plan.plan_ref, &mut refs, &mut facts)
        .expect("stale generation receipt");
    assert_eq!(
        stale_generation.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::MutableObservationDrift)
    );
    let checkpoint = handlers
        .iter()
        .find(|handler| handler.kind == WorldOperationKind::Checkpoint)
        .expect("checkpoint handler");
    assert_eq!(checkpoint.execute_calls, 0);
}

#[test]
fn unknown_component_outcome_stops_later_operations_without_retry() {
    let request = logical_request();
    let plan = plan_world_operator_request(&request).expect("workflow plan");
    let mut handlers = fixture_handlers(Some((WorldOperationKind::Promote, WorldComponentCompletionState::Unknown)));
    let mut refs = handler_refs(&mut handlers);
    let mut facts = FixtureFacts {
        stale: false,
        observations: 0,
    };
    let run = apply_world_operator_with_handlers(&request, &plan.plan.plan_ref, &mut refs, &mut facts)
        .expect("unknown outcome receipt");

    assert_eq!(run.receipt.completion, WorldWorkflowCompletionState::Unknown);
    assert_eq!(
        run.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ComponentOutcomeUnknown)
    );
    let export = handlers.iter().find(|handler| handler.kind == WorldOperationKind::Export).expect("export handler");
    assert_eq!(export.preview_calls, 0);
    assert_eq!(export.execute_calls, 0);
}

#[test]
fn missing_handler_and_unavailable_stronger_profile_are_explicit_blockers() {
    let request = logical_request();
    let mut handlers = fixture_handlers(None);
    handlers.retain(|handler| handler.kind != WorldOperationKind::Replay);
    let mut refs = handler_refs(&mut handlers);
    let preview = preview_world_operator_with_handlers(&request, &mut refs).expect("missing handler receipt");
    assert_eq!(
        preview.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::HandlerUnavailable)
    );

    let mut witnessed = single_request(WorldOperationKind::Branch, WorldProfileKind::WitnessedHead);
    witnessed.profiles[0].status = WorldProfileStatus::Unavailable;
    let plan = plan_world_operator_request(&witnessed).expect("unavailable witness plan");
    assert_eq!(
        plan.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ProfileUnavailable)
    );

    let mut extent = single_request(WorldOperationKind::Run, WorldProfileKind::ExecutableExtent);
    extent.profiles[0].status = WorldProfileStatus::Unsupported;
    let plan = plan_world_operator_request(&extent).expect("unsupported executable extent plan");
    assert_eq!(
        plan.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::ProfileUnsupported)
    );
}

#[test]
fn exact_opaque_replay_completes_without_semantic_fallback() {
    let request = single_request(WorldOperationKind::Replay, WorldProfileKind::Opaque);
    let mut handler = FixtureHandler::new(WorldOperationKind::Replay);
    let mut refs: Vec<&mut dyn WorldOperationHandler> = vec![&mut handler];
    let mut facts = FixtureFacts {
        stale: false,
        observations: 0,
    };
    let plan = plan_world_operator_request(&request).expect("opaque replay plan");
    let run = apply_world_operator_with_handlers(&request, &plan.plan.plan_ref, &mut refs, &mut facts)
        .expect("opaque replay apply");
    assert_eq!(run.receipt.completion, WorldWorkflowCompletionState::Complete);
    assert_eq!(facts.observations, 0);

    let diff = single_request(WorldOperationKind::Diff, WorldProfileKind::Opaque);
    let blocked = plan_world_operator_request(&diff).expect("opaque diff receipt");
    assert_eq!(
        blocked.receipt.first_blocker.as_ref().map(|blocker| blocker.code),
        Some(WorldWorkflowBlockerCode::OpaqueSemanticOperation)
    );
}

// r[verify molten.world_operator.verification]
#[test]
fn component_overclaim_and_sensitive_flags_are_rejected_before_aggregate_publication() {
    let request = single_request(WorldOperationKind::Inspect, WorldProfileKind::Logical);
    let mut handler = FixtureHandler::new(WorldOperationKind::Inspect);
    handler.owner = WorldComponentOwner::WorldHead;
    let mut refs: Vec<&mut dyn WorldOperationHandler> = vec![&mut handler];
    let error = preview_world_operator_with_handlers(&request, &mut refs).expect_err("crossed component owner denied");
    assert!(error.to_string().contains("crosses a component owner boundary"));

    let mut handler = FixtureHandler::new(WorldOperationKind::Inspect);
    handler.claims_authority = true;
    let mut refs: Vec<&mut dyn WorldOperationHandler> = vec![&mut handler];
    let error = preview_world_operator_with_handlers(&request, &mut refs).expect_err("authority overclaim denied");
    assert!(error.to_string().contains("ReceiptOverclaimsAuthority"));

    let mut handler = FixtureHandler::new(WorldOperationKind::Inspect);
    handler.sensitive_material_present = true;
    let mut refs: Vec<&mut dyn WorldOperationHandler> = vec![&mut handler];
    let error = preview_world_operator_with_handlers(&request, &mut refs).expect_err("sensitive receipt denied");
    assert!(error.to_string().contains("ReceiptContainsSensitiveMaterial"));
}

#[test]
fn canonical_records_and_publication_order_are_stable_and_evidence_only() {
    let request = single_request(WorldOperationKind::Inspect, WorldProfileKind::Logical);
    let first = plan_world_operator_request(&request).expect("first plan");
    let second = plan_world_operator_request(&request).expect("second plan");
    assert_eq!(first.plan.plan_ref, second.plan.plan_ref);
    assert_eq!(first.plan_record.bytes, second.plan_record.bytes);
    assert_eq!(first.receipt_record.bytes, second.receipt_record.bytes);
    assert_eq!(first.summary_record.bytes, second.summary_record.bytes);

    let mut port = RecordingPort::default();
    let published = publish_world_operator_run(&first, &mut port).expect("publish records");
    assert_eq!(published, port.refs);
    assert_eq!(published.len(), EXPECTED_PUBLISHED_RECORDS);
    assert!(
        first
            .summary
            .non_claims
            .contains(&"plans and receipts do not grant branch or effect authority".to_string())
    );
}
