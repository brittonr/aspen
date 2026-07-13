
pub fn parse_coordination_state_snapshot(value: &IoValue) -> Result<CoordinationStateSnapshot> {
    let fields = simple_record(value, "coordination-state-snapshot-v1", 11)?;
    require_schema(&fields[0], COORDINATION_STATE_SNAPSHOT_SCHEMA, "coordination state schema")?;
    let retention_refs = record_ref_sequence(&fields[9], "retention")?;
    require_check(&parse_checks(&fields[10])?, "deterministic-state-order", "coordination state")?;
    Ok(CoordinationStateSnapshot {
        state_ref: canonical_hash(value)?,
        retention_refs,
        value: value.clone(),
    })
}

// r[impl molten.coordination.read_consistency_modes]
pub fn coordination_status_assertion_value(input: StatusAssertionInput<'_>) -> Result<IoValue> {
    validate_service(input.service)?;
    validate_key(input.key)?;
    validate_read_consistency_mode(input.read_consistency_mode)?;
    validate_ref(input.state_ref, "coordination assertion state ref")?;
    validate_ref(input.receipt_ref, "coordination assertion receipt ref")?;
    Ok(record("coordination-status-assertion-v1", vec![
        string(COORDINATION_STATUS_ASSERTION_SCHEMA),
        record("service", vec![string(input.service)]),
        record("key", vec![string(input.key)]),
        record("read-consistency", vec![string(input.read_consistency_mode)]),
        record("fact", vec![input.fact.clone()]),
        record("state", vec![string(input.state_ref)]),
        record("receipt", vec![string(input.receipt_ref)]),
        checks_value(&[
            ("dataspace-observation-only", "pass"),
            ("read-consistency-declared", "pass"),
            ("committed-state-bound", if input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE { "pass" } else { "diagnostic" }),
        ]),
    ]))
}

pub fn parse_coordination_status_assertion(value: &IoValue) -> Result<CoordinationStatusAssertion> {
    let fields = simple_record(value, "coordination-status-assertion-v1", 8)?;
    require_schema(&fields[0], COORDINATION_STATUS_ASSERTION_SCHEMA, "coordination assertion schema")?;
    let service = record_string(&fields[1], "service")?;
    let key = record_string(&fields[2], "key")?;
    let read_consistency_mode = record_string(&fields[3], "read-consistency")?;
    let state_ref = record_ref(&fields[5], "state")?;
    let receipt_ref = record_ref(&fields[6], "receipt")?;
    validate_read_consistency_mode(&read_consistency_mode)?;
    require_check(&parse_checks(&fields[7])?, "dataspace-observation-only", "coordination assertion")?;
    Ok(CoordinationStatusAssertion {
        assertion_ref: canonical_hash(value)?,
        service,
        key,
        read_consistency_mode,
        state_ref,
        receipt_ref,
        value: value.clone(),
    })
}

