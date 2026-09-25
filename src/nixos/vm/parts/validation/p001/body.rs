
pub fn validate_nixos_vm_fault_evidence(
    input: &NixosVmFaultEvidenceValidationInput<'_>,
) -> Result<NixosVmFaultEvidenceValidation> {
    let topology = parse_topology(input.topology_value)?;
    let topology_ref = crate::preserves_rail::canonical_hash(input.topology_value)?;
    let descriptors = parse_fault_descriptors(input.descriptor_values)?;
    let receipts = parse_fault_receipts(input.receipt_values)?;
    let descriptor_refs = canonical_refs(input.descriptor_values)?;
    let receipt_refs = canonical_refs(input.receipt_values)?;
    let diagnostics =
        fault_validation_diagnostics(&topology, &topology_ref, &descriptors, &descriptor_refs, &receipts)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_fault_validation_value(&decision, &topology_ref, &descriptor_refs, &receipt_refs, &diagnostics)?;
    let validation_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(NixosVmFaultEvidenceValidation {
        decision,
        diagnostics,
        topology_ref,
        descriptor_refs,
        receipt_refs,
        validation_ref,
        value,
    })
}

pub fn build_vm_evidence_manifest(input: &VmEvidenceManifestInput<'_>) -> Result<VmEvidenceManifest> {
    let diagnostics = manifest_closure_diagnostics(input.entries, input.required_artifacts)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_evidence_manifest_value_inner(
        input.entries,
        input.required_artifacts,
        &decision,
        &diagnostics,
        input.caveats,
    )?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(VmEvidenceManifest {
        decision,
        diagnostics,
        manifest_ref,
        value,
    })
}

pub fn vm_evidence_manifest_value(entries: &[VmEvidenceManifestEntry], caveats: &[String]) -> Result<IoValue> {
    vm_evidence_manifest_value_inner(entries, &[], "pass", &[], caveats)
}

fn vm_evidence_manifest_value_inner(
    entries: &[VmEvidenceManifestEntry],
    required_artifacts: &[VmEvidenceManifestRequiredArtifact],
    decision: &str,
    diagnostics: &[String],
    caveats: &[String],
) -> Result<IoValue> {
    validate_manifest_entries(entries)?;
    validate_required_artifacts(required_artifacts)?;
    validate_decision(decision)?;
    validate_strings("manifest diagnostic", diagnostics)?;
    validate_strings("manifest caveat", caveats)?;
    Ok(record("nixos-vm-evidence-manifest-v1", vec![
        string(VM_EVIDENCE_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("artifacts", vec![sequence(manifest_entry_values(entries)?)]),
        record("required-artifacts", vec![sequence(required_artifact_values(required_artifacts)?)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("caveats", vec![sequence(caveats.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            check_value("canonical-evidence-preserved", status(has_authoritative_entries(entries))),
            check_value("diagnostic-logs-marked", status(has_diagnostic_entries(entries))),
            check_value("manifest-does-not-grant-authority", "pass"),
            check_value("required-artifact-closure", status(diagnostics.is_empty())),
        ])]),
    ]))
}

struct ValidationContext<'a> {
    topology: &'a ParsedTopology,
    topology_ref: &'a str,
    nodes: &'a [ParsedNodeEvidence],
    node_refs: &'a [String],
    test_run: &'a ParsedTestRun,
    prod_soaks: &'a [ParsedProdSoakRun],
    child_artifacts: &'a [ParsedChildReceipt],
    expected_nodes: &'a [String],
    expected_package_ref: Option<&'a str>,
    expected_child_refs: &'a [String],
    expected_child_receipts: &'a [NixosVmExpectedChildReceipt],
}

struct ValidationValueInput<'a> {
    decision: &'a str,
    diagnostics: &'a [String],
    topology_ref: &'a str,
    node_evidence_refs: &'a [String],
    test_run_ref: &'a str,
    prod_soak_refs: &'a [String],
    child_artifact_refs: &'a [String],
}

