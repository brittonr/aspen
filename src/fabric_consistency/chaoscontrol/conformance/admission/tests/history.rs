struct FixtureHistory {
    drains: Vec<chaoscontrol_smr::observation::Drain>,
    cohort: chaoscontrol_smr::observation::CohortResult,
    history: chaoscontrol_smr::HistoryReport,
    observations_ref: String,
    stabilization: Option<chaoscontrol_smr::StabilizationFacts>,
    liveness: chaoscontrol_smr::LivenessReport,
    oracle: chaoscontrol_smr::observation::OracleResult,
    receipt: chaoscontrol_smr::observation::ProtocolReceipt,
    execution_ref: String,
    source_ref: String,
}

impl FixtureHistory {
    fn new(expected: &super::ExpectedCohort, plan: &chaoscontrol_smr::SmrWorkloadPlan,
        observations: &[super::super::wire::BoundObservation]) -> Self
    {
        let records: Vec<_> = observations.iter().map(|o| o.envelope.clone()).collect();
        let chains: Vec<_> = observations.iter().map(|o| o.apply.chain.clone()).collect();
        let drains = expected
            .protocol
            .participants
            .iter()
            .map(|p| chaoscontrol_smr::observation::Drain {
                participant_ref: p.participant_ref.clone(),
                producer_ref: p.producer_ref.clone(),
                generation: p.generation,
                last_sequence: 2,
                dropped: 0,
                final_drain: true,
            })
            .collect::<Vec<_>>();
        let cohort = chaoscontrol_smr::observation::assemble_cohort(&expected.protocol, &expected.boundary_ref, &records, &drains).unwrap();
        let history = chaoscontrol_smr::validate_history(plan, &chains).unwrap();
        let stabilization = Some(stabilization(expected));
        let liveness = chaoscontrol_smr::evaluate_liveness(plan, stabilization.as_ref()).unwrap();
        let oracle = initial_oracle(expected, &cohort);
        let receipt = chaoscontrol_smr::observation::ProtocolReceipt::build(
            &expected.protocol,
            &cohort,
            &oracle,
            None,
            vec![expected.scheduler_ref.clone()],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let execution_ref = id("diagnostic-retained-input");
        let source_ref = chaoscontrol_smr::observation::guest::validate_sources(&expected.protocol, &super::Intake {
            sources: &expected.sources,
            execution_ref: &execution_ref,
            records: &records,
            drains: &drains,
        })
        .unwrap();
        Self { drains, cohort, history, observations_ref: chaoscontrol_smr::observations_ref(&chains).unwrap(),
            stabilization, liveness, oracle, receipt, execution_ref, source_ref }
    }
}

fn stabilization(expected: &super::ExpectedCohort) -> chaoscontrol_smr::StabilizationFacts {
    chaoscontrol_smr::StabilizationFacts {
        profile_id: expected.workload.liveness.profile_id.clone(),
        quorum_available: true,
        lifecycle_ready: true,
        disruptive_faults_active: false,
        virtual_progress_start: 0,
        virtual_progress_end: 10,
        committed_before: 0,
        committed_after: 2,
    }
}

fn initial_oracle(expected: &super::ExpectedCohort, cohort: &chaoscontrol_smr::observation::CohortResult)
    -> chaoscontrol_smr::observation::OracleResult
{
    chaoscontrol_smr::observation::OracleResult {
        schema: chaoscontrol_smr::observation::ORACLE_RESULT_SCHEMA.into(),
        oracle_ref: expected.protocol.independent_oracle_ref.clone(),
        cohort_ref: cohort.cohort_ref.clone(),
        verdict: chaoscontrol_smr::observation::OracleVerdict::Pass,
        diagnostics: Vec::new(),
    }
}
