
fn parse_fault_receipts(values: &[IoValue]) -> Result<Vec<ParsedFaultReceipt>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM fault receipt count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_fault_receipt(value)?);
    }
    Ok(output)
}

fn parse_fault_receipt(value: &IoValue) -> Result<ParsedFaultReceipt> {
    let receipt = simple_record(value, "nixos-vm-fault-receipt-v1", FAULT_RECEIPT_ARITY)?;
    require_schema(&receipt[FAULT_RECEIPT_SCHEMA_INDEX], VM_FAULT_RECEIPT_SCHEMA, "fault receipt")?;
    Ok(ParsedFaultReceipt {
        decision: required_record_string(&receipt[FAULT_RECEIPT_DECISION_INDEX], "decision", "fault receipt decision")?,
        descriptor_ref: required_record_ref(
            &receipt[FAULT_RECEIPT_DESCRIPTOR_INDEX],
            "descriptor",
            "fault descriptor",
        )?,
        host_support: required_record_string(
            &receipt[FAULT_RECEIPT_HOST_SUPPORT_INDEX],
            "host-support",
            "fault host support",
        )?,
        pre_fault_refs: required_ref_sequence_record(&receipt[FAULT_RECEIPT_PRE_INDEX], "pre-fault", "fault pre refs")?,
        injection_refs: required_ref_sequence_record(
            &receipt[FAULT_RECEIPT_INJECTION_INDEX],
            "injection",
            "fault injection refs",
        )?,
        child_refs: required_ref_sequence_record(&receipt[FAULT_RECEIPT_CHILDREN_INDEX], "children", "fault children")?,
        post_fault_refs: required_ref_sequence_record(
            &receipt[FAULT_RECEIPT_POST_INDEX],
            "post-fault",
            "fault post refs",
        )?,
        replay_status: required_record_string(&receipt[FAULT_RECEIPT_REPLAY_INDEX], "replay-status", "fault replay")?,
        diagnostics: required_string_sequence_record(
            &receipt[FAULT_RECEIPT_DIAGNOSTICS_INDEX],
            "diagnostics",
            "fault diagnostics",
        )?,
        log_refs: required_ref_sequence_record(&receipt[FAULT_RECEIPT_LOGS_INDEX], "logs", "fault logs")?,
        caveats: required_string_sequence_record(&receipt[FAULT_RECEIPT_CAVEATS_INDEX], "caveats", "fault caveats")?,
    })
}

fn parse_prod_soaks(values: &[IoValue]) -> Result<Vec<ParsedProdSoakRun>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM prod-soak count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_prod_soak_run(value)?);
    }
    Ok(output)
}

fn parse_prod_soak_run(value: &IoValue) -> Result<ParsedProdSoakRun> {
    let run = value
        .collect_simple_record("prod-soak-run-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected prod-soak-run-v1 receipt"))?;
    require_schema(&run[0], crate::preserves_rail::PROD_SOAK_RUN_SCHEMA, "prod soak run")?;
    Ok(ParsedProdSoakRun {
        decision: required_record_string(&run[SOAK_RUN_DECISION_INDEX], "decision", "prod soak decision")?,
        topology_ref: required_record_ref(&run[SOAK_RUN_TOPOLOGY_INDEX], "topology", "prod soak topology")?,
        node_evidence_refs: required_ref_sequence_record(
            &run[SOAK_RUN_NODE_EVIDENCE_INDEX],
            "node-evidence",
            "prod soak nodes",
        )?,
        replay_status: required_record_string(&run[SOAK_RUN_REPLAY_INDEX], "replay-status", "prod soak replay status")?,
        caveats: required_string_sequence_record(&run[SOAK_RUN_CAVEATS_INDEX], "caveats", "prod soak caveats")?,
    })
}

fn vm_evidence_validation_value(input: ValidationValueInput<'_>) -> Result<IoValue> {
    crate::preserves_rail::validate_content_ref(input.topology_ref)?;
    crate::preserves_rail::validate_content_ref(input.test_run_ref)?;
    validate_ref_list("node evidence", input.node_evidence_refs)?;
    validate_ref_list("prod soak", input.prod_soak_refs)?;
    validate_ref_list("child artifact", input.child_artifact_refs)?;
    validate_decision(input.decision)?;
    validate_strings("validation diagnostic", input.diagnostics)?;
    Ok(record("nixos-vm-evidence-validation-v1", vec![
        string(VM_EVIDENCE_VALIDATION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("topology", vec![string(input.topology_ref)]),
        record("node-evidence", vec![sequence(input.node_evidence_refs.iter().map(string).collect())]),
        record("test-run", vec![string(input.test_run_ref)]),
        record("prod-soak", vec![sequence(input.prod_soak_refs.iter().map(string).collect())]),
        record("child-artifacts", vec![sequence(input.child_artifact_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            check_value("canonical-receipts-parsed", "pass"),
            check_value("topology-bound", status(input.decision == "pass")),
            check_value("logs-diagnostic-only", "pass"),
            check_value("evidence-does-not-grant-authority", "pass"),
            check_value("child-receipts-bound", status(input.decision == "pass")),
        ])]),
    ]))
}

