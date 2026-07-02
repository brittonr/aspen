const _: () = assert!(MAX_NODE_DIAGNOSTICS > 0);

struct GateScan {
    validation_refs: Vec<String>,
    diagnostics: Vec<String>,
}

struct AdapterStart {
    values: Vec<IoValue>,
    receipts: Vec<NodeAdapterReceiptRef>,
}

fn validate_start_input(config: &NodeConfig, input: &NodeRuntimeStartInput) -> Result<()> {
    validate_refs(&input.index_receipt_refs, "node runtime index receipt ref")?;
    validate_refs(&input.source_gate_receipt_refs, "node runtime source gate receipt ref")?;
    validate_refs(&input.capability_receipt_refs, "node runtime capability receipt ref")?;
    validate_refs(&input.resource_receipt_refs, "node runtime resource receipt ref")?;
    validate_refs(&input.version_refs, "node runtime version ref")?;
    ensure_count_at_most(config.adapters.len(), MAX_NODE_ADAPTERS, "node runtime adapters")?;
    ensure_count_at_most(
        input.source_gate_receipt_refs.len(),
        MAX_NODE_SOURCE_GATE_RECEIPTS,
        "node runtime source gate receipt refs",
    )?;
    ensure_count_at_most(
        input.source_gate_receipt_values.len(),
        MAX_NODE_SOURCE_GATE_RECEIPTS,
        "node runtime source gate receipt values",
    )?;
    validate_ref(&input.identity_receipt_ref, "node runtime identity receipt ref")
}

fn startup_diagnostics(config: &NodeConfig, input: &NodeRuntimeStartInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let missing = missing_required_adapters(&config.adapters);
    if !missing.is_empty() {
        push_bounded(
            &mut diagnostics,
            format!("missing required node runtime adapters: {}", missing.join(",")),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.index_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires adapter index verification receipts".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.source_gate_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires strict Octet source gate receipt refs".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.resource_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires resource profile receipts".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.source_gate_receipt_refs.len() != input.source_gate_receipt_values.len() {
        push_bounded(
            &mut diagnostics,
            "node runtime source gate refs must have matching receipt values".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn scan_gates(config_ref: &str, input: &NodeRuntimeStartInput) -> Result<GateScan> {
    let mut validation_refs = Vec::with_capacity(input.source_gate_receipt_values.len());
    let mut diagnostics = Vec::new();
    for (index, value) in input.source_gate_receipt_values.iter().enumerate() {
        let validation =
            crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
                consumer: "node-startup".to_string(),
                subject_ref: config_ref.to_string(),
                receipt_value: Some(value.clone()),
                source_scope: Vec::new(),
            })?;
        if let Some(expected_ref) = input.source_gate_receipt_refs.get(index)
            && validation.gate_receipt_ref.as_ref() != Some(expected_ref)
        {
            push_bounded(
                &mut diagnostics,
                format!(
                    "node runtime source gate ref {expected_ref} does not match validated receipt {:?}",
                    validation.gate_receipt_ref
                ),
                MAX_NODE_DIAGNOSTICS,
                "node runtime startup diagnostics",
            )?;
        }
        push_bounded(
            &mut validation_refs,
            validation.validation_ref.clone(),
            MAX_NODE_SOURCE_GATE_RECEIPTS,
            "node runtime source gate validation refs",
        )?;
        if validation.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                format!("node runtime strict Octet source gate validation {} denied", validation.validation_ref),
                MAX_NODE_DIAGNOSTICS,
                "node runtime startup diagnostics",
            )?;
        }
    }
    Ok(GateScan {
        validation_refs,
        diagnostics,
    })
}

fn adapter_start(
    ordered: &[NodeAdapterBinding],
    input: &NodeRuntimeStartInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<AdapterStart> {
    let mut values = Vec::with_capacity(ordered.len());
    for adapter in ordered {
        values.push(node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
            operation: "start",
            decision,
            adapter,
            index_receipt_refs: &input.index_receipt_refs,
            resource_receipt_refs: &input.resource_receipt_refs,
            diagnostics,
        })?);
    }
    let mut receipts = Vec::with_capacity(ordered.len());
    for (adapter, receipt) in ordered.iter().zip(values.iter()) {
        receipts.push(NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref: canonical_hash(receipt)?,
        });
    }
    Ok(AdapterStart { values, receipts })
}

