// r[verify molten.fabric_simulation.stateful_storage]
// r[verify molten.fabric_simulation.stateful_transport]
#[test]
fn delayed_completion_changes_whether_an_acknowledged_write_survives_a_crash() {
    let durable_manifest = causal_acknowledgment_manifest(false).expect("causal acknowledgment manifest");
    let delayed_manifest = causal_acknowledgment_manifest(true).expect("delayed causal acknowledgment manifest");
    let durable = run_reference_world(&durable_manifest, DEFAULT_REFERENCE_SEED)
        .expect("acknowledgment survives when the completion is not delayed");
    let delayed = run_reference_world(&delayed_manifest, DEFAULT_REFERENCE_SEED)
        .expect("delayed completion holds the acknowledged write");

    for fixture in [&durable, &delayed] {
        assert_eq!(fixture.observations.len(), EXPECTED_CAUSAL_WORKLOAD_STEPS);
        assert_eq!(fixture.observations[0].observation.semantic_invariants_passed, vec![
            "transaction-version-monotonic".to_string(),
            "conflict-does-not-mutate".to_string()
        ]);
    }
    assert_eq!(
        durable.run.summary.choice_records[0].semantic_output_ref,
        delayed.run.summary.choice_records[0].semantic_output_ref,
        "the acknowledgment itself is identical across fault presence"
    );
    assert_eq!(durable.crash_recoveries.len(), 1);
    assert_eq!(delayed.crash_recoveries.len(), 1);
    assert!(durable.crash_recoveries[0].lost_operations.is_empty());
    assert_eq!(durable.crash_recoveries[0].durable_entry_request_refs.len(), 1);
    let lost = &delayed.crash_recoveries[0].lost_operations;
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0].phase, crate::core_api::world_faults::FaultPhase::AfterPossibleSubmit);
    assert_eq!(
        crate::core_api::world_faults::expected_recovery_for_phase(lost[0].phase),
        crate::core_api::world_faults::RecoveryClass::Uncertain
    );
    assert!(delayed.crash_recoveries[0].durable_entry_request_refs.is_empty());
    let durable_state = durable
        .service_states
        .get("node-transactional-key-value")
        .and_then(|state| match state {
            ReferenceServiceState::TransactionalKeyValue(inner) => Some(inner.clone()),
            _ => None,
        })
        .expect("durable run recovered its transactional service");
    let delayed_state = delayed
        .service_states
        .get("node-transactional-key-value")
        .and_then(|state| match state {
            ReferenceServiceState::TransactionalKeyValue(inner) => Some(inner.clone()),
            _ => None,
        })
        .expect("delayed run recovered its transactional service");
    assert!(durable_state.values.contains_key("key-a"));
    assert_eq!(durable_state.version, 1);
    assert!(delayed_state.values.is_empty());
    assert_eq!(delayed_state.version, 0);
    assert_ne!(durable.run.run_ref, delayed.run.run_ref);
}

// r[verify molten.fabric_simulation.causal_exploration]
#[test]
fn different_seeds_explore_different_schedules_and_repeat_deterministically() {
    let manifest = reference_world_manifest().expect("reference world manifest");
    let first = run_reference_world(&manifest, DEFAULT_REFERENCE_SEED).expect("first seed run");
    let repeated = run_reference_world(&manifest, DEFAULT_REFERENCE_SEED).expect("first seed rerun");
    let other = run_reference_world(&manifest, DEFAULT_REFERENCE_SEED + 1).expect("second seed run");

    let first_ids = first
        .run
        .summary
        .choice_records
        .iter()
        .map(|record| record.selected.choice_id.clone())
        .collect::<Vec<_>>();
    let other_ids = other
        .run
        .summary
        .choice_records
        .iter()
        .map(|record| record.selected.choice_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(first.run.run_ref, repeated.run.run_ref);
    assert_eq!(
        first_ids,
        repeated
            .run
            .summary
            .choice_records
            .iter()
            .map(|record| record.selected.choice_id.clone())
            .collect::<Vec<_>>()
    );
    assert_ne!(first_ids, other_ids);
    assert_eq!(first_ids.len(), EXPECTED_CHOICE_RECORDS);
    assert!(
        first
            .run
            .summary
            .choice_records
            .iter()
            .all(|record| record.semantic_output_ref.starts_with("blake3:"))
    );
}

// r[verify molten.fabric_simulation.causal_exploration]
#[test]
fn replay_detects_a_changed_semantic_output_at_the_first_diverging_record() {
    let fixture = run_reference_simulation_fixture().expect("reference simulation");
    let mut diverged = fixture.run.summary.choice_records.clone();
    diverged[1].semantic_output_ref = blake3_ref(b"replay-tampered-semantic-output");

    let comparison = compare_replay(&fixture.run.summary.choice_records, &diverged);

    assert!(!comparison.matches);
    let divergence = comparison.first_divergence.expect("first mismatch is reported");
    assert_eq!(divergence.position, 1);
    assert!(divergence.diagnostic.contains("semantic-output-ref"));
    assert_eq!(divergence.expected_choice_id, fixture.run.summary.choice_records[1].selected.choice_id);
}

// r[verify molten.fabric_simulation.causal_exploration]
#[test]
fn shrink_reruns_each_candidate_and_keeps_only_reproducing_failures() {
    let fixture = run_reference_shrink_fixture().expect("reference shrink");

    assert!(fixture.shrink.result.failure_preserved);
    assert!(fixture.shrink.result.attempts > 0);
    assert!(fixture.shrink.result.removed_workload_steps > 0);
    let has_retained_kv = fixture
        .shrunk_world
        .admitted
        .manifest
        .workload
        .iter()
        .any(|step| step.node_id == "node-transactional-key-value");
    assert!(has_retained_kv, "the minimized case still reproduces the failing service observation");
    assert!(fixture.shrink.shrink_ref.starts_with("blake3:"));
    let rerun = run_reference_world(&fixture.shrunk_world.admitted.manifest, DEFAULT_REFERENCE_SEED)
        .expect("minimized world reruns");
    assert_eq!(rerun.run.summary.decision, SimulationDecision::InvariantFailed);
    let fingerprint = failure_fingerprint(&rerun.run.summary).expect("minimized failure fingerprints");
    assert!(
        fingerprint
            .failed_invariants
            .contains(&"extension:transactional-key-value:reference-fixture-failing-invariant".to_string())
    );
}