fn vm_fault_validation_value(
    decision: &str,
    topology_ref: &str,
    descriptor_refs: &[String],
    receipt_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    crate::preserves_rail::validate_content_ref(topology_ref)?;
    validate_ref_list("fault descriptor", descriptor_refs)?;
    validate_ref_list("fault receipt", receipt_refs)?;
    validate_strings("fault validation diagnostic", diagnostics)?;
    Ok(record("nixos-vm-fault-validation-v1", vec![
        string(VM_FAULT_VALIDATION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("topology", vec![string(topology_ref)]),
        record("descriptors", vec![sequence(descriptor_refs.iter().map(string).collect())]),
        record("receipts", vec![sequence(receipt_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            check_value("descriptor-topology-bound", status(!diagnostics.iter().any(|item| item.contains("topology")))),
            check_value(
                "unsupported-is-not-pass",
                status(!diagnostics.iter().any(|item| item.contains("unavailable"))),
            ),
            check_value("logs-diagnostic-only", "pass"),
            check_value("canonical-fault-receipts-parsed", "pass"),
        ])]),
    ]))
}

fn canonical_refs(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

fn validate_manifest_entries(entries: &[VmEvidenceManifestEntry]) -> Result<()> {
    if entries.is_empty() {
        return Err(MoltenError::invalid_harness("VM evidence manifest requires entries"));
    }
    if entries.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM evidence manifest entry count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            entries.len()
        )));
    }
    let mut paths = OrderedSet::new();
    for entry in entries {
        validate_text("manifest path", &entry.path)?;
        validate_text("manifest kind", &entry.kind)?;
        crate::preserves_rail::validate_content_ref(&entry.content_ref)?;
        if !paths.insert(entry.path.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate VM evidence manifest path {}", entry.path)));
        }
    }
    Ok(())
}

fn validate_required_artifacts(required_artifacts: &[VmEvidenceManifestRequiredArtifact]) -> Result<()> {
    if required_artifacts.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM required artifact count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            required_artifacts.len()
        )));
    }
    for artifact in required_artifacts {
        validate_text("required artifact kind", &artifact.kind)?;
        crate::preserves_rail::validate_content_ref(&artifact.content_ref)?;
    }
    Ok(())
}

fn manifest_closure_diagnostics(
    entries: &[VmEvidenceManifestEntry],
    required_artifacts: &[VmEvidenceManifestRequiredArtifact],
) -> Result<Vec<String>> {
    validate_manifest_entries(entries)?;
    validate_required_artifacts(required_artifacts)?;
    let mut diagnostics = Vec::new();
    let mut semantic_artifacts = OrderedSet::new();
    let entries_by_ref = entries.iter().map(|entry| (entry.content_ref.as_str(), entry)).collect::<OrderedMap<_, _>>();
    for entry in entries {
        if !semantic_artifacts.insert((entry.kind.as_str(), entry.content_ref.as_str())) {
            push_diagnostic(
                &mut diagnostics,
                format!("duplicate-semantic-artifact:{}:{}", entry.kind, entry.content_ref),
            )?;
        }
    }
    for required in required_artifacts {
        let Some(entry) = entries_by_ref.get(required.content_ref.as_str()) else {
            push_diagnostic(
                &mut diagnostics,
                format!("required-artifact-missing:{}:{}", required.kind, required.content_ref),
            )?;
            continue;
        };
        push_if(&mut diagnostics, entry.kind != required.kind, "required-artifact-kind-mismatch")?;
        push_if(&mut diagnostics, entry.diagnostic_only, "required-artifact-only-present-as-diagnostic")?;
    }
    Ok(diagnostics)
}

fn manifest_entry_values(entries: &[VmEvidenceManifestEntry]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(record("artifact", vec![
            record("path", vec![string(&entry.path)]),
            record("kind", vec![string(&entry.kind)]),
            record("ref", vec![string(&entry.content_ref)]),
            record("diagnostic-only", vec![crate::preserves_rail::bool_value(entry.diagnostic_only)]),
        ]));
    }
    Ok(values)
}

fn required_artifact_values(required_artifacts: &[VmEvidenceManifestRequiredArtifact]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(required_artifacts.len());
    for artifact in required_artifacts {
        validate_text("required artifact kind", &artifact.kind)?;
        crate::preserves_rail::validate_content_ref(&artifact.content_ref)?;
        values.push(record("required-artifact", vec![
            record("kind", vec![string(&artifact.kind)]),
            record("ref", vec![string(&artifact.content_ref)]),
        ]));
    }
    Ok(values)
}

fn has_authoritative_entries(entries: &[VmEvidenceManifestEntry]) -> bool {
    entries.iter().any(|entry| !entry.diagnostic_only)
}

fn has_diagnostic_entries(entries: &[VmEvidenceManifestEntry]) -> bool {
    entries.iter().any(|entry| entry.diagnostic_only)
}

fn node_names(value: &Value<IoValue>) -> Result<Vec<String>> {
    let sequence = required_sequence_record(value, "nodes", "topology nodes")?;
    let mut nodes = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        let node = item
            .collect_simple_record("node", Some(1))
            .ok_or_else(|| MoltenError::invalid_harness("topology node must be <node string>"))?;
        nodes.push(required_string(&node[0], "topology node")?);
    }
    Ok(nodes)
}

fn required_ref_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<String>> {
    let refs = required_string_sequence_record(value, label, context)?;
    validate_ref_list(context, &refs)?;
    Ok(refs)
}

fn required_string_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<String>> {
    let sequence = required_sequence_record(value, label, context)?;
    let mut output = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        output.push(required_string(item, context)?);
    }
    Ok(output)
}

fn required_sequence_record(value: &Value<IoValue>, label: &str, context: &str) -> Result<Vec<Value<IoValue>>> {
    let record = simple_field_record(value, label, context)?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {context}")))?;
    Ok(sequence.into_owned())
}
