fn id(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("consumer-control-id", vec![
        crate::preserves_rail::string(label),
    ]))
    .unwrap()
}

fn fixture() -> (super::ExpectedCohort, super::Bundle) {
    let expected = expected();
    let plan = chaoscontrol_smr::admit_profile(&expected.workload).unwrap();
    let observations = observations(&expected, &plan);
    let retained = FixtureHistory::new(&expected, &plan, &observations);
    let bundle = bundle(&expected, observations, retained);
    (expected, bundle)
}

fn bundle(expected: &super::ExpectedCohort, observations: Vec<super::super::wire::BoundObservation>,
    retained: FixtureHistory) -> super::Bundle
{
    let FixtureHistory { drains, cohort, history, observations_ref, stabilization, liveness,
        oracle, receipt, execution_ref, source_ref } = retained;
    let proposals = expected
        .operations
        .iter()
        .enumerate()
        .map(|(offset, o)| chaoscontrol_smr::ProposalAttempt {
            event_sequence: u64::try_from(offset).unwrap() + 1,
            logical_operation_id: o.operation_ref.clone(),
            operation_id: o.operation_ref.clone(),
            attempt: 0,
            command_ref: o.command_ref.clone(),
            outcome: chaoscontrol_smr::ProposalOutcome::Acknowledged,
        })
        .collect();
    super::Bundle {
        schema: super::BUNDLE_SCHEMA.into(),
        producer_revision: expected.producer_revision.clone(),
        evidence_class: expected.evidence_class,
        claim: expected.claim.clone(),
        molten: super::MoltenIdentity {
            source_ref: expected.molten_source_ref.clone(),
            build_ref: expected.molten_build_ref.clone(),
            dependency_closure_ref: expected.dependency_closure_ref.clone(),
            adapter_ref: expected.adapter_ref.clone(),
            observer_ref: expected.observer_ref.clone(),
        },
        sources: expected.sources.clone(),
        execution_ref,
        observations,
        drains,
        source_ref,
        cohort,
        history_ref: chaoscontrol_smr::history_report_ref(&history).unwrap(),
        observations_ref,
        history,
        proposals,
        stabilization,
        liveness,
        terminal_status: chaoscontrol_smr::TerminalStatus::Completed,
        faults: Vec::new(),
        fault_effects: Vec::new(),
        replay: None,
        oracle,
        receipt,
        non_claims: super::NON_CLAIMS.iter().map(|s| (*s).into()).collect(),
    }
}