fn validation_diagnostics(input: ValidationContext<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, input.test_run.decision != "pass", "vm-test-run-not-pass")?;
    push_if(
        &mut diagnostics,
        input.test_run.topology_ref != input.topology_ref,
        "test-run-topology-ref-mismatch",
    )?;
    push_if(&mut diagnostics, input.test_run.child_refs.is_empty(), "test-run-missing-child-workflow-refs")?;
    push_if(&mut diagnostics, input.test_run.replay_status.trim().is_empty(), "test-run-missing-replay-status")?;
    push_if(&mut diagnostics, input.test_run.log_refs.is_empty(), "test-run-missing-diagnostic-log-refs")?;
    push_if(&mut diagnostics, input.test_run.caveats.is_empty(), "test-run-missing-evidence-only-caveats")?;
    validate_topology_expectations(input.topology, input.expected_nodes, input.expected_package_ref, &mut diagnostics)?;
    validate_node_evidence(input.topology, input.nodes, input.node_refs, input.test_run, &mut diagnostics)?;
    validate_child_expectations(
        input.expected_child_refs,
        &input.test_run.child_refs,
        input.child_artifacts,
        input.expected_child_receipts,
        &mut diagnostics,
    )?;
    validate_prod_soak_runs(input.topology_ref, input.node_refs, input.prod_soaks, &mut diagnostics)?;
    Ok(diagnostics)
}

fn validate_topology_expectations(
    topology: &ParsedTopology,
    expected_nodes: &[String],
    expected_package_ref: Option<&str>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if !expected_nodes.is_empty() {
        let actual = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
        let expected = expected_nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
        push_if(diagnostics, actual != expected, "topology-node-set-mismatch")?;
    }
    if let Some(package_ref) = expected_package_ref {
        push_if(diagnostics, topology.package_ref != package_ref, "topology-package-ref-mismatch")?;
    }
    push_if(diagnostics, topology.network.trim().is_empty(), "topology-missing-network")?;
    push_if(diagnostics, topology.caveats.is_empty(), "topology-missing-caveats")?;
    Ok(())
}

fn validate_node_evidence(
    topology: &ParsedTopology,
    nodes: &[ParsedNodeEvidence],
    node_refs: &[String],
    test_run: &ParsedTestRun,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let topology_nodes = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
    let evidence_node_count = nodes.iter().map(|node| node.node.as_str()).collect::<OrderedSet<_>>().len();
    for node in nodes {
        push_if(diagnostics, !topology_nodes.contains(node.node.as_str()), "node-evidence-outside-topology")?;
        push_if(diagnostics, node.state_root.trim().is_empty(), "node-evidence-missing-state-root")?;
        push_if(diagnostics, node.log_refs.is_empty(), "node-evidence-missing-diagnostic-log-refs")?;
        push_if(diagnostics, node.startup_ref == node.health_ref, "node-evidence-reuses-startup-health-ref")?;
    }
    push_if(diagnostics, evidence_node_count != topology.nodes.len(), "node-evidence-count-mismatch")?;
    let node_ref_set = node_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for node_ref in &test_run.node_evidence_refs {
        push_if(diagnostics, !node_ref_set.contains(node_ref.as_str()), "test-run-node-ref-not-provided")?;
    }
    push_if(
        diagnostics,
        test_run.node_evidence_refs.len() != node_refs.len(),
        "test-run-node-ref-count-mismatch",
    )?;
    Ok(())
}

