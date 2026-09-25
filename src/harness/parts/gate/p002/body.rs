
struct EvidenceRefs {
    executor_preflights_ref: String,
    executor_execution_receipts_ref: String,
    runtime_predicate_receipts_ref: String,
    policy_ref: String,
    policy_gate_ref: String,
    policy_nickel_source_ref: String,
    policy_nickel_export_ref: String,
    policy_basalt_preflight_ref: String,
    budget_ref: String,
    budget_gate_ref: String,
    budget_nickel_source_ref: String,
    budget_nickel_export_ref: String,
    budget_basalt_preflight_ref: String,
    capability_ref: String,
    capability_gate_ref: String,
    capability_authority_preflight_ref: String,
    capability_proofset_ref: String,
    capability_ucan_verification_receipts_ref: String,
    capability_derived_grants_ref: String,
    capability_authority_receipts_ref: String,
    capability_request_refs_ref: String,
}

struct AuthorityAggregateRefs {
    ucan_verification_receipts_ref: String,
    derived_grants_ref: String,
    authority_receipts_ref: String,
    request_refs_ref: String,
}

fn check_report(value: &IoValue, artifact_kind: String, artifact_ref: Option<String>) -> Result<Check> {
    let validation = super::replay::validate_report_value(value)?;
    let replay = super::replay::replay_report_value(value)?;
    let report = super::schema::parse_report(value)?;
    if validation.report_ref != replay.expected_report_ref || validation.report_ref != replay.actual_report_ref {
        return Err(MoltenError::invalid_harness("gate replay report refs do not match validation report ref"));
    }
    if validation.final_state_hash != replay.final_state_hash {
        return Err(MoltenError::invalid_harness("gate replay final state does not match validation final state"));
    }
    let refs = evidence_refs(&report)?;
    let deterministic_replay_verify_value =
        harness_replay_verify_value(&replay.expected_report_ref, &replay.actual_report_ref, &replay.final_state_hash);
    let deterministic_replay_verify_ref = canonical_hash(&deterministic_replay_verify_value)?;
    let chain_evidence = build_gate_chain_evidence(
        &validation.report_ref,
        &validation.suite_ref,
        &report.final_state_hash,
        &report.profile,
    )?;
    let turn_journals = build_turn_journals(&report)?;
    Ok(Check {
        artifact_kind,
        artifact_ref: artifact_ref.unwrap_or_else(|| validation.report_ref.clone()),
        report_ref: validation.report_ref,
        suite_ref: validation.suite_ref,
        initial_state_hash: report.initial_state_hash,
        final_state_hash: report.final_state_hash,
        replay_actual_report_ref: replay.actual_report_ref,
        deterministic_replay_verify_ref,
        deterministic_replay_verify_value,
        executor_preflights_ref: refs.executor_preflights_ref,
        executor_execution_receipts_ref: refs.executor_execution_receipts_ref,
        runtime_predicate_receipts_ref: refs.runtime_predicate_receipts_ref,
        policy_ref: refs.policy_ref,
        policy_gate_ref: refs.policy_gate_ref,
        policy_nickel_source_ref: refs.policy_nickel_source_ref,
        policy_nickel_export_ref: refs.policy_nickel_export_ref,
        policy_basalt_preflight_ref: refs.policy_basalt_preflight_ref,
        budget_ref: refs.budget_ref,
        budget_gate_ref: refs.budget_gate_ref,
        budget_nickel_source_ref: refs.budget_nickel_source_ref,
        budget_nickel_export_ref: refs.budget_nickel_export_ref,
        budget_basalt_preflight_ref: refs.budget_basalt_preflight_ref,
        capability_ref: refs.capability_ref,
        capability_gate_ref: refs.capability_gate_ref,
        capability_authority_preflight_ref: refs.capability_authority_preflight_ref,
        capability_proofset_ref: refs.capability_proofset_ref,
        capability_ucan_verification_receipts_ref: refs.capability_ucan_verification_receipts_ref,
        capability_derived_grants_ref: refs.capability_derived_grants_ref,
        capability_authority_receipts_ref: refs.capability_authority_receipts_ref,
        capability_request_refs_ref: refs.capability_request_refs_ref,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        observations: validation.observations as u64,
        actors: report.actors,
        budget: report.budget,
        chain_evidence,
        turn_journals,
    })
}

fn evidence_refs(report: &super::schema::Report) -> Result<EvidenceRefs> {
    let policy = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing policy gate evidence"))?;
    let budget = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing budget gate evidence"))?;
    let capability = report
        .capability_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing capability gate evidence"))?;
    let preflights = report
        .executor_preflights
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing executor preflight evidence"))?;
    let authority_refs = authority_aggregate_refs(&report.observations)?;
    Ok(EvidenceRefs {
        executor_preflights_ref: canonical_hash(&preflights.value)?,
        executor_execution_receipts_ref: executor_execution_receipts_ref(&report.observations)?,
        runtime_predicate_receipts_ref: runtime_predicate_receipts_ref(&report.observations)?,
        policy_ref: policy.policy_ref.clone(),
        policy_gate_ref: canonical_hash(&policy.value)?,
        policy_nickel_source_ref: policy.nickel_source_ref.clone(),
        policy_nickel_export_ref: policy.nickel_export_ref.clone(),
        policy_basalt_preflight_ref: policy.basalt_preflight_ref.clone(),
        budget_ref: budget.budget_ref.clone(),
        budget_gate_ref: canonical_hash(&budget.value)?,
        budget_nickel_source_ref: budget.nickel_source_ref.clone(),
        budget_nickel_export_ref: budget.nickel_export_ref.clone(),
        budget_basalt_preflight_ref: budget.basalt_preflight_ref.clone(),
        capability_ref: capability.capability_ref.clone(),
        capability_gate_ref: canonical_hash(&capability.value)?,
        capability_authority_preflight_ref: capability.authority_preflight_ref.clone(),
        capability_proofset_ref: capability.proofset_ref.clone(),
        capability_ucan_verification_receipts_ref: authority_refs.ucan_verification_receipts_ref,
        capability_derived_grants_ref: authority_refs.derived_grants_ref,
        capability_authority_receipts_ref: authority_refs.authority_receipts_ref,
        capability_request_refs_ref: authority_refs.request_refs_ref,
    })
}

