
pub fn vm_fault_receipt_value(input: &NixosVmFaultReceiptInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_content_ref(input.descriptor_ref)?;
    validate_host_support(input.host_support)?;
    validate_ref_slice("fault pre", input.pre_fault_refs)?;
    validate_ref_slice("fault injection", input.injection_refs)?;
    validate_ref_slice("fault child", input.child_refs)?;
    validate_ref_slice("fault post", input.post_fault_refs)?;
    validate_text_field("fault replay status", input.replay_status)?;
    validate_ref_slice("fault log", input.log_refs)?;
    Ok(record("nixos-vm-fault-receipt-v1", vec![
        string(NIXOS_VM_FAULT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("descriptor", vec![string(input.descriptor_ref)]),
        record("host-support", vec![string(input.host_support)]),
        record("pre-fault", vec![sequence(ref_values(input.pre_fault_refs)?)]),
        record("injection", vec![sequence(ref_values(input.injection_refs)?)]),
        record("children", vec![sequence(ref_values(input.child_refs)?)]),
        record("post-fault", vec![sequence(ref_values(input.post_fault_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "fault diagnostic",
            input.diagnostics,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "fault receipt caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("canonical-fault-descriptor-bound", "pass"),
            check_value("unsupported-is-not-pass-evidence", "pass"),
            check_value("logs-diagnostic-only", "pass"),
            check_value("vm-fault-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn network_control_probe_value(input: &NixosVmNetworkControlProbeInput<'_>) -> Result<IoValue> {
    validate_text_field("network-control backend", input.backend)?;
    validate_text_field("network-control target link", input.target_link)?;
    validate_content_ref(input.topology_ref)?;
    validate_host_support(input.host_support)?;
    validate_text_field("network-control cleanup strategy", input.cleanup_strategy)?;
    Ok(record("nixos-vm-network-control-probe-v1", vec![
        string(NIXOS_VM_NETWORK_CONTROL_PROBE_SCHEMA),
        record("backend", vec![string(input.backend)]),
        record("target-link", vec![string(input.target_link)]),
        record("topology", vec![string(input.topology_ref)]),
        record("host-support", vec![string(input.host_support)]),
        record("cleanup-strategy", vec![string(input.cleanup_strategy)]),
        record("diagnostics", vec![sequence(string_values(
            "network-control diagnostic",
            input.diagnostics,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "network-control caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("backend-explicit", "pass"),
            check_value("cleanup-strategy-explicit", "pass"),
            check_value("unavailable-is-not-pass-evidence", "pass"),
        ])]),
    ]))
}

pub fn evaluate_vm_shard_run(input: &NixosVmShardRunInput<'_>) -> Result<NixosVmShardRunReceipt> {
    let mut diagnostics = Vec::new();
    collect_vm_shard_diagnostics(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_shard_run_value(input, &decision, &diagnostics)?;
    let shard_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(NixosVmShardRunReceipt {
        decision,
        diagnostics,
        shard_ref,
        value,
    })
}

pub fn evaluate_vm_aggregate(input: &NixosVmAggregateInput<'_>) -> Result<NixosVmAggregateReceipt> {
    let mut diagnostics = Vec::new();
    collect_vm_aggregate_diagnostics(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_aggregate_value(input, &decision, &diagnostics)?;
    let aggregate_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(NixosVmAggregateReceipt {
        decision,
        diagnostics,
        aggregate_ref,
        value,
    })
}

fn collect_vm_shard_diagnostics(
    input: &NixosVmShardRunInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    // r[impl molten.testing.vm_shard_scope.synthetic_metadata_boundary]
    // r[impl molten.testing.vm_shard_scope.aggregate_scope_denial]
    validate_text_field("VM shard", input.shard_id)?;
    validate_content_ref(input.scenario_fixture_ref)?;
    validate_content_ref(input.topology_ref)?;
    validate_content_ref(input.package_ref)?;
    validate_vm_evidence_scope(input.evidence_scope)?;
    validate_decision(input.claimed_decision)?;
    validate_ref_slice("VM shard node evidence", input.node_evidence_refs)?;
    validate_ref_slice("VM shard child receipt", input.child_receipt_refs)?;
    validate_ref_slice("VM shard diagnostic log", input.diagnostic_log_refs)?;
    if input.claimed_decision == "pass" && input.evidence_scope != NIXOS_VM_SCOPE_EXECUTABLE_VM {
        diagnostics.push_item(format!("vm-shard-non-executable-pass:{}:{}", input.shard_id, input.evidence_scope));
    }
    if input.claimed_decision == "pass" && input.unavailable {
        diagnostics.push_item(format!("vm-shard-unavailable-as-pass:{}", input.shard_id));
    }
    if input.claimed_decision == "pass" && input.child_receipt_refs.is_empty() {
        diagnostics.push_item(format!("vm-shard-log-only-pass:{}", input.shard_id));
    }
    if input.node_evidence_refs.is_empty() {
        diagnostics.push_item(format!("vm-shard-missing-node-evidence:{}", input.shard_id));
    }
    if input.diagnostic_log_refs.is_empty() {
        diagnostics.push_item(format!("vm-shard-missing-diagnostic-log:{}", input.shard_id));
    }
    if input.caveats.is_empty() {
        diagnostics.push_item(format!("vm-shard-missing-caveat:{}", input.shard_id));
    }
    Ok(())
}

fn collect_vm_aggregate_diagnostics(
    input: &NixosVmAggregateInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    // r[impl molten.testing.vm_shard_scope.aggregate_scope_denial]
    validate_content_ref(input.topology_ref)?;
    validate_content_ref(input.package_ref)?;
    validate_content_ref(input.manifest_ref)?;
    validate_strings("VM aggregate required shard", input.required_shard_ids, MAX_VM_TEXT_FIELDS)?;
    validate_ref_slice("VM aggregate shard", input.shard_refs)?;
    validate_strings("VM aggregate shard scope", input.shard_scopes, MAX_VM_TEXT_FIELDS)?;
    for scope in input.shard_scopes {
        validate_vm_evidence_scope(scope)?;
    }
    validate_strings("VM aggregate denied shard", input.denied_shard_ids, MAX_VM_TEXT_FIELDS)?;
    validate_strings(
        "VM aggregate unavailable-as-pass shard",
        input.unavailable_as_pass_shard_ids,
        MAX_VM_TEXT_FIELDS,
    )?;
    validate_ref_slice("VM aggregate stale child", input.stale_child_refs)?;
    validate_ref_slice("VM aggregate log-only child", input.log_only_child_refs)?;
    if input.required_shard_ids.is_empty() {
        diagnostics.push_item("vm-aggregate-missing-required-shards".to_string());
    }
    if input.shard_refs.len() < input.required_shard_ids.len() {
        diagnostics.push_item("vm-aggregate-missing-shard-ref".to_string());
    }
    if input.shard_scopes.len() != input.shard_refs.len() {
        diagnostics.push_item("vm-aggregate-shard-scope-count-mismatch".to_string());
    }
    for scope in input.shard_scopes {
        if scope != NIXOS_VM_SCOPE_EXECUTABLE_VM {
            diagnostics.push_item(format!("vm-aggregate-non-executable-platform-scope:{scope}"));
        }
    }
    for shard_id in input.denied_shard_ids {
        diagnostics.push_item(format!("vm-aggregate-denied-shard:{shard_id}"));
    }
    for shard_id in input.unavailable_as_pass_shard_ids {
        diagnostics.push_item(format!("vm-aggregate-unavailable-as-pass:{shard_id}"));
    }
    for stale_ref in input.stale_child_refs {
        diagnostics.push_item(format!("vm-aggregate-stale-child:{stale_ref}"));
    }
    for log_only_ref in input.log_only_child_refs {
        diagnostics.push_item(format!("vm-aggregate-log-only-child:{log_only_ref}"));
    }
    if input.caveats.is_empty() {
        diagnostics.push_item("vm-aggregate-missing-caveat".to_string());
    }
    Ok(())
}

fn vm_shard_run_value(input: &NixosVmShardRunInput<'_>, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("nixos-vm-shard-run-v1", vec![
        string(NIXOS_VM_SHARD_RUN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("claimed-decision", vec![string(input.claimed_decision)]),
        record("shard", vec![string(input.shard_id)]),
        record("scenario-fixture", vec![string(input.scenario_fixture_ref)]),
        record("topology", vec![string(input.topology_ref)]),
        record("package", vec![string(input.package_ref)]),
        record("evidence-scope", vec![string(input.evidence_scope)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("children", vec![sequence(ref_values(input.child_receipt_refs)?)]),
        record("diagnostic-logs", vec![sequence(ref_values(input.diagnostic_log_refs)?)]),
        record("unavailable", vec![crate::preserves_rail::bool_value(input.unavailable)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("caveats", vec![sequence(string_values(
            "VM shard caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("scenario-bound", status(decision == "pass")),
            check_value("evidence-scope-explicit", "pass"),
            check_value("logs-diagnostic-only", "pass"),
            check_value("unavailable-is-not-pass", status(!input.unavailable || input.claimed_decision != "pass")),
        ])]),
    ]))
}

fn vm_aggregate_value(input: &NixosVmAggregateInput<'_>, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("nixos-vm-multinode-aggregate-v1", vec![
        string(NIXOS_VM_MULTINODE_AGGREGATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("topology", vec![string(input.topology_ref)]),
        record("package", vec![string(input.package_ref)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("required-shards", vec![sequence(input.required_shard_ids.iter().map(string).collect())]),
        record("shards", vec![sequence(ref_values(input.shard_refs)?)]),
        record("shard-scopes", vec![sequence(input.shard_scopes.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("caveats", vec![sequence(string_values(
            "VM aggregate caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("child-shards-bound", status(!input.shard_refs.is_empty())),
            check_value(
                "child-scope-preserved",
                status(input.shard_scopes.iter().all(|scope| scope == NIXOS_VM_SCOPE_EXECUTABLE_VM)),
            ),
            check_value("unavailable-not-promoted", status(input.unavailable_as_pass_shard_ids.is_empty())),
            check_value("logs-diagnostic-only", "pass"),
        ])]),
    ]))
}

fn validate_nodes(nodes: &[String]) -> Result<()> {
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("nixos VM topology requires at least one node"));
    }
    if nodes.len() > MAX_VM_NODES {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM topology node count {} exceeds bound {MAX_VM_NODES}",
            nodes.len()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for node in nodes {
        validate_text_field("node", node)?;
        if !seen.insert(node.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate nixos VM node {node}")));
        }
    }
    Ok(())
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("nixos VM {label} must not be empty")));
    }
    Ok(())
}

fn validate_optional_text(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        validate_text_field(label, value)?;
    }
    Ok(())
}