fn validate_child_expectations(
    expected_child_refs: &[String],
    actual_child_refs: &[String],
    child_artifacts: &[ParsedChildReceipt],
    expected_child_receipts: &[NixosVmExpectedChildReceipt],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let mut actual = OrderedSet::new();
    for child_ref in actual_child_refs {
        if !actual.insert(child_ref.as_str()) {
            push_diagnostic(diagnostics, format!("duplicate-child-ref:{child_ref}"))?;
        }
    }
    let expected = expected_child_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for child_ref in expected_child_refs {
        push_if(diagnostics, !actual.contains(child_ref.as_str()), "expected-child-ref-missing")?;
    }
    if !expected.is_empty() {
        for child_ref in actual_child_refs {
            push_if(diagnostics, !expected.contains(child_ref.as_str()), "undeclared-child-ref-present")?;
        }
    }

    let artifacts_by_ref = child_artifacts
        .iter()
        .map(|artifact| (artifact.child_ref.as_str(), artifact))
        .collect::<OrderedMap<_, _>>();
    for artifact in child_artifacts {
        push_if(diagnostics, !actual.contains(artifact.child_ref.as_str()), "child-artifact-not-bound-by-test-run")?;
    }
    for expectation in expected_child_receipts {
        validate_expected_child_receipt(expectation)?;
        let Some(artifact) = artifacts_by_ref.get(expectation.child_ref.as_str()) else {
            push_diagnostic(diagnostics, format!("expected-child-receipt-artifact-missing:{}", expectation.child_ref))?;
            continue;
        };
        push_if(
            diagnostics,
            artifact.receipt_class != expectation.receipt_class,
            "expected-child-receipt-class-mismatch",
        )?;
        push_if(diagnostics, artifact.decision != expectation.decision, "expected-child-receipt-decision-mismatch")?;
        validate_optional_child_binding(
            diagnostics,
            &artifact.node_id,
            expectation.node_id.as_deref(),
            "expected-child-receipt-node-mismatch",
        )?;
        validate_optional_child_binding(
            diagnostics,
            &artifact.peer_id,
            expectation.peer_id.as_deref(),
            "expected-child-receipt-peer-mismatch",
        )?;
        validate_optional_child_binding(
            diagnostics,
            &artifact.operation_id,
            expectation.operation_id.as_deref(),
            "expected-child-receipt-operation-mismatch",
        )?;
    }
    Ok(())
}

fn validate_expected_child_receipt(expectation: &NixosVmExpectedChildReceipt) -> Result<()> {
    crate::preserves_rail::validate_content_ref(&expectation.child_ref)?;
    validate_text("expected child receipt class", &expectation.receipt_class)?;
    validate_text("expected child decision", &expectation.decision)?;
    if let Some(node_id) = &expectation.node_id {
        validate_text("expected child node", node_id)?;
    }
    if let Some(peer_id) = &expectation.peer_id {
        validate_text("expected child peer", peer_id)?;
    }
    if let Some(operation_id) = &expectation.operation_id {
        validate_text("expected child operation", operation_id)?;
    }
    Ok(())
}

fn validate_optional_child_binding(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    actual: &Option<String>,
    expected: Option<&str>,
    diagnostic: &'static str,
) -> Result<()> {
    if let Some(expected_value) = expected {
        push_if(diagnostics, actual.as_deref() != Some(expected_value), diagnostic)?;
    }
    Ok(())
}

fn validate_prod_soak_runs(
    topology_ref: &str,
    node_refs: &[String],
    prod_soaks: &[ParsedProdSoakRun],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    let node_ref_set = node_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for run in prod_soaks {
        push_if(diagnostics, run.decision != "pass", "prod-soak-run-not-pass")?;
        push_if(diagnostics, run.topology_ref != topology_ref, "prod-soak-topology-ref-mismatch")?;
        push_if(diagnostics, run.node_evidence_refs.is_empty(), "prod-soak-missing-node-evidence")?;
        for node_ref in &run.node_evidence_refs {
            push_if(diagnostics, !node_ref_set.contains(node_ref.as_str()), "prod-soak-node-ref-not-provided")?;
        }
        push_if(diagnostics, run.replay_status.trim().is_empty(), "prod-soak-missing-replay-status")?;
        push_if(diagnostics, run.caveats.is_empty(), "prod-soak-missing-caveats")?;
    }
    Ok(())
}