fn authority_aggregate_refs(observations: &[super::schema::Observation]) -> Result<AuthorityAggregateRefs> {
    let authorities = observations
        .iter()
        .flat_map(|observation| observation.events.iter())
        .filter(|event| super::schema::event_boundary(event) == super::schema::EventBoundary::PolicyDecision)
        .filter_map(|event| {
            super::schema::parse_admission_decision_event(event).map(|admission| admission.authority).transpose()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(AuthorityAggregateRefs {
        ucan_verification_receipts_ref: canonical_hash(&record(
            "ucan-verification-receipts",
            vec![sequence(
                authorities
                    .iter()
                    .flat_map(|authority| authority.ucan_verification_receipt_refs.as_slice())
                    .map(string)
                    .collect(),
            )],
        ))?,
        derived_grants_ref: canonical_hash(&record(
            "derived-grants",
            vec![sequence(
                authorities.iter().flat_map(|authority| authority.derived_grant_refs.as_slice()).map(string).collect(),
            )],
        ))?,
        authority_receipts_ref: canonical_hash(&record(
            "basalt-enforcement-receipts",
            vec![sequence(
                authorities.iter().map(|authority| string(&authority.basalt_enforcement_receipt_ref)).collect(),
            )],
        ))?,
        request_refs_ref: canonical_hash(&record(
            "authority-requests",
            vec![sequence(authorities.iter().map(|authority| string(&authority.request_ref)).collect())],
        ))?,
    })
}

fn executor_execution_receipts_ref(observations: &[super::schema::Observation]) -> Result<String> {
    let receipts = observations
        .iter()
        .flat_map(|observation| observation.events.iter())
        .filter(|event| {
            matches!(
                super::schema::event_boundary(event),
                super::schema::EventBoundary::SteelExecution | super::schema::EventBoundary::WasmExecution
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    canonical_hash(&record("executor-execution-receipts", vec![sequence(receipts)]))
}

fn runtime_predicate_receipts_ref(observations: &[super::schema::Observation]) -> Result<String> {
    let receipts = observations
        .iter()
        .flat_map(|observation| observation.events.iter())
        .filter(|event| super::schema::event_boundary(event) == super::schema::EventBoundary::RuntimePredicate)
        .cloned()
        .collect::<Vec<_>>();
    canonical_hash(&record("runtime-predicate-receipts", vec![sequence(receipts)]))
}

fn tool_value() -> IoValue {
    record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))])
}

fn artifact_refs_value(refs: &[(&str, &str)]) -> IoValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(*kind), string(*artifact_ref)]))
            .collect(),
    )])
}

fn validation_value(check: &Check) -> IoValue {
    record("validation", vec![
        record("status", vec![string("pass")]),
        record("report", vec![string(&check.report_ref)]),
        record("suite", vec![string(&check.suite_ref)]),
        record("final-state", vec![string(&check.final_state_hash)]),
        record("observations", vec![u64_value(check.observations)]),
        super::schema::actor_registry_value(&check.actors),
        super::schema::budget_value(&check.budget.limits, &check.budget.usage),
    ])
}

fn harness_replay_verify_value(expected_report_ref: &str, actual_report_ref: &str, final_state_hash: &str) -> IoValue {
    record("deterministic-replay-verify-v1", vec![
        string(DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        string("pass"),
        record("expected-report-ref", vec![string(expected_report_ref)]),
        record("actual-report-ref", vec![string(actual_report_ref)]),
        record("final-state-ref", vec![string(final_state_hash)]),
        record("divergence", vec![string("none")]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("report-replayed"), string("pass")]),
            record("check", vec![string("final-state-bound"), string("pass")]),
            record("check", vec![string("no-divergence"), string("pass")]),
        ])]),
    ])
}

fn replay_value(check: &Check) -> IoValue {
    record("replay", vec![
        record("status", vec![string("pass")]),
        record("expected-report", vec![string(&check.report_ref)]),
        record("actual-report", vec![string(&check.replay_actual_report_ref)]),
        record("final-state", vec![string(&check.final_state_hash)]),
        record("verify-ref", vec![string(&check.deterministic_replay_verify_ref)]),
        check.deterministic_replay_verify_value.clone(),
    ])
}

struct PassLink {
    chain: crate::evidence_chain::ChainScope,
    producer: crate::evidence_chain::ChainProducer,
    link_ref: String,
    link_value: IoValue,
    payload_refs: Vec<String>,
    subject_refs: Vec<String>,
    context_refs: Vec<String>,
}

struct PassPredicates {
    values: Vec<IoValue>,
    refs: Vec<String>,
    range_ref: String,
}

struct PassArtifacts {
    anchor_ref: String,
    anchor_value: IoValue,
    verify_ref: String,
    verify_value: IoValue,
    checkpoint_ref: String,
    checkpoint_value: IoValue,
}

struct Pred<'a> {
    predicate: &'a str,
    subject_refs: &'a [String],
    input_refs: &'a [String],
    context_refs: &'a [String],
    checks: &'a [crate::evidence_chain::ChainCheck],
}
