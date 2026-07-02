
pub fn node_startup_receipt_value(input: &StartupReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.identity_receipt_ref, "node startup identity receipt ref")?;
    validate_refs(input.source_gate_receipt_refs, "node startup source gate receipt ref")?;
    validate_refs(input.source_gate_validation_refs, "node startup source gate validation ref")?;
    validate_refs(input.capability_receipt_refs, "node startup capability receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node startup resource receipt ref")?;
    validate_refs(input.version_refs, "node startup version ref")?;
    for receipt in input.adapter_receipts {
        validate_adapter_name(&receipt.name)?;
        validate_ref(&receipt.receipt_ref, "node startup adapter receipt ref")?;
    }
    let adapter_names = input.adapter_receipts.iter().map(|receipt| receipt.name.clone()).collect::<Vec<_>>();
    let has_deterministic_adapter_order = adapter_names == deterministic_adapter_order(&input.config.adapters);
    Ok(record("node-startup-receipt-v1", vec![
        string(NODE_STARTUP_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("node-config", vec![string(&input.config.config_ref)]),
        record("identity", vec![string(input.identity_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("policy", vec![refs_sequence(&input.config.policy_refs)]),
        record("source-gates", vec![refs_sequence(input.source_gate_receipt_refs)]),
        record("source-gate-validations", vec![refs_sequence(input.source_gate_validation_refs)]),
        record("capability", vec![refs_sequence(input.capability_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("version", vec![refs_sequence(input.version_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("explicit-state-root", "pass"),
            ("adapter-order-deterministic", status(has_deterministic_adapter_order)),
            (
                "strict-octet-source-gate-bound",
                status(!input.source_gate_receipt_refs.is_empty() && !input.source_gate_validation_refs.is_empty()),
            ),
            ("no-ambient-authority", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn control_request_value(input: &ControlRequestValueInput<'_>) -> Result<IoValue> {
    validate_control_operation(input.operation)?;
    if let Some(target_ref) = input.target_ref {
        validate_ref(target_ref, "node control target ref")?;
    }
    if let Some(payload_ref) = input.payload_ref {
        validate_ref(payload_ref, "node control payload ref")?;
    }
    validate_refs(input.authority_refs, "node control authority ref")?;
    validate_refs(input.policy_refs, "node control policy ref")?;
    validate_refs(input.resource_refs, "node control resource ref")?;
    validate_refs(input.evidence_refs, "node control evidence ref")?;
    Ok(record("node-control-request-v1", vec![
        string(NODE_CONTROL_REQUEST_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("control-profile", vec![string("local-preserves-control-v1")]),
        checks_value(&[
            ("local-only-control", "pass"),
            ("preserves-control-surface", "pass"),
            ("authority-refs-explicit", status(!input.authority_refs.is_empty())),
            ("resource-refs-explicit", status(!input.resource_refs.is_empty())),
            ("evidence-refs-canonical", "pass"),
        ]),
    ]))
}

pub fn legacy_node_control_request_value(input: &ControlRequestValueInput<'_>) -> Result<IoValue> {
    validate_control_operation(input.operation)?;
    if let Some(target_ref) = input.target_ref {
        validate_ref(target_ref, "node control target ref")?;
    }
    if let Some(payload_ref) = input.payload_ref {
        validate_ref(payload_ref, "node control payload ref")?;
    }
    validate_refs(input.authority_refs, "node control authority ref")?;
    validate_refs(input.policy_refs, "node control policy ref")?;
    validate_refs(input.resource_refs, "node control resource ref")?;
    Ok(record("node-control-request-v1", vec![
        string(NODE_CONTROL_REQUEST_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("control-profile", vec![string("local-preserves-control-v1")]),
        checks_value(&[
            ("local-only-control", "pass"),
            ("preserves-control-surface", "pass"),
            ("authority-refs-explicit", status(!input.authority_refs.is_empty())),
            ("resource-refs-explicit", status(!input.resource_refs.is_empty())),
        ]),
    ]))
}

pub fn parse_control_request(value: &IoValue) -> Result<ControlRequest> {
    if let Some(fields) = value.collect_simple_record("node-control-request-v1", Some(10)) {
        require_schema(&fields[0], NODE_CONTROL_REQUEST_SCHEMA, "node control request")?;
        let operation = record_string(&fields[1], "operation")?;
        validate_control_operation(&operation)?;
        return Ok(ControlRequest {
            request_ref: canonical_hash(value)?,
            operation,
            target_ref: record_optional_ref(&fields[2], "target")?,
            payload_ref: record_optional_ref(&fields[3], "payload")?,
            authority_refs: record_ref_sequence(&fields[4], "authority")?,
            policy_refs: record_ref_sequence(&fields[5], "policy")?,
            resource_refs: record_ref_sequence(&fields[6], "resource")?,
            evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
            value: value.clone(),
        });
    }
    let fields = value
        .collect_simple_record("node-control-request-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-request-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_REQUEST_SCHEMA, "node control request")?;
    let operation = record_string(&fields[1], "operation")?;
    validate_control_operation(&operation)?;
    Ok(ControlRequest {
        request_ref: canonical_hash(value)?,
        operation,
        target_ref: record_optional_ref(&fields[2], "target")?,
        payload_ref: record_optional_ref(&fields[3], "payload")?,
        authority_refs: record_ref_sequence(&fields[4], "authority")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        resource_refs: record_ref_sequence(&fields[6], "resource")?,
        evidence_refs: Vec::new(),
        value: value.clone(),
    })
}

pub fn control_receipt_value(input: &ControlReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node control startup receipt ref")?;
    validate_refs(input.authority_receipt_refs, "node control authority receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node control resource receipt ref")?;
    validate_refs(input.subreceipt_refs, "node control subreceipt ref")?;
    let has_authority_receipts = !input.request.authority_refs.is_empty() && !input.authority_receipt_refs.is_empty();
    let has_resource_receipts = !input.request.resource_refs.is_empty() && !input.resource_receipt_refs.is_empty();
    let has_required_subreceipts =
        input.request.operation == "status" || !input.subreceipt_refs.is_empty() || input.decision == "deny";
    Ok(record("node-control-receipt-v1", vec![
        string(NODE_CONTROL_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("subreceipts", vec![refs_sequence(input.subreceipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("local-preserves-control", "pass"),
            ("authority-gated", status(has_authority_receipts)),
            ("resource-gated", status(has_resource_receipts)),
            ("subreceipts-bound", status(has_required_subreceipts)),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn parse_control_receipt(value: &IoValue) -> Result<ControlReceipt> {
    let fields = value
        .collect_simple_record("node-control-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_RECEIPT_SCHEMA, "node control receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-receipt", "node control receipt")?;
    Ok(ControlReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        request_ref: record_ref(&fields[2], "request")?,
        startup_receipt_ref: record_ref(&fields[3], "startup")?,
        authority_receipt_refs: record_ref_sequence(&fields[4], "authority")?,
        resource_receipt_refs: record_ref_sequence(&fields[5], "resource")?,
        subreceipt_refs: record_ref_sequence(&fields[6], "subreceipts")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn control_deny_receipt_value(
    request: &ControlRequest,
    startup_receipt_ref: &str,
    diagnostic: &str,
) -> Result<IoValue> {
    let diagnostics = [diagnostic.to_string()];
    control_receipt_value(&ControlReceiptValueInput {
        decision: "deny",
        request,
        startup_receipt_ref,
        authority_receipt_refs: &[],
        resource_receipt_refs: &[],
        subreceipt_refs: &[],
        diagnostics: &diagnostics,
    })
}

pub fn node_shutdown_receipt_value(input: &ShutdownReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node shutdown startup receipt ref")?;
    validate_refs(input.drained_job_refs, "node shutdown drained job ref")?;
    validate_refs(input.index_receipt_refs, "node shutdown index receipt ref")?;
    for adapter in input.adapter_receipts {
        validate_adapter_name(&adapter.name)?;
        validate_ref(&adapter.receipt_ref, "node shutdown adapter receipt ref")?;
    }
    let is_graceful_shutdown =
        input.diagnostics.is_empty() && !input.adapter_receipts.is_empty() && !input.index_receipt_refs.is_empty();
    Ok(record("node-shutdown-receipt-v1", vec![
        string(crate::preserves_rail::NODE_SHUTDOWN_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("drained-jobs", vec![refs_sequence(input.drained_job_refs)]),
        record("indexes", vec![refs_sequence(input.index_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("stop-intake", "pass"),
            ("drain-complete", status(input.diagnostics.is_empty())),
            ("indexes-persisted", status(!input.index_receipt_refs.is_empty())),
            ("adapters-closed", status(!input.adapter_receipts.is_empty())),
            ("graceful-shutdown", status(is_graceful_shutdown && input.decision == "pass")),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn parse_node_shutdown_receipt(value: &IoValue) -> Result<NodeShutdownReceipt> {
    let fields = value
        .collect_simple_record("node-shutdown-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-shutdown-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_SHUTDOWN_RECEIPT_SCHEMA, "node shutdown receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "node shutdown receipt")?;
    Ok(NodeShutdownReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        adapters: parse_adapter_receipt_refs(&fields[3])?,
        drained_job_refs: record_ref_sequence(&fields[4], "drained-jobs")?,
        index_receipt_refs: record_ref_sequence(&fields[5], "indexes")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_health_receipt_value(input: &HealthReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node health startup receipt ref")?;
    if let Some(shutdown_receipt_ref) = input.shutdown_receipt_ref {
        validate_ref(shutdown_receipt_ref, "node health shutdown receipt ref")?;
    }
    validate_refs(input.index_receipt_refs, "node health index receipt ref")?;
    validate_refs(input.head_refs, "node health head ref")?;
    validate_refs(input.open_job_refs, "node health open job ref")?;
    for adapter in input.adapter_receipts {
        validate_adapter_name(&adapter.name)?;
        validate_ref(&adapter.receipt_ref, "node health adapter receipt ref")?;
    }
    Ok(record("node-health-receipt-v1", vec![
        string(crate::preserves_rail::NODE_HEALTH_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("shutdown", vec![optional_ref_value(input.shutdown_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("indexes", vec![refs_sequence(input.index_receipt_refs)]),
        record("heads", vec![refs_sequence(input.head_refs)]),
        record("open-jobs", vec![refs_sequence(input.open_job_refs)]),
        record("replay", vec![string(if input.replay_is_eligible {
            "eligible"
        } else {
            "ineligible"
        })]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("startup-verified", "pass"),
            ("shutdown-verified", status(input.shutdown_receipt_ref.is_some())),
            ("adapter-indexes-current", status(!input.index_receipt_refs.is_empty())),
            ("health-heads-bound", status(!input.head_refs.is_empty())),
            ("no-open-jobs-for-replay", status(input.open_job_refs.is_empty())),
            ("replay-eligibility", status(input.replay_is_eligible)),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}
