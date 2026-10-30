//! Shared in-memory admission fixtures: one complete valid run that tests mutate per regression.

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn profile() -> crate::live_reliability::profile::Register {
    serde_json::from_value(serde_json::json!({
        "schema":"molten.live-register-profile.v1", "group":"group", "service":"registry", "keys":["key"],
        "initial_value":null,"values":[1,2],"generator_ref":reference("generator"),"adapter_ref":reference("adapter"),
        "model_ref":reference("model"),"outcome_ref":reference("outcome"),"seed":7,"read_weight":1,"write_weight":1,
        "clients":["client"],"endpoints":["endpoint-a","endpoint-b"],"concurrency":1,"operations":5,"control_operations":1,
        "consistency_model":"register","recovery":{"timeout_ms":1000,"stable_membership_observations":1,
        "require_all_endpoint_reads":true},"evidence":{"max_attempts_per_operation":1,"max_events":32,"max_uncertain":1,
        "max_bytes":1048576,"checker_states":100,"checker_branches":100,"checker_memo_bytes":4096,"checker_reductions":10}
    })).expect("profile")
}

pub(super) fn fixture() -> (super::super::Expected, super::super::Observed) {
    let admitted = crate::live_reliability::profile::admit(&profile()).expect("admitted");
    let plan = crate::live_reliability::generator::generate(&admitted).expect("admitted plan");
    let mut identities: std::collections::BTreeMap<_, _> =
        super::super::IDENTITIES.iter().map(|k| ((*k).to_owned(), reference(k))).collect();
    identities.insert("profile".into(), admitted.profile_ref().into());
    identities.insert("plan".into(), crate::live_reliability::profile::canonical_ref(&plan).expect("plan ref"));
    let (history, recovery) = history(&admitted, &plan, &identities["source"]);
    let history_id = crate::live_reliability::checker::semantic_history_id(
        &serde_json::from_value(history.clone()).expect("typed history"),
    )
    .expect("producer identity");
    let report = serde_json::json!({"schema_version":1,"history_id":history_id,
        "checker_id":"chaoscontrol-semantic-checker.v1","model":"register","profile_id":admitted.profile_ref(),
        "bounds":history["profile"]["bounds"],"completeness":history["completeness"],"verdict":"valid",
        "witness":[],"invalid":null,"unknown":null,"reference_status":"not_run"});
    let (expected_readiness, readiness): (std::collections::BTreeMap<_, _>, std::collections::BTreeMap<_, _>) =
        admitted
            .profile()
            .endpoints
            .iter()
            .map(|endpoint| {
                let (expected, proof) = crate::live_reliability::evidenceproof::tests::fixture(endpoint);
                ((endpoint.clone(), expected), (endpoint.clone(), proof))
            })
            .unzip();
    let evidence = super::super::Observed {
        identities: identities.clone(),
        profile_ref: admitted.profile_ref().into(),
        plan_ref: identities["plan"].clone(),
        history,
        report,
        reference_report: None,
        faults: vec![],
        recovery,
        teardown: true,
        readiness,
        non_claims: super::super::NON_CLAIMS.iter().map(|v| (*v).into()).collect(),
    };
    let checked = checked(&evidence);
    let expected = super::super::Expected {
        admitted,
        plan,
        identities,
        endpoints: profile().endpoints.into_iter().collect(),
        max_events: 32,
        max_witness_bytes: 1048576,
        max_recovery_ms: 1000,
        checked,
        reference_checked: None,
        readiness: expected_readiness,
    };
    (expected, evidence)
}

fn history(
    admitted: &crate::live_reliability::profile::Admitted,
    plan: &crate::live_reliability::generator::OperationPlan,
    source: &str,
) -> (serde_json::Value, Vec<super::super::FinalRead>) {
    let mut events = vec![
        serde_json::json!({"kind":"lifecycle","event_index":0,"phase":"ready","controller_time_ns":0}),
        serde_json::json!({"kind":"lifecycle","event_index":1,"phase":"stable-membership","controller_time_ns":0}),
    ];
    let mut recovery = Vec::with_capacity(admitted.profile().endpoints.len());
    let mut current = crate::live_reliability::generator::Value::Null;
    for (index, operation) in plan.operations.iter().enumerate() {
        let is_write = operation.function == crate::live_reliability::generator::Function::Write;
        let outcome =
            crate::live_reliability::history::PublicOutcome::Acknowledged(if is_write { None } else { Some(current) });
        let observation = crate::live_reliability::history::AttemptObservation {
            ordinal: 0,
            event_index: events.len() as u64,
            started_ns: events.len() as u64,
            completed_ns: events.len() as u64 + 1,
            source_artifact: source.into(),
            outcome,
        };
        for event in
            crate::live_reliability::history::project_attempt(admitted, operation, &observation).expect("project")
        {
            events.push(serde_json::to_value(event).expect("event"));
        }
        if is_write {
            current = operation.value;
        }
        if index >= plan.operations.len() - admitted.profile().endpoints.len() {
            recovery.push(super::super::FinalRead {
                endpoint: operation.endpoint.clone(),
                attempt_id: crate::live_reliability::history::attempt_id(operation, 0),
                value: serde_json::to_value(current).expect("value"),
                ready: true,
                ready_event: 0,
                stable_membership_event: 1,
                stable_membership_observations: 1,
                stable_membership: true,
                elapsed_ms: 0,
            });
        }
    }
    events.push(serde_json::json!({"kind":"lifecycle","phase":"teardown",
        "event_index":events.len(),"controller_time_ns":events.len()}));
    let p = admitted.profile();
    (
        serde_json::json!({"schema_version":2,"workload_profile":admitted.profile_ref(),"source_artifact":source,
        "profile":{"profile_id":admitted.profile_ref(),"model":"register","independent_key_decomposition":false,
        "bounds":{"max_operations":p.operations,"max_depth":p.operations,"max_states":p.evidence.checker_states,
        "max_branches":p.evidence.checker_branches,"max_memo_bytes":p.evidence.checker_memo_bytes,
        "max_reductions":p.evidence.checker_reductions}},"completeness":{"invocations":p.operations,"completions":p.operations,
        "pending":0,"applied_faults":0,"observed_faults":0},"events":events}),
        recovery,
    )
}

pub(super) fn checked(evidence: &super::super::Observed) -> crate::live_reliability::checker::CheckedReport {
    crate::live_reliability::checker::CheckedReport {
        history_ref: crate::live_reliability::checker::canonical_ref(&evidence.history).expect("history ref"),
        report_ref: crate::live_reliability::checker::canonical_ref(&evidence.report).expect("report ref"),
        checker_ref: evidence.identities["checker"].clone(),
        report: evidence.report.clone(),
    }
}
