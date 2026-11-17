/// Build candidate summaries using the public producer functions, with the
/// expected consumer verdict supplied explicitly by each control, not by import.
fn retain(expected: &super::ExpectedCohort, b: &mut super::Bundle, verdict: chaoscontrol_smr::observation::OracleVerdict, diagnostics: &[&str]) {
    let plan = chaoscontrol_smr::admit_profile(&expected.workload).unwrap();
    let records: Vec<_> = b.observations.iter().map(|o| o.envelope.clone()).collect();
    let chains: Vec<_> = b.observations.iter().map(|o| o.apply.chain.clone()).collect();
    b.cohort = chaoscontrol_smr::observation::assemble_cohort(&expected.protocol, &expected.boundary_ref, &records, &b.drains).unwrap();
    b.history = chaoscontrol_smr::validate_history(&plan, &chains).unwrap();
    b.history_ref = chaoscontrol_smr::history_report_ref(&b.history).unwrap();
    b.observations_ref = chaoscontrol_smr::observations_ref(&chains).unwrap();
    b.liveness = chaoscontrol_smr::evaluate_liveness(&plan, b.stabilization.as_ref()).unwrap();
    b.oracle = chaoscontrol_smr::observation::OracleResult {
        schema: chaoscontrol_smr::observation::ORACLE_RESULT_SCHEMA.into(),
        oracle_ref: expected.protocol.independent_oracle_ref.clone(),
        cohort_ref: b.cohort.cohort_ref.clone(),
        verdict,
        diagnostics: diagnostics.iter().map(|s| (*s).into()).collect(),
    };
    let replay_refs = b.replay.as_ref().map_or_else(Vec::new, |r| {
        vec![
            super::artifact_ref("molten-chaoscontrol-semantic-original-v1", &r.original).unwrap(),
            super::artifact_ref("molten-chaoscontrol-semantic-repeat-v1", &r.repeat).unwrap(),
            super::artifact_ref("molten-chaoscontrol-repeat-bindings-v1", &r.repeat_observations).unwrap(),
            chaoscontrol_smr::replay_comparison_ref(&r.comparison).unwrap(),
        ]
    });
    let mut fault_refs = b
        .faults
        .iter()
        .map(|f| super::artifact_ref("molten-chaoscontrol-fault-outcome-v1", f).unwrap())
        .collect::<Vec<_>>();
    fault_refs.sort();
    b.receipt = chaoscontrol_smr::observation::ProtocolReceipt::build(
        &expected.protocol,
        &b.cohort,
        &b.oracle,
        None,
        vec![expected.scheduler_ref.clone()],
        fault_refs,
        replay_refs,
    )
    .unwrap();
}

fn attach_replay(expected: &super::ExpectedCohort, b: &mut super::Bundle, is_mismatch: bool) {
    let original = chaoscontrol_smr::SemanticRun {
        operation_ids: expected.operations.iter().map(|o| o.operation_ref.clone()).collect(),
        proposal_outcomes: b.proposals.clone(),
        observations: b.observations.iter().map(|o| o.apply.chain.clone()).collect(),
        safety_prefixes: b.history.safety_prefixes.clone(),
        stabilization_facts: b.stabilization.clone(),
        liveness: b.liveness.clone(),
        terminal_status: b.terminal_status,
    };
    let mut repeat = original.clone();
    if is_mismatch {
        repeat.terminal_status = chaoscontrol_smr::TerminalStatus::Failed;
    }
    let comparison = chaoscontrol_smr::compare_replay_bounded(&original, &repeat, &expected.workload.bounds).unwrap();
    b.replay = Some(super::ReplayEvidence {
        original,
        repeat,
        repeat_observations: b.observations.clone(),
        comparison,
    });
    if is_mismatch {
        retain(expected, b, chaoscontrol_smr::observation::OracleVerdict::NotEvaluated, &["semantic-replay-mismatch"]);
    } else {
        retain(expected, b, chaoscontrol_smr::observation::OracleVerdict::Pass, &[]);
    }
}