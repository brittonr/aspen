use super::*;

// r[verify molten.world_replay_boundary.inventory]
// r[verify molten.world_replay_boundary.opaque_native]
#[test]
fn a_native_process_descriptor_binds_exact_identities_and_stays_detached() {
    let mut detached = valid_request();
    let mut native = diagnostic("chaoscontrol-native-cohort", vec![OpaqueDiagnosticClaim::DetachedDiagnostic]);
    native.native_process = Some(NativeProcessDiagnostic {
        profile_ref: digest("native-process-profile"),
        descriptor_ref: digest("native-process-descriptor"),
        producer_cohort: digest("chaoscontrol-native-cohort"),
        replay_class: NativeProcessReplayClass::RecordReplayMatched,
        record_count: 4,
        non_claims: vec![String::from("descriptor does not establish semantic equivalence")],
    });
    detached.boundary.opaque_diagnostics = vec![native];
    let admission = admit(&detached).expect("bound descriptor");
    assert_eq!(admission.opaque_diagnostic_refs, vec![digest("native-process-descriptor")]);

    let descriptor_ref = digest("native-process-descriptor");
    let mut swapped = detached.clone();
    if let Some(native) = &mut swapped.boundary.opaque_diagnostics[0].native_process {
        native.descriptor_ref = digest("another-descriptor");
    }
    assert_eq!(blockers(&swapped), vec![ReplayBoundaryBlocker::InvalidRecord("native-process-descriptor")]);

    let mut foreign = detached.clone();
    if let Some(native) = &mut foreign.boundary.opaque_diagnostics[0].native_process {
        native.producer_cohort = digest("another-cohort");
    }
    assert_eq!(blockers(&foreign), vec![ReplayBoundaryBlocker::OpaqueCohortMismatch(descriptor_ref.clone())]);

    let mut unborn = detached.clone();
    if let Some(native) = &mut unborn.boundary.opaque_diagnostics[0].native_process {
        native.record_count = 0;
    }
    assert!(blockers(&unborn).contains(&ReplayBoundaryBlocker::InvalidRecord("native-process-records")));

    let mut overclaim = detached.clone();
    if let Some(native) = &mut overclaim.boundary.opaque_diagnostics[0].native_process {
        native.non_claims.clear();
    }
    assert_eq!(blockers(&overclaim), vec![ReplayBoundaryBlocker::OpaqueOverclaim(descriptor_ref)]);
}

// r[verify molten.world_replay_boundary.completeness]
// r[verify molten.world_replay_boundary.verification]
#[test]
fn a_complete_boundary_is_admitted_and_bound_into_a_stable_plan() {
    let request = valid_request();
    let admission = admit(&request).expect("complete boundary");
    assert_eq!(admission, admit(&request).expect("stable admission"), "admission is stable");
    assert_eq!(
        (admission.deterministic_sources, admission.simulated_sources, admission.recorded_sources),
        (1, 2, 3),
        "every handling class is represented"
    );
    assert_eq!(admission.recorded_observations, 4);
    assert_eq!(admission.horizon, Some((second_step(&request), 1)));
    let send = &request.boundary.ledger.observations[2];
    assert_eq!(send.slot.source_id, SEND);
    assert_eq!(admission.sealed_effect_refs, vec![send.observation_ref.clone()], "the send is sealed");

    let plan = plan_world_replay(&request).expect("plan");
    assert_eq!(plan, plan_world_replay(&request).expect("stable plan"));
    assert_eq!(plan.inventory_ref, request.boundary.inventory.inventory_ref);
    assert_eq!(plan.ledger_ref, request.boundary.ledger.ledger_ref);
    let admit_position = plan
        .operations
        .iter()
        .position(|operation| operation.kind == WorldReplayOperationKind::AdmitObservationBoundary)
        .expect("boundary admission operation");
    let execute_position = plan
        .operations
        .iter()
        .position(|operation| operation.kind == WorldReplayOperationKind::ExecuteTransition)
        .expect("transition execution");
    assert!(admit_position < execute_position, "the boundary is admitted before any transition executes");
}

// r[verify molten.world_replay_boundary.inventory]
#[test]
fn an_unknown_source_or_an_omitted_inventory_row_blocks_replay() {
    let mut unknown = valid_request();
    with_ledger(&mut unknown, |ledger| {
        ledger.reached_sources.push("hostcall:subscribe".to_string());
        ledger.reached_sources.sort();
    });
    assert!(blockers(&unknown).contains(&ReplayBoundaryBlocker::UnknownSource("hostcall:subscribe".to_string())));

    let mut omitted = valid_request();
    with_inventory(&mut omitted, |inventory| inventory.sources.retain(|source| source.source_id != DELIVERY));
    assert!(blockers(&omitted).contains(&ReplayBoundaryBlocker::UnknownSource(DELIVERY.to_string())));

    let mut unsupported = valid_request();
    with_ledger(&mut unsupported, |ledger| {
        ledger.reached_sources.push(BLOB_PUT.to_string());
        ledger.reached_sources.sort();
    });
    assert_eq!(blockers(&unsupported), vec![ReplayBoundaryBlocker::UnsupportedSource(BLOB_PUT.to_string())]);
}