pub fn start_node_runtime(input: &NodeRuntimeStartInput) -> Result<NodeRuntimeStart> {
    let config = parse_node_config(&input.config_value)?;
    validate_start_input(&config, input)?;
    let ordered = deterministic_adapter_bindings(&config.adapters);
    let mut diagnostics = startup_diagnostics(&config, input)?;
    let gate_scan = scan_gates(&config.config_ref, input)?;
    for diagnostic in gate_scan.diagnostics {
        push_bounded(&mut diagnostics, diagnostic, MAX_NODE_DIAGNOSTICS, "node runtime startup diagnostics")?;
    }
    let source_gate_validation_refs = gate_scan.validation_refs;
    let has_valid_source_gate_values = !source_gate_validation_refs.is_empty()
        && diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains("source gate") && !diagnostic.contains("Source gate"));
    if !has_valid_source_gate_values
        && !input.source_gate_receipt_refs.is_empty()
        && input.source_gate_receipt_values.is_empty()
    {
        push_bounded(
            &mut diagnostics,
            "node runtime source gate refs lack validated receipt content".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let adapter_start = adapter_start(&ordered, input, decision, &diagnostics)?;
    let startup_value = node_startup_receipt_value(&StartupReceiptValueInput {
        decision,
        config: &config,
        identity_receipt_ref: &input.identity_receipt_ref,
        adapter_receipts: &adapter_start.receipts,
        source_gate_receipt_refs: &input.source_gate_receipt_refs,
        source_gate_validation_refs: &source_gate_validation_refs,
        capability_receipt_refs: &input.capability_receipt_refs,
        resource_receipt_refs: &input.resource_receipt_refs,
        version_refs: &input.version_refs,
        diagnostics: &diagnostics,
    })?;
    let startup_receipt = parse_node_startup_receipt(&startup_value)?;
    Ok(NodeRuntimeStart {
        decision: decision.to_string(),
        config,
        adapter_receipt_values: adapter_start.values,
        adapter_receipts: adapter_start.receipts,
        startup_receipt,
    })
}

pub fn parse_node_config(value: &IoValue) -> Result<NodeConfig> {
    let fields = value
        .collect_simple_record("node-config-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-config-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONFIG_SCHEMA, "node config")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "explicit-state-root", "node config")?;
    require_check(&checks, "no-ambient-authority", "node config")?;
    let adapters = parse_adapter_bindings(&fields[3])?;
    if adapters.is_empty() {
        return Err(MoltenError::invalid_harness("node config requires explicit adapter profiles"));
    }
    Ok(NodeConfig {
        config_ref: canonical_hash(value)?,
        identity_ref: record_ref(&fields[1], "node-id")?,
        state_root_ref: record_ref(&fields[2], "state-root")?,
        adapters,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        capability_refs: record_ref_sequence(&fields[5], "capability")?,
        resource_refs: record_ref_sequence(&fields[6], "resource")?,
        effect_profile_refs: record_ref_sequence(&fields[7], "effects")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_adapter_receipt_value(
    operation: &str,
    decision: &str,
    adapter: &NodeAdapterBinding,
    diagnostics: &[String],
) -> Result<IoValue> {
    node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
        operation,
        decision,
        adapter,
        index_receipt_refs: &[],
        resource_receipt_refs: &[],
        diagnostics,
    })
}

pub fn node_adapter_lifecycle_receipt_value(input: &AdapterLifecycleReceiptInput<'_>) -> Result<IoValue> {
    validate_adapter_operation(input.operation)?;
    validate_decision(input.decision)?;
    validate_refs(input.index_receipt_refs, "node adapter index receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node adapter resource receipt ref")?;
    Ok(record("node-adapter-receipt-v1", vec![
        string(NODE_ADAPTER_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("adapter", vec![string(&input.adapter.name)]),
        record("profile", vec![string(&input.adapter.profile_ref)]),
        record("index", vec![refs_sequence(input.index_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("adapter-profile-bound", "pass"),
            ("adapter-index-verified", status(!input.index_receipt_refs.is_empty())),
            ("adapter-resource-profile-bound", status(!input.resource_receipt_refs.is_empty())),
            ("no-invisible-startup", if input.decision == "pass" { "pass" } else { "fail" }),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}