// r[impl molten.coordination.read_consistency_modes]
// r[impl molten.coordination_state_machine_proof.transition_receipt_binding]
pub fn coordination_receipt_value(input: ReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_service(input.service)?;
    validate_operation(input.service, input.operation)?;
    validate_read_consistency_mode(input.read_consistency_mode)?;
    validate_ref(input.request_ref, "coordination receipt request ref")?;
    if let Some(value) = input.raft_receipt_ref {
        validate_ref(value, "coordination receipt raft ref")?;
    }
    if let Some(value) = input.token_ref {
        validate_ref(value, "coordination receipt token ref")?;
    }
    validate_ref(input.state_ref, "coordination receipt state ref")?;
    validate_receipt_transition(input.decision, input.state_ref, input.transition)?;
    validate_refs(input.dataspace_assertion_refs, "coordination receipt assertion ref")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_COORDINATION_DIAGNOSTICS, "coordination receipt diagnostics")?;
    Ok(record("coordination-receipt-v1", vec![
        string(COORDINATION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("service", vec![string(input.service)]),
        record("operation", vec![string(input.operation)]),
        record("read-consistency", vec![string(input.read_consistency_mode)]),
        record("request", vec![string(input.request_ref)]),
        record("raft", vec![optional_ref_value(input.raft_receipt_ref)]),
        record("token", vec![optional_ref_value(input.token_ref)]),
        record("state", vec![string(input.state_ref)]),
        coordination_transition_value(input.transition)?,
        record("dataspace", vec![strings_sequence(input.dataspace_assertion_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(input.checks),
    ]))
}

pub fn parse_coordination_receipt(value: &IoValue) -> Result<CoordinationReceipt> {
    let fields = simple_record(value, "coordination-receipt-v1", COORDINATION_RECEIPT_FIELD_COUNT)?;
    require_schema(&fields[0], COORDINATION_RECEIPT_SCHEMA, "coordination receipt schema")?;
    let decision = record_string(&fields[1], "decision")?;
    let service = record_string(&fields[2], "service")?;
    let operation = record_string(&fields[3], "operation")?;
    let read_consistency_mode = record_string(&fields[4], "read-consistency")?;
    let request_ref = record_ref(&fields[5], "request")?;
    let raft_receipt_ref = record_optional_ref(&fields[6], "raft")?;
    let token_ref = record_optional_ref(&fields[7], "token")?;
    let state_ref = record_ref(&fields[8], "state")?;
    let transition = parse_coordination_transition(&fields[9])?;
    let dataspace_assertion_refs = record_ref_sequence(&fields[10], "dataspace")?;
    let diagnostics = record_string_sequence(&fields[11], "diagnostics")?;
    validate_read_consistency_mode(&read_consistency_mode)?;
    validate_receipt_transition(
        &decision,
        &state_ref,
        ReceiptTransitionInput {
            kind: &transition.kind,
            before_state_ref: &transition.before_state_ref,
            after_state_ref: transition.after_state_ref.as_deref(),
            preserved_state_ref: transition.preserved_state_ref.as_deref(),
            output_refs: &transition.output_refs,
            control_plane_intent_ref: transition.control_plane_intent_ref.as_deref(),
            prior_receipt_ref: transition.prior_receipt_ref.as_deref(),
        },
    )?;
    require_check(&parse_checks(&fields[12])?, "coordination-request-bound", "coordination receipt")?;
    Ok(CoordinationReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        service,
        operation,
        read_consistency_mode,
        request_ref,
        raft_receipt_ref,
        token_ref,
        state_ref,
        transition_kind: transition.kind,
        before_state_ref: transition.before_state_ref,
        after_state_ref: transition.after_state_ref,
        preserved_state_ref: transition.preserved_state_ref,
        output_refs: transition.output_refs,
        control_plane_intent_ref: transition.control_plane_intent_ref,
        prior_receipt_ref: transition.prior_receipt_ref,
        dataspace_assertion_refs,
        diagnostics,
        value: value.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedReceiptTransition {
    kind: String,
    before_state_ref: String,
    after_state_ref: Option<String>,
    preserved_state_ref: Option<String>,
    output_refs: Vec<String>,
    control_plane_intent_ref: Option<String>,
    prior_receipt_ref: Option<String>,
}

fn coordination_transition_value(input: ReceiptTransitionInput<'_>) -> Result<IoValue> {
    validate_transition_kind(input.kind)?;
    validate_ref(input.before_state_ref, "coordination transition before state ref")?;
    if let Some(value) = input.after_state_ref {
        validate_ref(value, "coordination transition after state ref")?;
    }
    if let Some(value) = input.preserved_state_ref {
        validate_ref(value, "coordination transition preserved state ref")?;
    }
    validate_refs(input.output_refs, "coordination transition output ref")?;
    if let Some(value) = input.control_plane_intent_ref {
        validate_ref(value, "coordination transition control-plane intent ref")?;
    }
    if let Some(value) = input.prior_receipt_ref {
        validate_ref(value, "coordination transition prior receipt ref")?;
    }
    Ok(record("transition", vec![
        record("kind", vec![string(input.kind)]),
        record("before-state", vec![string(input.before_state_ref)]),
        record("after-state", vec![optional_ref_value(input.after_state_ref)]),
        record("preserved-state", vec![optional_ref_value(input.preserved_state_ref)]),
        record("outputs", vec![strings_sequence(input.output_refs)]),
        record("control-plane-intent", vec![optional_ref_value(input.control_plane_intent_ref)]),
        record("prior-receipt", vec![optional_ref_value(input.prior_receipt_ref)]),
        checks_value(&[("transition-state-bound", "pass")]),
    ]))
}

fn parse_coordination_transition(value: &Value<IoValue>) -> Result<ParsedReceiptTransition> {
    let transition = value_to_iovalue(value);
    let fields = simple_record(&transition, "transition", COORDINATION_TRANSITION_FIELD_COUNT)?;
    let kind = record_string(&fields[0], "kind")?;
    validate_transition_kind(&kind)?;
    let before_state_ref = record_ref(&fields[1], "before-state")?;
    let after_state_ref = record_optional_ref(&fields[2], "after-state")?;
    let preserved_state_ref = record_optional_ref(&fields[3], "preserved-state")?;
    let output_refs = record_ref_sequence(&fields[4], "outputs")?;
    let control_plane_intent_ref = record_optional_ref(&fields[5], "control-plane-intent")?;
    let prior_receipt_ref = record_optional_ref(&fields[6], "prior-receipt")?;
    require_check(&parse_checks(&fields[7])?, "transition-state-bound", "coordination transition")?;
    Ok(ParsedReceiptTransition {
        kind,
        before_state_ref,
        after_state_ref,
        preserved_state_ref,
        output_refs,
        control_plane_intent_ref,
        prior_receipt_ref,
    })
}

pub fn new_coordination_runtime(manifest_value: &IoValue) -> Result<CoordinationRuntime> {
    let manifest = parse_coordination_service_manifest(manifest_value)?;
    let raft_manifest = crate::raft_control_plane::control_registry_fixture_manifest_value()?;
    let raft = crate::raft_control_plane::new_control_registry_model_runtime(&raft_manifest)?;
    Ok(CoordinationRuntime {
        manifest,
        raft,
        state: CoordinationState {
            next_fencing_token: 1,
            ..CoordinationState::default()
        },
        applied_operations: OrderedMap::new(),
        receipts: Vec::new(),
        assertions: Vec::new(),
        tokens: Vec::new(),
        next_sequence: 1,
    })
}

pub fn apply_coordination_request(
    runtime: &mut CoordinationRuntime,
    request_value: &IoValue,
) -> Result<CoordinationApplyResult> {
    let request = parse_coordination_request(request_value)?;
    let current_snapshot = snapshot_from_state(&runtime.state)?;
    if let Some(existing) = runtime.applied_operations.get(&request.operation_id_ref).cloned() {
        return replay_or_conflicting_duplicate(runtime, request, current_snapshot, existing);
    }
    let mut diagnostics = Vec::new();
    collect_admission_diagnostics(runtime, &request, &mut diagnostics)?;
    if diagnostics.is_empty() {
        if request.operation == OP_READ {
            return apply_coordination_read(runtime, request);
        }
        let transition = primitive_transition(runtime, &request)?;
        if transition.decision == "pass" {
            return commit_prepared_mutation(runtime, request, current_snapshot, transition);
        }
        return deny_transition_result(runtime, request, current_snapshot, transition, &["semantic-state-transition", "fail"]);
    }
    deny_result(runtime, request, current_snapshot, diagnostics, &["admission-gate", "fail"])
}

pub fn coordination_fixture_manifest_value() -> Result<IoValue> {
    let group_ref = canonical_hash(&crate::raft_control_plane::control_registry_fixture_manifest_value()?)?;
    coordination_service_manifest_value(&CoordinationServiceManifestInput {
        service_id: DEFAULT_COORDINATION_SERVICE_ID.to_string(),
        services: supported_services(),
        control_group_ref: group_ref,
        queue_capacity: DEFAULT_COORDINATION_QUEUE_CAPACITY,
        semaphore_capacity: DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
        rate_limit: DEFAULT_COORDINATION_RATE_LIMIT,
        barrier_parties: DEFAULT_COORDINATION_BARRIER_PARTIES,
        policy_refs: vec![fixture_ref("coordination-policy")],
        resource_refs: vec![fixture_ref("coordination-resource")],
    })
}

pub fn run_coordination_fixture() -> Result<CoordinationFixtureRun> {
    let manifest_value = coordination_fixture_manifest_value()?;
    let mut runtime = new_coordination_runtime(&manifest_value)?;
    let auth = vec![fixture_ref("coordination-authority")];
    let resources = vec![fixture_ref("coordination-resource")];
    let policies = vec![fixture_ref("coordination-policy")];
    let fixture_refs = CoordinationRefSlices {
        authority_refs: &auth,
        resource_refs: &resources,
        policy_refs: &policies,
    };
    let mut evidence_values = vec![manifest_value.clone()];
    let mut receipt_refs = Vec::new();
    let mut assertion_refs = Vec::new();
    for request_value in case_requests(&fixture_refs)? {
        let result = apply_coordination_request(&mut runtime, &request_value)?;
        evidence_values.push(request_value);
        evidence_values.extend(result.evidence_values.iter().cloned());
        receipt_refs.push_limited(
            result.receipt.receipt_ref.clone(),
            MAX_COORDINATION_ITEMS,
            "coordination fixture receipts",
        )?;
        for assertion in &result.assertions {
            assertion_refs.push_limited(
                assertion.assertion_ref.clone(),
                MAX_COORDINATION_ITEMS,
                "coordination fixture assertions",
            )?;
        }
    }
    let final_state = snapshot_from_state(&runtime.state)?;
    let report_value = case_report(&manifest_value, &final_state.state_ref, &receipt_refs, &assertion_refs)?;
    evidence_values.push(final_state.value.clone());
    evidence_values.push(report_value.clone());
    Ok(CoordinationFixtureRun {
        decision: "pass".to_string(),
        manifest_ref: canonical_hash(&manifest_value)?,
        final_state_ref: final_state.state_ref,
        receipt_refs,
        assertion_refs,
        evidence_values,
        report_value,
    })
}

fn case_requests(refs: &CoordinationRefSlices<'_>) -> Result<Vec<IoValue>> {
    let cases = [
        (SERVICE_LOCK, OP_ACQUIRE, "resource:alpha", "client-a", 1, None),
        (SERVICE_LOCK, OP_ACQUIRE, "resource:alpha", "client-a", 1, None),
        (SERVICE_LOCK, OP_RELEASE, "resource:alpha", "client-b", 2, Some(record("token", vec![u64_value(0)]))),
        (SERVICE_QUEUE, OP_ENQUEUE, "queue:work", "client-a", 3, Some(record("item", vec![string("job-1")]))),
        (SERVICE_QUEUE, OP_DEQUEUE, "queue:work", "client-b", 4, None),
        (
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:api",
            "client-a",
            5,
            Some(record("endpoint", vec![
                string(fixture_ref("endpoint-api")),
                string(fixture_ref("registry-evidence")),
            ])),
        ),
        (SERVICE_REGISTRY, OP_READ, "svc:api", "client-b", 6, None),
    ];
    let mut requests = Vec::with_capacity(cases.len());
    for (service, operation, key, client_session, sequence, payload) in cases {
        requests.push(fixture_request(FixtureRequestInput {
            service,
            operation,
            key,
            client_session,
            sequence,
            payload,
            refs,
        })?);
    }
    Ok(requests)
}

fn case_report(
    manifest_value: &IoValue,
    final_state_ref: &str,
    receipt_refs: &[String],
    assertion_refs: &[String],
) -> Result<IoValue> {
    Ok(record("coordination-fixture-report-v1", vec![
        string("molten.coordination.fixture-report.v1"),
        record("decision", vec![string("pass")]),
        record("manifest", vec![string(canonical_hash(manifest_value)?)]),
        record("state", vec![string(final_state_ref)]),
        record("receipts", vec![strings_sequence(receipt_refs)]),
        record("assertions", vec![strings_sequence(assertion_refs)]),
    ]))
}

pub fn coordination_apply_report_value(input: ApplyReportValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.manifest_ref, "coordination apply report manifest ref")?;
    validate_ref(input.final_state_ref, "coordination apply report state ref")?;
    validate_refs(input.receipt_refs, "coordination apply report receipt ref")?;
    validate_refs(input.assertion_refs, "coordination apply report assertion ref")?;
    validate_refs(input.evidence_refs, "coordination apply report evidence ref")?;
    Ok(record("coordination-apply-report-v1", vec![
        string(COORDINATION_APPLY_REPORT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("state", vec![string(input.final_state_ref)]),
        record("receipts", vec![strings_sequence(input.receipt_refs)]),
        record("assertions", vec![strings_sequence(input.assertion_refs)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[
            ("control-plane-apply-batch", "pass"),
            ("evidence-index-bound", "pass"),
            ("dataspace-observation-only", "pass"),
        ]),
    ]))
}