// r[verify molten.world_replay_boundary.completeness]
#[test]
fn a_missing_result_an_extra_value_and_a_duplicate_event_block_replay() {
    let mut missing = valid_request();
    with_ledger(&mut missing, |ledger| {
        ledger.observations.remove(0);
    });
    assert_eq!(blockers(&missing), vec![ReplayBoundaryBlocker::MissingObservation {
        step_position: first_step(&missing),
        logical_position: 0,
    }]);

    let mut extra = valid_request();
    let inventory = extra.boundary.inventory.clone();
    let stray = record(&inventory, slot(second_step(&extra), 2, DELIVERY, "delivery-3"), None);
    with_ledger(&mut extra, |ledger| ledger.observations.push(stray));
    assert_eq!(blockers(&extra), vec![ReplayBoundaryBlocker::ExtraObservation {
        step_position: second_step(&extra),
        logical_position: 2,
    }]);

    let mut duplicate = valid_request();
    with_ledger(&mut duplicate, |ledger| {
        let repeated = ledger.observations[3].clone();
        ledger.observations.push(repeated);
    });
    assert!(blockers(&duplicate).contains(&ReplayBoundaryBlocker::DuplicateObservation {
        step_position: second_step(&duplicate),
        logical_position: 1,
    }));
}

// r[verify molten.world_replay_boundary.observations]
#[test]
fn a_result_bound_to_another_request_is_rejected() {
    let mut wrong = valid_request();
    with_ledger(&mut wrong, |ledger| {
        ledger.observations[0].slot.request_ref = digest("another-request");
        ledger.observations[0] = reidentify_observation(ledger.observations[0].clone());
    });
    assert_eq!(blockers(&wrong), vec![ReplayBoundaryBlocker::WrongRequest {
        step_position: first_step(&wrong),
        logical_position: 0,
    }]);
}

