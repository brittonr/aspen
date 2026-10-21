fn canonical_execution_workload() -> LiveWorkload {
    let request = crate::preserves_rail::content_ref_from_bytes(b"execution-request-a");
    LiveWorkload::new(vec![LiveWorkloadStep {
        sequence: 1,
        class: crate::fabric::FabricPortClass::DurableState,
        request_ref: request,
        permitted: vec![OutcomeClass::Delivered, OutcomeClass::Delayed],
        ordering_key: Some("node-a".to_string()),
    }])
    .expect("canonical execution workload")
}

// r[verify molten.fabric_simulation.live_differential]
// r[verify molten.fabric_simulation.differential_results]
#[test]
fn execution_differential_records_agreement_divergence_and_failure() {
    let workload = canonical_execution_workload();
    let simulation_side = crate::preserves_rail::content_ref_from_bytes(b"simulation-side");
    let live_side = crate::preserves_rail::content_ref_from_bytes(b"live-side");
    let build = |simulated: &[Option<OutcomeClass>], live: Option<&[Option<OutcomeClass>]>| {
        execution_differential(ExecutionDifferentialInput {
            workload: &workload,
            simulation_side_ref: &simulation_side,
            live_side_ref: &live_side,
            simulated,
            live,
        })
        .expect("canonical execution differential")
    };

    let delivered = [Some(OutcomeClass::Delivered)];
    let agreement = build(&delivered, Some(&delivered));
    assert_eq!(agreement.result_kind, "agreement");

    let delayed = [Some(OutcomeClass::Delayed)];
    let divergence = build(&delivered, Some(&delayed));
    assert_eq!(divergence.result_kind, "permitted-divergence");
    assert_ne!(divergence.report_ref, agreement.report_ref);

    let unpermitted = [Some(OutcomeClass::Backpressured)];
    assert_eq!(build(&delivered, Some(&unpermitted)).result_kind, "failure");

    let unavailable = build(&delivered, None);
    assert_eq!(unavailable.result_kind, "incomplete");
    assert_ne!(unavailable.report_ref, agreement.report_ref);
}

// r[verify molten.fabric_simulation.differential_evidence]
#[test]
fn differential_evidence_rejects_one_shared_identity() {
    let fixture = run_reference_fixture().expect("reference simulation");
    let mut execution = fixture.execution_differential.clone();
    execution.report_ref = fixture.differential.report_ref.clone();
    let denied = differential_evidence(DifferentialEvidenceInput {
        structural: &fixture.differential,
        execution: &execution,
    });
    assert!(denied.is_err(), "one shared identity cannot bind two artifacts");
    let evidence = differential_evidence(DifferentialEvidenceInput {
        structural: &fixture.differential,
        execution: &fixture.execution_differential,
    })
    .expect("separate differential evidence");
    assert_ne!(evidence.structural_ref, evidence.execution_ref);
    let text = crate::preserves_rail::to_text(&evidence.value).expect("evidence text");
    assert!(text.contains("agreement-does-not-prove-live-equivalence"));
    assert!(text.contains("incomplete-count"));
}
