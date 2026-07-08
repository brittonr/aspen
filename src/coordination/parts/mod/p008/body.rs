const COORDINATION_BATCH_ENVELOPE_SCHEMA: &str = "molten.coordination.batch-envelope.v1";
const COORDINATION_BATCH_FIELD_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationBatchEnvelopeInput {
    pub batch_id_ref: String,
    pub compare_state_ref: Option<String>,
    pub requests: Vec<IoValue>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationBatchEnvelope {
    pub batch_ref: String,
    pub batch_id_ref: String,
    pub compare_state_ref: Option<String>,
    pub request_refs: Vec<String>,
    pub requests: Vec<IoValue>,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationBatchApplyResult {
    pub decision: String,
    pub batch_ref: String,
    pub report: CoordinationApplyReport,
    pub results: Vec<CoordinationApplyResult>,
    pub evidence_values: Vec<IoValue>,
}

// r[impl molten.coordination.batched_control_plane_operations]
pub fn coordination_batch_envelope_value(input: &CoordinationBatchEnvelopeInput) -> Result<IoValue> {
    validate_ref(&input.batch_id_ref, "coordination batch id ref")?;
    if let Some(reference) = &input.compare_state_ref {
        validate_ref(reference, "coordination batch compare state ref")?;
    }
    validate_refs(&input.policy_refs, "coordination batch policy ref")?;
    validate_batch_requests(&input.requests)?;
    let request_refs = request_refs_for(&input.requests)?;
    Ok(record("coordination-batch-envelope-v1", vec![
        string(COORDINATION_BATCH_ENVELOPE_SCHEMA),
        record("batch-id", vec![string(&input.batch_id_ref)]),
        record("compare-state", vec![optional_ref_value(input.compare_state_ref.as_deref())]),
        record("request-refs", vec![strings_sequence(&request_refs)]),
        record("requests", vec![sequence(input.requests.clone())]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("operation-ids", vec![strings_sequence(&operation_ids_for(&input.requests)?)]),
        checks_value(&[
            ("coordination-batch-envelope", "pass"),
            ("per-operation-ids-bound", "pass"),
            ("compare-state-declared", if input.compare_state_ref.is_some() { "pass" } else { "diagnostic" }),
        ]),
    ]))
}

pub fn parse_coordination_batch_envelope(value: &IoValue) -> Result<CoordinationBatchEnvelope> {
    let fields = simple_record(value, "coordination-batch-envelope-v1", COORDINATION_BATCH_FIELD_COUNT)?;
    require_schema(&fields[0], COORDINATION_BATCH_ENVELOPE_SCHEMA, "coordination batch schema")?;
    let batch_id_ref = record_ref(&fields[1], "batch-id")?;
    let compare_state_ref = record_optional_ref(&fields[2], "compare-state")?;
    let request_refs = record_ref_sequence(&fields[3], "request-refs")?;
    let requests = record_iovalue_sequence(&fields[4], "requests")?;
    let policy_refs = record_ref_sequence(&fields[5], "policy")?;
    let operation_ids = record_ref_sequence(&fields[6], "operation-ids")?;
    require_check(&parse_checks(&fields[7])?, "coordination-batch-envelope", "coordination batch")?;
    validate_batch_requests(&requests)?;
    if request_refs_for(&requests)? != request_refs {
        return Err(MoltenError::invalid_harness("coordination batch request refs do not match embedded requests"));
    }
    if operation_ids_for(&requests)? != operation_ids {
        return Err(MoltenError::invalid_harness("coordination batch operation ids do not match embedded requests"));
    }
    Ok(CoordinationBatchEnvelope {
        batch_ref: canonical_hash(value)?,
        batch_id_ref,
        compare_state_ref,
        request_refs,
        requests,
        policy_refs,
        value: value.clone(),
    })
}

// r[impl molten.coordination.batched_control_plane_operations]
pub fn apply_coordination_batch(
    runtime: &mut CoordinationRuntime,
    envelope_value: &IoValue,
) -> Result<CoordinationBatchApplyResult> {
    let envelope = parse_coordination_batch_envelope(envelope_value)?;
    let preflight = batch_preflight(runtime, &envelope)?;
    if !preflight.diagnostics.is_empty() {
        return denied_batch(runtime, envelope, preflight);
    }
    let mut evidence_values = Vec::new();
    evidence_values.push(envelope.value.clone());
    let mut results = Vec::with_capacity(envelope.requests.len());
    let mut receipt_refs = Vec::with_capacity(envelope.requests.len());
    let mut assertion_refs = Vec::new();
    let mut decision = "pass";
    for request in &envelope.requests {
        let result = apply_coordination_request(runtime, request)?;
        if result.receipt.decision != "pass" {
            decision = "deny";
        }
        receipt_refs.push(result.receipt.receipt_ref.clone());
        assertion_refs.extend(result.assertions.iter().map(|assertion| assertion.assertion_ref.clone()));
        evidence_values.extend(result.evidence_values.iter().cloned());
        results.push(result);
    }
    let final_state = snapshot_from_state(&runtime.state)?;
    evidence_values.push(final_state.value.clone());
    let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let report_value = coordination_apply_report_value(ApplyReportValueInput {
        decision,
        manifest_ref: &runtime.manifest.manifest_ref,
        final_state_ref: &final_state.state_ref,
        receipt_refs: &receipt_refs,
        assertion_refs: &assertion_refs,
        evidence_refs: &evidence_refs,
    })?;
    let report = parse_coordination_apply_report(&report_value)?;
    evidence_values.push(report_value);
    Ok(CoordinationBatchApplyResult {
        decision: decision.to_string(),
        batch_ref: envelope.batch_ref,
        report,
        results,
        evidence_values,
    })
}

struct BatchPreflight {
    snapshot: CoordinationStateSnapshot,
    diagnostics: Vec<String>,
}

fn batch_preflight(runtime: &CoordinationRuntime, envelope: &CoordinationBatchEnvelope) -> Result<BatchPreflight> {
    let snapshot = snapshot_from_state(&runtime.state)?;
    let mut diagnostics = Vec::new();
    if let Some(compare_state_ref) = envelope.compare_state_ref.as_deref() {
        if compare_state_ref != snapshot.state_ref {
            diagnostics.push(format!(
                "coordination batch compare-state mismatch: expected {compare_state_ref}, current {}",
                snapshot.state_ref
            ));
        }
    }
    let mut operation_ids = OrderedSet::new();
    for request_value in &envelope.requests {
        let request = parse_coordination_request(request_value)?;
        if !operation_ids.insert(request.operation_id_ref.clone()) {
            diagnostics.push(format!("duplicate coordination batch operation id {}", request.operation_id_ref));
        }
        collect_admission_diagnostics(runtime, &request, &mut diagnostics)?;
    }
    Ok(BatchPreflight { snapshot, diagnostics })
}

fn denied_batch(
    runtime: &CoordinationRuntime,
    envelope: CoordinationBatchEnvelope,
    preflight: BatchPreflight,
) -> Result<CoordinationBatchApplyResult> {
    let mut evidence_values = vec![envelope.value.clone(), preflight.snapshot.value.clone()];
    let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let report_value = coordination_apply_report_value(ApplyReportValueInput {
        decision: "deny",
        manifest_ref: &runtime.manifest.manifest_ref,
        final_state_ref: &preflight.snapshot.state_ref,
        receipt_refs: &[],
        assertion_refs: &[],
        evidence_refs: &evidence_refs,
    })?;
    let report = parse_coordination_apply_report(&report_value)?;
    evidence_values.push(report_value);
    Ok(CoordinationBatchApplyResult {
        decision: "deny".to_string(),
        batch_ref: envelope.batch_ref,
        report,
        results: Vec::new(),
        evidence_values,
    })
}

fn validate_batch_requests(requests: &[IoValue]) -> Result<()> {
    ensure_count_at_most(requests.len(), MAX_COORDINATION_ITEMS, "coordination batch requests")?;
    if requests.is_empty() {
        return Err(MoltenError::invalid_harness("coordination batch requires at least one request"));
    }
    for request in requests {
        parse_coordination_request(request)?;
    }
    Ok(())
}

fn request_refs_for(requests: &[IoValue]) -> Result<Vec<String>> {
    requests.iter().map(canonical_hash).collect()
}

fn operation_ids_for(requests: &[IoValue]) -> Result<Vec<String>> {
    requests
        .iter()
        .map(|request| parse_coordination_request(request).map(|parsed| parsed.operation_id_ref))
        .collect()
}

fn record_iovalue_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_COORDINATION_ITEMS, label)?;
    Ok(items.iter().map(value_to_iovalue).collect())
}
