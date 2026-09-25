
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