// r[verify molten.world_replay_boundary.ordering]
#[test]
fn reordered_events_report_the_earliest_divergence_and_arrival_never_orders() {
    let request = valid_request();
    let observations = &request.boundary.ledger.observations;
    assert!(
        observations[1].arrival_observation > observations[3].arrival_observation,
        "the fixture's arrivals run opposite to logical order"
    );
    let dispatched = replay_dispatch_order(observations)
        .into_iter()
        .map(|observation| observation.slot.operation_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(dispatched, ["read-1", "delivery-1", "send-1", "delivery-2"], "logical positions order dispatch");

    let mut reordered = valid_request();
    let second = second_step(&reordered);
    with_ledger(&mut reordered, |ledger| {
        let (send, delivery) = (ledger.observations[2].clone(), ledger.observations[3].clone());
        let mut moved_send = send.clone();
        moved_send.slot.logical_position = 1;
        let mut moved_delivery = delivery.clone();
        moved_delivery.slot.logical_position = 0;
        ledger.observations[2] = reidentify_observation(moved_delivery);
        ledger.observations[3] = reidentify_observation(moved_send);
    });
    assert_eq!(blockers(&reordered), vec![ReplayBoundaryBlocker::OrderingDivergence {
        step_position: second,
        logical_position: 0,
    }]);
}

// r[verify molten.world_replay_boundary.completeness]
#[test]
fn profile_adapter_protection_and_secrecy_drift_block_replay() {
    let mut profile = valid_request();
    with_inventory(&mut profile, |inventory| inventory.profile_ref = digest("another-profile"));
    assert_eq!(blockers(&profile), vec![ReplayBoundaryBlocker::ProfileDrift]);

    let mut adapter = valid_request();
    with_ledger(&mut adapter, |ledger| {
        ledger.observations[1].adapter_cohort_ref = digest("another-adapter");
        ledger.observations[1] = reidentify_observation(ledger.observations[1].clone());
    });
    let drifted = adapter.boundary.ledger.observations[1].observation_ref.clone();
    assert_eq!(blockers(&adapter), vec![ReplayBoundaryBlocker::AdapterDrift(drifted)]);

    let mut secret = valid_request();
    with_ledger(&mut secret, |ledger| {
        ledger.observations[0].protection = WorldReplayMemberProtection::Public;
        ledger.observations[0] = reidentify_observation(ledger.observations[0].clone());
    });
    let exposed = secret.boundary.ledger.observations[0].observation_ref.clone();
    assert_eq!(blockers(&secret), vec![ReplayBoundaryBlocker::PlaintextSecret(exposed)]);
}

// r[verify molten.world_replay_boundary.observations]
#[test]
fn a_tampered_observation_or_ledger_is_rejected() {
    let mut observation = valid_request();
    with_ledger(&mut observation, |ledger| {
        ledger.observations[1].outcome = ReplayObservationOutcome::Error {
            error_class: "timeout".to_string(),
        };
    });
    let tampered = observation.boundary.ledger.observations[1].observation_ref.clone();
    assert_eq!(blockers(&observation), vec![ReplayBoundaryBlocker::ObservationIdentityMismatch(tampered)]);

    let mut ledger = valid_request();
    ledger.boundary.ledger.slots[0].operation_id = "read-2".to_string();
    assert!(blockers(&ledger).contains(&ReplayBoundaryBlocker::LedgerIdentityMismatch));
}

// r[verify molten.world_replay_boundary.effect_sealing]
#[test]
fn an_external_mutation_cannot_be_declared_replayable() {
    let mut replayable = valid_request();
    with_inventory(&mut replayable, |inventory| {
        let send = inventory.sources.iter_mut().find(|source| source.source_id == SEND).expect("send source");
        send.handling = ReplaySourceHandling::Deterministic;
    });
    let found = blockers(&replayable);
    assert!(found.contains(&ReplayBoundaryBlocker::MutationNotSealed(SEND.to_string())), "{found:?}");
}

fn diagnostic(cohort: &str, claims: Vec<OpaqueDiagnosticClaim>) -> OpaqueReplayDiagnostic {
    OpaqueReplayDiagnostic {
        descriptor_ref: digest("native-process-descriptor"),
        byte_length: 4_096,
        cohort_ref: digest(cohort),
        claims,
        native_process: None,
    }
}

// r[verify molten.world_replay_boundary.opaque_native]
#[test]
fn an_opaque_diagnostic_stays_detached_and_satisfies_no_observation() {
    let mut detached = valid_request();
    detached.boundary.opaque_diagnostics = vec![diagnostic("chaoscontrol-native-cohort", vec![
        OpaqueDiagnosticClaim::DetachedDiagnostic,
    ])];
    let admission = admit(&detached).expect("detached diagnostic");
    assert_eq!(admission.opaque_diagnostic_refs, vec![digest("native-process-descriptor")]);
    let required = required_world_replay_boundary_closure(&detached.boundary);
    assert!(
        required
            .iter()
            .any(|requirement| requirement.role == WorldReplayCapsuleMemberRole::OpaqueDiagnostic)
    );

    let mut overclaim = detached.clone();
    overclaim.boundary.opaque_diagnostics[0].claims.push(OpaqueDiagnosticClaim::SatisfiesObservation);
    assert_eq!(blockers(&overclaim), vec![ReplayBoundaryBlocker::OpaqueOverclaim(digest(
        "native-process-descriptor"
    ))]);

    let mut foreign = detached.clone();
    foreign.boundary.opaque_diagnostics[0].cohort_ref = digest("another-cohort");
    assert_eq!(blockers(&foreign), vec![ReplayBoundaryBlocker::OpaqueCohortMismatch(digest(
        "native-process-descriptor"
    ))]);

    let mut replacement = detached;
    with_ledger(&mut replacement, |ledger| {
        ledger.observations.remove(0);
    });
    assert_eq!(blockers(&replacement), vec![ReplayBoundaryBlocker::MissingObservation {
        step_position: first_step(&replacement),
        logical_position: 0,
    }]);
}

// r[verify molten.world_replay_boundary.completeness]
#[test]
fn a_blocked_boundary_or_an_unbound_ledger_blocks_the_replay_plan() {
    let mut missing = valid_request();
    with_ledger(&mut missing, |ledger| {
        ledger.observations.remove(0);
    });
    let issues = plan_world_replay(&missing).expect_err("blocked boundary");
    assert!(issues.contains(&WorldReplayIssue::ObservationBoundary(ReplayBoundaryBlocker::MissingObservation {
        step_position: first_step(&missing),
        logical_position: 0,
    })));

    let mut unbound = valid_request();
    with_ledger(&mut unbound, |ledger| ledger.reached_sources.retain(|source| source != CLOCK));
    admit(&unbound).expect("the changed ledger is itself admissible");
    let issues = plan_world_replay(&unbound).expect_err("the capsule does not carry the new ledger");
    assert!(
        issues.iter().any(
            |issue| matches!(issue, WorldReplayIssue::MissingClosureRole { role, .. } if role == "observation-ledger")
        ),
        "{issues:?}"
    );
}
