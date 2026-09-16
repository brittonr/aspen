type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const VM_EVIDENCE_VALIDATION_SCHEMA: &str = "molten.testing.nixos-vm.evidence-validation.v1";
const VM_EVIDENCE_MANIFEST_SCHEMA: &str = "molten.testing.nixos-vm.evidence-manifest.v1";
const VM_FAULT_DESCRIPTOR_SCHEMA: &str = "molten.testing.nixos-vm.fault-descriptor.v1";
const VM_FAULT_RECEIPT_SCHEMA: &str = "molten.testing.nixos-vm.fault-receipt.v1";
const VM_FAULT_VALIDATION_SCHEMA: &str = "molten.testing.nixos-vm.fault-validation.v1";
const VM_FAULT_MIN_DURATION_MILLIS: u64 = 1;
const TOPOLOGY_ARITY: usize = 7;
const TOPOLOGY_SCHEMA_INDEX: usize = 0;
const TOPOLOGY_NODES_INDEX: usize = 1;
const TOPOLOGY_PACKAGE_INDEX: usize = 2;
const TOPOLOGY_NETWORK_INDEX: usize = 3;
const TOPOLOGY_CAVEATS_INDEX: usize = 5;
const NODE_EVIDENCE_ARITY: usize = 11;
const NODE_SCHEMA_INDEX: usize = 0;
const NODE_NAME_INDEX: usize = 1;
const NODE_STATE_ROOT_INDEX: usize = 2;
const NODE_STARTUP_INDEX: usize = 4;
const NODE_HEALTH_INDEX: usize = 5;
const NODE_CONTROL_LOOP_INDEX: usize = 6;
const NODE_HEARTBEAT_INDEX: usize = 7;
const NODE_LOGS_INDEX: usize = 9;
const TEST_RUN_ARITY: usize = 12;
const TEST_RUN_SCHEMA_INDEX: usize = 0;
const TEST_RUN_DECISION_INDEX: usize = 1;
const TEST_RUN_TOPOLOGY_INDEX: usize = 2;
const TEST_RUN_NODE_EVIDENCE_INDEX: usize = 5;
const TEST_RUN_CHILDREN_INDEX: usize = 6;
const TEST_RUN_REPLAY_INDEX: usize = 7;
const TEST_RUN_LOGS_INDEX: usize = 9;
const TEST_RUN_CAVEATS_INDEX: usize = 10;
const FAULT_DESCRIPTOR_ARITY: usize = 13;
const FAULT_DESCRIPTOR_SCHEMA_INDEX: usize = 0;
const FAULT_DESCRIPTOR_ID_INDEX: usize = 1;
const FAULT_DESCRIPTOR_TOPOLOGY_INDEX: usize = 2;
const FAULT_DESCRIPTOR_TARGET_NODE_INDEX: usize = 3;
const FAULT_DESCRIPTOR_KIND_INDEX: usize = 5;
const FAULT_DESCRIPTOR_EXPECTED_INDEX: usize = 7;
const FAULT_DESCRIPTOR_DURATION_INDEX: usize = 8;
const FAULT_DESCRIPTOR_CAVEATS_INDEX: usize = 11;
const FAULT_RECEIPT_ARITY: usize = 13;
const FAULT_RECEIPT_SCHEMA_INDEX: usize = 0;
const FAULT_RECEIPT_DECISION_INDEX: usize = 1;
const FAULT_RECEIPT_DESCRIPTOR_INDEX: usize = 2;
const FAULT_RECEIPT_HOST_SUPPORT_INDEX: usize = 3;
const FAULT_RECEIPT_PRE_INDEX: usize = 4;
const FAULT_RECEIPT_INJECTION_INDEX: usize = 5;
const FAULT_RECEIPT_CHILDREN_INDEX: usize = 6;
const FAULT_RECEIPT_POST_INDEX: usize = 7;
const FAULT_RECEIPT_REPLAY_INDEX: usize = 8;
const FAULT_RECEIPT_DIAGNOSTICS_INDEX: usize = 9;
const FAULT_RECEIPT_LOGS_INDEX: usize = 10;
const FAULT_RECEIPT_CAVEATS_INDEX: usize = 11;
const SOAK_RUN_DECISION_INDEX: usize = 1;
const SOAK_RUN_TOPOLOGY_INDEX: usize = 3;
const SOAK_RUN_NODE_EVIDENCE_INDEX: usize = 5;
const SOAK_RUN_REPLAY_INDEX: usize = 15;
const SOAK_RUN_CAVEATS_INDEX: usize = 18;
const SHARD_RUN_ARITY: usize = 15;
const SHARD_RUN_DECISION_INDEX: usize = 1;
const AGGREGATE_ARITY: usize = 11;
const AGGREGATE_DECISION_INDEX: usize = 1;
const MAX_VM_VALIDATION_ITEMS: usize = 512;
const CHILD_RECEIPT_CLASSES: &[&str] = &[
    "nixos-vm-fault-receipt-v1",
    "nixos-vm-network-control-probe-v1",
    "nixos-vm-test-run-v1",
    "nixos-vm-shard-run-v1",
    "nixos-vm-aggregate-receipt-v1",
];
const UNKNOWN_CHILD_DECISION: &str = "unknown";
const _: () = assert!(MAX_VM_VALIDATION_ITEMS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmEvidenceValidationInput<'a> {
    pub topology_value: &'a IoValue,
    pub node_evidence_values: &'a [IoValue],
    pub test_run_value: &'a IoValue,
    pub prod_soak_values: &'a [IoValue],
    pub child_artifact_values: &'a [IoValue],
    pub expected_nodes: &'a [String],
    pub expected_package_ref: Option<&'a str>,
    pub expected_child_refs: &'a [String],
    pub expected_child_receipts: &'a [NixosVmExpectedChildReceipt],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmExpectedChildReceipt {
    pub child_ref: String,
    pub receipt_class: String,
    pub decision: String,
    pub node_id: Option<String>,
    pub peer_id: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmEvidenceValidation {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub topology_ref: String,
    pub node_evidence_refs: Vec<String>,
    pub test_run_ref: String,
    pub prod_soak_refs: Vec<String>,
    pub validation_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmFaultEvidenceValidationInput<'a> {
    pub topology_value: &'a IoValue,
    pub descriptor_values: &'a [IoValue],
    pub receipt_values: &'a [IoValue],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmFaultEvidenceValidation {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub topology_ref: String,
    pub descriptor_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub validation_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEvidenceManifestEntry {
    pub path: String,
    pub kind: String,
    pub content_ref: String,
    pub diagnostic_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEvidenceManifestRequiredArtifact {
    pub kind: String,
    pub content_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEvidenceManifestInput<'a> {
    pub entries: &'a [VmEvidenceManifestEntry],
    pub required_artifacts: &'a [VmEvidenceManifestRequiredArtifact],
    pub caveats: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmEvidenceManifest {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub manifest_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTopology {
    nodes: Vec<String>,
    package_ref: String,
    network: String,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedNodeEvidence {
    node: String,
    state_root: String,
    startup_ref: String,
    health_ref: String,
    control_loop_ref: String,
    heartbeat_ref: String,
    log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTestRun {
    decision: String,
    topology_ref: String,
    node_evidence_refs: Vec<String>,
    child_refs: Vec<String>,
    replay_status: String,
    log_refs: Vec<String>,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedChildReceipt {
    child_ref: String,
    receipt_class: String,
    decision: String,
    node_id: Option<String>,
    peer_id: Option<String>,
    operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedProdSoakRun {
    decision: String,
    topology_ref: String,
    node_evidence_refs: Vec<String>,
    replay_status: String,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFaultDescriptor {
    id: String,
    topology_ref: String,
    target_node: String,
    fault_kind: String,
    expected_outcome: String,
    duration_millis: u64,
    caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFaultReceipt {
    decision: String,
    descriptor_ref: String,
    host_support: String,
    pre_fault_refs: Vec<String>,
    injection_refs: Vec<String>,
    child_refs: Vec<String>,
    post_fault_refs: Vec<String>,
    replay_status: String,
    diagnostics: Vec<String>,
    log_refs: Vec<String>,
    caveats: Vec<String>,
}

pub fn validate_nixos_vm_evidence(input: &NixosVmEvidenceValidationInput<'_>) -> Result<NixosVmEvidenceValidation> {
    let topology = parse_topology(input.topology_value)?;
    let topology_ref = crate::preserves_rail::canonical_hash(input.topology_value)?;
    let parsed_nodes = parse_node_evidence_values(input.node_evidence_values)?;
    let node_evidence_refs = canonical_refs(input.node_evidence_values)?;
    let test_run = parse_test_run(input.test_run_value)?;
    let test_run_ref = crate::preserves_rail::canonical_hash(input.test_run_value)?;
    let prod_soaks = parse_prod_soaks(input.prod_soak_values)?;
    let prod_soak_refs = canonical_refs(input.prod_soak_values)?;
    let child_artifacts = parse_child_receipts(input.child_artifact_values)?;
    let child_artifact_refs = child_artifacts.iter().map(|artifact| artifact.child_ref.clone()).collect::<Vec<_>>();
    let diagnostics = validation_diagnostics(ValidationContext {
        topology: &topology,
        topology_ref: &topology_ref,
        nodes: &parsed_nodes,
        node_refs: &node_evidence_refs,
        test_run: &test_run,
        prod_soaks: &prod_soaks,
        child_artifacts: &child_artifacts,
        expected_nodes: input.expected_nodes,
        expected_package_ref: input.expected_package_ref,
        expected_child_refs: input.expected_child_refs,
        expected_child_receipts: input.expected_child_receipts,
    })?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = vm_evidence_validation_value(ValidationValueInput {
        decision: &decision,
        diagnostics: &diagnostics,
        topology_ref: &topology_ref,
        node_evidence_refs: &node_evidence_refs,
        test_run_ref: &test_run_ref,
        prod_soak_refs: &prod_soak_refs,
        child_artifact_refs: &child_artifact_refs,
    })?;
    let validation_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(NixosVmEvidenceValidation {
        decision,
        diagnostics,
        topology_ref,
        node_evidence_refs,
        test_run_ref,
        prod_soak_refs,
        validation_ref,
        value,
    })
}

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
    diagnostics: &mut Vec<String>,
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
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let topology_nodes = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
    let mut evidence_nodes = OrderedMap::new();
    for node in nodes {
        push_if(diagnostics, !topology_nodes.contains(node.node.as_str()), "node-evidence-outside-topology")?;
        push_if(diagnostics, node.state_root.trim().is_empty(), "node-evidence-missing-state-root")?;
        push_if(diagnostics, node.log_refs.is_empty(), "node-evidence-missing-diagnostic-log-refs")?;
        push_if(diagnostics, node.startup_ref == node.health_ref, "node-evidence-reuses-startup-health-ref")?;
        evidence_nodes.insert(node.node.as_str(), node);
    }
    push_if(diagnostics, evidence_nodes.len() != topology.nodes.len(), "node-evidence-count-mismatch")?;
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
    diagnostics: &mut Vec<String>,
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
    diagnostics: &mut Vec<String>,
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
    diagnostics: &mut Vec<String>,
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

fn fault_validation_diagnostics(
    topology: &ParsedTopology,
    topology_ref: &str,
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
    receipts: &[ParsedFaultReceipt],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, descriptors.is_empty(), "vm-fault-missing-descriptors")?;
    push_if(&mut diagnostics, receipts.is_empty(), "vm-fault-missing-receipts")?;
    let topology_nodes = topology.nodes.iter().map(String::as_str).collect::<OrderedSet<_>>();
    let descriptor_ref_set = descriptor_refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    for descriptor in descriptors {
        push_if(&mut diagnostics, descriptor.id.trim().is_empty(), "vm-fault-descriptor-missing-id")?;
        push_if(&mut diagnostics, descriptor.topology_ref != topology_ref, "vm-fault-descriptor-topology-mismatch")?;
        push_if(
            &mut diagnostics,
            !topology_nodes.contains(descriptor.target_node.as_str()),
            "vm-fault-descriptor-target-outside-topology",
        )?;
        push_if(
            &mut diagnostics,
            descriptor.duration_millis < VM_FAULT_MIN_DURATION_MILLIS,
            "vm-fault-descriptor-unbounded-duration",
        )?;
        push_if(&mut diagnostics, descriptor.caveats.is_empty(), "vm-fault-descriptor-missing-caveats")?;
    }
    for receipt in receipts {
        push_if(
            &mut diagnostics,
            !descriptor_ref_set.contains(receipt.descriptor_ref.as_str()),
            "vm-fault-receipt-descriptor-missing",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.host_support != "supported",
            "vm-fault-unavailable-cannot-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.pre_fault_refs.is_empty(),
            "vm-fault-pass-missing-pre-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.injection_refs.is_empty(),
            "vm-fault-pass-missing-injection-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.child_refs.is_empty(),
            "vm-fault-log-only-pass",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision == "pass" && receipt.post_fault_refs.is_empty(),
            "vm-fault-pass-missing-post-ref",
        )?;
        push_if(
            &mut diagnostics,
            receipt.decision != "pass" && receipt.diagnostics.is_empty(),
            "vm-fault-deny-missing-diagnostic",
        )?;
        push_if(&mut diagnostics, receipt.log_refs.is_empty(), "vm-fault-missing-log-ref")?;
        push_if(&mut diagnostics, receipt.replay_status.trim().is_empty(), "vm-fault-missing-replay-status")?;
        push_if(&mut diagnostics, receipt.caveats.is_empty(), "vm-fault-missing-caveats")?;
        validate_fault_expected_outcome(receipt, descriptors, descriptor_refs, &mut diagnostics)?;
    }
    Ok(diagnostics)
}

fn validate_fault_expected_outcome(
    receipt: &ParsedFaultReceipt,
    descriptors: &[ParsedFaultDescriptor],
    descriptor_refs: &[String],
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let Some((descriptor, _)) = descriptors
        .iter()
        .zip(descriptor_refs.iter())
        .find(|(_, descriptor_ref)| descriptor_ref.as_str() == receipt.descriptor_ref)
    else {
        return Ok(());
    };
    push_if(
        diagnostics,
        descriptor.expected_outcome == "unavailable" && receipt.decision == "pass",
        "vm-fault-unavailable-expected-cannot-pass",
    )?;
    push_if(
        diagnostics,
        descriptor.fault_kind == "log-only-pass" && receipt.decision == "pass",
        "vm-fault-log-only-pass",
    )?;
    Ok(())
}

fn parse_topology(value: &IoValue) -> Result<ParsedTopology> {
    let topology = simple_record(value, "nixos-vm-topology-v1", TOPOLOGY_ARITY)?;
    require_schema(&topology[TOPOLOGY_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TOPOLOGY_SCHEMA, "topology")?;
    let nodes = node_names(&topology[TOPOLOGY_NODES_INDEX])?;
    let package = simple_field_record(&topology[TOPOLOGY_PACKAGE_INDEX], "package", "topology package")?;
    let package_record = value_to_iovalue(&package[0]);
    let molten_package = simple_record(&package_record, "molten-package", 2)?;
    let package_ref = required_record_string(&molten_package[0], "ref", "topology package ref")?;
    let network = required_record_string(&topology[TOPOLOGY_NETWORK_INDEX], "network", "topology network")?;
    let caveats = required_string_sequence_record(&topology[TOPOLOGY_CAVEATS_INDEX], "caveats", "topology caveats")?;
    Ok(ParsedTopology {
        nodes,
        package_ref,
        network,
        caveats,
    })
}

fn parse_node_evidence_values(values: &[IoValue]) -> Result<Vec<ParsedNodeEvidence>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM node evidence count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_node_evidence(value)?);
    }
    Ok(output)
}

fn parse_node_evidence(value: &IoValue) -> Result<ParsedNodeEvidence> {
    let node = simple_record(value, "nixos-vm-node-evidence-v1", NODE_EVIDENCE_ARITY)?;
    require_schema(&node[NODE_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_NODE_EVIDENCE_SCHEMA, "node evidence")?;
    Ok(ParsedNodeEvidence {
        node: required_record_string(&node[NODE_NAME_INDEX], "node", "node evidence node")?,
        state_root: required_record_string(&node[NODE_STATE_ROOT_INDEX], "state-root", "node evidence state root")?,
        startup_ref: required_record_ref(&node[NODE_STARTUP_INDEX], "startup-receipt", "node startup")?,
        health_ref: required_record_ref(&node[NODE_HEALTH_INDEX], "health-receipt", "node health")?,
        control_loop_ref: required_record_ref(
            &node[NODE_CONTROL_LOOP_INDEX],
            "control-loop-receipt",
            "node control loop",
        )?,
        heartbeat_ref: required_record_ref(&node[NODE_HEARTBEAT_INDEX], "heartbeat-receipt", "node heartbeat")?,
        log_refs: required_ref_sequence_record(&node[NODE_LOGS_INDEX], "logs", "node logs")?,
    })
}

fn parse_test_run(value: &IoValue) -> Result<ParsedTestRun> {
    let run = simple_record(value, "nixos-vm-test-run-v1", TEST_RUN_ARITY)?;
    require_schema(&run[TEST_RUN_SCHEMA_INDEX], crate::preserves_rail::NIXOS_VM_TEST_RUN_SCHEMA, "test run")?;
    Ok(ParsedTestRun {
        decision: required_record_string(&run[TEST_RUN_DECISION_INDEX], "decision", "test run decision")?,
        topology_ref: required_record_ref(&run[TEST_RUN_TOPOLOGY_INDEX], "topology", "test run topology")?,
        node_evidence_refs: required_ref_sequence_record(
            &run[TEST_RUN_NODE_EVIDENCE_INDEX],
            "node-evidence",
            "test run nodes",
        )?,
        child_refs: required_ref_sequence_record(
            &run[TEST_RUN_CHILDREN_INDEX],
            "child-workflows",
            "test run children",
        )?,
        replay_status: required_record_string(&run[TEST_RUN_REPLAY_INDEX], "replay-status", "test run replay status")?,
        log_refs: required_ref_sequence_record(&run[TEST_RUN_LOGS_INDEX], "logs", "test run logs")?,
        caveats: required_string_sequence_record(&run[TEST_RUN_CAVEATS_INDEX], "caveats", "test run caveats")?,
    })
}

fn parse_child_receipts(values: &[IoValue]) -> Result<Vec<ParsedChildReceipt>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM child artifact count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut receipts = Vec::with_capacity(values.len());
    let mut refs = OrderedSet::new();
    for value in values {
        let receipt = parse_child_receipt(value)?;
        if !refs.insert(receipt.child_ref.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate VM child artifact ref {}", receipt.child_ref)));
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn parse_child_receipt(value: &IoValue) -> Result<ParsedChildReceipt> {
    let child_ref = crate::preserves_rail::canonical_hash(value)?;
    let receipt_class = child_receipt_class(value).to_string();
    let decision = child_receipt_decision(value).unwrap_or_else(|| UNKNOWN_CHILD_DECISION.to_string());
    Ok(ParsedChildReceipt {
        child_ref,
        receipt_class,
        decision,
        node_id: None,
        peer_id: None,
        operation_id: child_receipt_operation_id(value),
    })
}

fn child_receipt_class(value: &IoValue) -> &'static str {
    for record_label in CHILD_RECEIPT_CLASSES {
        if value.collect_simple_record(record_label, None).is_some() {
            return record_label;
        }
    }
    crate::ledger::artifact_kind(value)
}

fn child_receipt_decision(value: &IoValue) -> Option<String> {
    if let Ok(run) = simple_record(value, "nixos-vm-test-run-v1", TEST_RUN_ARITY) {
        return required_record_string(&run[TEST_RUN_DECISION_INDEX], "decision", "test run decision").ok();
    }
    if let Ok(receipt) = simple_record(value, "nixos-vm-fault-receipt-v1", FAULT_RECEIPT_ARITY) {
        return required_record_string(&receipt[FAULT_RECEIPT_DECISION_INDEX], "decision", "fault receipt decision")
            .ok();
    }
    if let Ok(shard) = simple_record(value, "nixos-vm-shard-run-v1", SHARD_RUN_ARITY) {
        return required_record_string(&shard[SHARD_RUN_DECISION_INDEX], "decision", "shard run decision").ok();
    }
    if let Ok(aggregate) = simple_record(value, "nixos-vm-multinode-aggregate-v1", AGGREGATE_ARITY) {
        return required_record_string(&aggregate[AGGREGATE_DECISION_INDEX], "decision", "aggregate decision").ok();
    }
    None
}

fn child_receipt_operation_id(value: &IoValue) -> Option<String> {
    if let Ok(receipt) = simple_record(value, "nixos-vm-fault-receipt-v1", FAULT_RECEIPT_ARITY) {
        return required_record_string(&receipt[FAULT_RECEIPT_DESCRIPTOR_INDEX], "descriptor", "fault descriptor").ok();
    }
    None
}

fn parse_fault_descriptors(values: &[IoValue]) -> Result<Vec<ParsedFaultDescriptor>> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM fault descriptor count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(parse_fault_descriptor(value)?);
    }
    Ok(output)
}

fn parse_fault_descriptor(value: &IoValue) -> Result<ParsedFaultDescriptor> {
    let descriptor = simple_record(value, "nixos-vm-fault-descriptor-v1", FAULT_DESCRIPTOR_ARITY)?;
    require_schema(&descriptor[FAULT_DESCRIPTOR_SCHEMA_INDEX], VM_FAULT_DESCRIPTOR_SCHEMA, "fault descriptor")?;
    Ok(ParsedFaultDescriptor {
        id: required_record_string(&descriptor[FAULT_DESCRIPTOR_ID_INDEX], "id", "fault descriptor id")?,
        topology_ref: required_record_ref(
            &descriptor[FAULT_DESCRIPTOR_TOPOLOGY_INDEX],
            "topology",
            "fault descriptor topology",
        )?,
        target_node: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_TARGET_NODE_INDEX],
            "target-node",
            "fault descriptor target node",
        )?,
        fault_kind: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_KIND_INDEX],
            "fault-kind",
            "fault descriptor kind",
        )?,
        expected_outcome: required_record_string(
            &descriptor[FAULT_DESCRIPTOR_EXPECTED_INDEX],
            "expected-outcome",
            "fault descriptor expected outcome",
        )?,
        duration_millis: required_record_u64(
            &descriptor[FAULT_DESCRIPTOR_DURATION_INDEX],
            "duration-millis",
            "fault descriptor duration",
        )?,
        caveats: required_string_sequence_record(
            &descriptor[FAULT_DESCRIPTOR_CAVEATS_INDEX],
            "caveats",
            "fault descriptor caveats",
        )?,
    })
}

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
    let mut entries_by_ref = OrderedMap::new();
    for entry in entries {
        if !semantic_artifacts.insert((entry.kind.as_str(), entry.content_ref.as_str())) {
            push_diagnostic(
                &mut diagnostics,
                format!("duplicate-semantic-artifact:{}:{}", entry.kind, entry.content_ref),
            )?;
        }
        entries_by_ref.insert(entry.content_ref.as_str(), entry);
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

fn required_record_ref(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let reference = required_record_string(value, label, context)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
}

fn required_record_string(value: &Value<IoValue>, label: &str, context: &str) -> Result<String> {
    let record = simple_field_record(value, label, context)?;
    required_string(&record[0], context)
}

fn required_record_u64(value: &Value<IoValue>, label: &str, context: &str) -> Result<u64> {
    let record = simple_field_record(value, label, context)?;
    record[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {context}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {context}: {error}")))
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unexpected {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_field_record<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    context: &str,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {context}")))
}

fn required_string(value: &Value<IoValue>, context: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {context}")))
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} ref count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            refs.len()
        )));
    }
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn validate_strings(label: &str, values: &[String]) -> Result<()> {
    if values.len() > MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "VM {label} count {} exceeds bound {MAX_VM_VALIDATION_ITEMS}",
            values.len()
        )));
    }
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("VM {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported VM decision {other}; expected pass or deny"))),
    }
}

fn push_if(diagnostics: &mut Vec<String>, condition: bool, diagnostic: &'static str) -> Result<()> {
    if condition {
        push_diagnostic(diagnostics, diagnostic.to_string())?;
    }
    Ok(())
}

fn push_diagnostic(diagnostics: &mut Vec<String>, diagnostic: String) -> Result<()> {
    validate_text("diagnostic", &diagnostic)?;
    if diagnostics.len() >= MAX_VM_VALIDATION_ITEMS {
        return Err(MoltenError::invalid_harness("VM validation diagnostics exceeded bound"));
    }
    diagnostics.push(diagnostic);
    Ok(())
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn check_value(name: &'static str, state: &'static str) -> IoValue {
    record("check", vec![string(name), string(state)])
}

fn status(is_passing: bool) -> &'static str {
    if is_passing { "pass" } else { "deny" }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_A: &str = "node_a";
    const NODE_B: &str = "node_b";
    const NETWORK: &str = "nixos-test-private";
    const STATE_ROOT: &str = "/var/lib/molten";
    const SCENARIO: &str = "phase2-live-control-service-job-restart";
    const FAULT_DURATION_MILLIS: u64 = 1000;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn topology() -> IoValue {
        crate::nixos_vm::topology_value(&crate::nixos_vm::NixosVmTopologyInput {
            nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            package_ref: &local_ref("package"),
            package_path: "/nix/store/example-molten",
            network: NETWORK,
            nix_inputs: &["source:locked".to_string()],
            caveats: &["vm evidence is platform integration evidence only".to_string()],
        })
        .expect("topology value")
    }

    fn node_evidence(node: &str, label: &str) -> IoValue {
        crate::nixos_vm::node_evidence_value(&crate::nixos_vm::NixosVmNodeEvidenceInput {
            node,
            state_root: STATE_ROOT,
            identity_receipt_ref: Some(&local_ref(&format!("{label}-identity"))),
            startup_receipt_ref: &local_ref(&format!("{label}-startup")),
            health_receipt_ref: &local_ref(&format!("{label}-health")),
            control_loop_receipt_ref: &local_ref(&format!("{label}-loop")),
            heartbeat_receipt_ref: &local_ref(&format!("{label}-heartbeat")),
            shutdown_receipt_ref: Some(&local_ref(&format!("{label}-shutdown"))),
            log_refs: &[local_ref(&format!("{label}-log"))],
        })
        .expect("node evidence value")
    }

    fn test_run(topology_ref: &str, node_refs: &[String], child_refs: &[String], decision: &str) -> IoValue {
        crate::nixos_vm::test_run_value(&crate::nixos_vm::NixosVmTestRunInput {
            decision,
            topology_ref,
            scenario: SCENARIO,
            fault_profile: "none",
            node_evidence_refs: node_refs,
            child_workflow_refs: child_refs,
            replay_status: "non-replayable-vm-observations",
            diagnostics: &[],
            log_refs: &[local_ref("vm-log")],
            caveats: &["vm evidence does not grant authority or policy trust".to_string()],
        })
        .expect("test run value")
    }

    fn fixture(decision: &str) -> (IoValue, Vec<IoValue>, IoValue, Vec<String>) {
        let topology = topology();
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let nodes = vec![node_evidence(NODE_A, "a"), node_evidence(NODE_B, "b")];
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let child_refs = vec![local_ref("protocol"), local_ref("job"), local_ref("coordination")];
        let run = test_run(&topology_ref, &node_refs, &child_refs, decision);
        (topology, nodes, run, child_refs)
    }

    #[test]
    fn passing_vm_evidence_validates_semantically() {
        let (topology, nodes, run, child_refs) = fixture("pass");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
            expected_child_receipts: &[],
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "pass");
        assert!(validation.diagnostics.is_empty());
    }

    #[test]
    fn marker_only_or_wrong_topology_evidence_denies() {
        let (topology, nodes, _, child_refs) = fixture("pass");
        let wrong_topology_ref = local_ref("wrong-topology");
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let run = test_run(&wrong_topology_ref, &node_refs, &child_refs, "pass");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
            expected_child_receipts: &[],
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "test-run-topology-ref-mismatch"));
    }

    #[test]
    fn deny_receipt_cannot_be_overridden_by_logs() {
        let (topology, nodes, run, child_refs) = fixture("deny");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
            expected_child_receipts: &[],
        })
        .expect("VM validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "vm-test-run-not-pass"));
    }

    fn shard_child_artifact(topology_ref: &str, node_refs: &[String]) -> crate::nixos_vm::NixosVmShardRunReceipt {
        crate::nixos_vm::evaluate_vm_shard_run(&crate::nixos_vm::NixosVmShardRunInput {
            shard_id: "live-control",
            scenario_fixture_ref: &local_ref("scenario-fixture"),
            topology_ref,
            package_ref: &local_ref("package"),
            evidence_scope: crate::nixos_vm::NIXOS_VM_SCOPE_EXECUTABLE_VM,
            node_evidence_refs: node_refs,
            child_receipt_refs: &[local_ref("operation-receipt")],
            diagnostic_log_refs: &[local_ref("shard-log")],
            unavailable: false,
            claimed_decision: "pass",
            caveats: &["VM shard evidence is bounded to declared child refs".to_string()],
        })
        .expect("shard child artifact")
    }

    #[test]
    fn expected_child_receipt_binding_passes_for_declared_artifact() {
        let (topology, nodes, _, _) = fixture("pass");
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let shard = shard_child_artifact(&topology_ref, &node_refs);
        let child_refs = vec![shard.shard_ref.clone()];
        let run = test_run(&topology_ref, &node_refs, &child_refs, "pass");
        let expected_child_receipts = vec![NixosVmExpectedChildReceipt {
            child_ref: shard.shard_ref.clone(),
            receipt_class: "nixos-vm-shard-run-v1".to_string(),
            decision: "pass".to_string(),
            node_id: None,
            peer_id: None,
            operation_id: None,
        }];
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[shard.value],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
            expected_child_receipts: &expected_child_receipts,
        })
        .expect("VM child receipt validation");
        assert_eq!(validation.decision, "pass");
        assert!(validation.diagnostics.is_empty());
    }

    #[test]
    fn duplicate_or_undeclared_child_refs_deny_vm_validation() {
        let (topology, nodes, _, _) = fixture("pass");
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let shard = shard_child_artifact(&topology_ref, &node_refs);
        let child_refs = vec![shard.shard_ref.clone(), shard.shard_ref.clone()];
        let expected_child_refs = vec![local_ref("different-child")];
        let run = test_run(&topology_ref, &node_refs, &child_refs, "pass");
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[shard.value],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &expected_child_refs,
            expected_child_receipts: &[],
        })
        .expect("VM child receipt validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("duplicate-child-ref:")));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "undeclared-child-ref-present"));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "expected-child-ref-missing"));
    }

    #[test]
    fn mismatched_child_receipt_semantics_deny_vm_validation() {
        let (topology, nodes, _, _) = fixture("pass");
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let node_refs = canonical_refs(&nodes).expect("node refs");
        let shard = shard_child_artifact(&topology_ref, &node_refs);
        let child_refs = vec![shard.shard_ref.clone()];
        let run = test_run(&topology_ref, &node_refs, &child_refs, "pass");
        let expected_child_receipts = vec![NixosVmExpectedChildReceipt {
            child_ref: shard.shard_ref.clone(),
            receipt_class: "nixos-vm-fault-receipt-v1".to_string(),
            decision: "deny".to_string(),
            node_id: Some(NODE_A.to_string()),
            peer_id: None,
            operation_id: None,
        }];
        let validation = validate_nixos_vm_evidence(&NixosVmEvidenceValidationInput {
            topology_value: &topology,
            node_evidence_values: &nodes,
            test_run_value: &run,
            prod_soak_values: &[],
            child_artifact_values: &[shard.value],
            expected_nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            expected_package_ref: None,
            expected_child_refs: &child_refs,
            expected_child_receipts: &expected_child_receipts,
        })
        .expect("VM child receipt validation");
        assert_eq!(validation.decision, "deny");
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "expected-child-receipt-class-mismatch")
        );
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "expected-child-receipt-decision-mismatch")
        );
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "expected-child-receipt-node-mismatch"));
    }

    #[test]
    fn manifest_binds_authoritative_and_diagnostic_artifacts() {
        let entries = vec![
            VmEvidenceManifestEntry {
                path: "topology.preserves".to_string(),
                kind: "nixos-vm-topology".to_string(),
                content_ref: local_ref("topology"),
                diagnostic_only: false,
            },
            VmEvidenceManifestEntry {
                path: "run.txt".to_string(),
                kind: "log".to_string(),
                content_ref: local_ref("run-log"),
                diagnostic_only: true,
            },
        ];
        let manifest =
            vm_evidence_manifest_value(&entries, &["logs are diagnostic only".to_string()]).expect("manifest value");
        let rendered = crate::preserves_rail::to_text(&manifest).expect("render manifest");
        assert!(rendered.contains("nixos-vm-evidence-manifest-v1"));
        assert!(rendered.contains("diagnostic-only #t"));
    }

    #[test]
    fn duplicate_manifest_path_is_rejected() {
        let duplicate = VmEvidenceManifestEntry {
            path: "topology.preserves".to_string(),
            kind: "nixos-vm-topology".to_string(),
            content_ref: local_ref("topology"),
            diagnostic_only: false,
        };
        let error = vm_evidence_manifest_value(&[duplicate.clone(), duplicate], &[])
            .expect_err("duplicate manifest path must fail");
        assert!(error.to_string().contains("duplicate VM evidence manifest path"));
    }

    #[test]
    fn manifest_closure_accepts_required_authoritative_artifacts() {
        let topology_ref = local_ref("topology-manifest-closure");
        let entries = vec![
            VmEvidenceManifestEntry {
                path: "topology.preserves".to_string(),
                kind: "nixos-vm-topology".to_string(),
                content_ref: topology_ref.clone(),
                diagnostic_only: false,
            },
            VmEvidenceManifestEntry {
                path: "run.log".to_string(),
                kind: "log".to_string(),
                content_ref: local_ref("closure-log"),
                diagnostic_only: true,
            },
        ];
        let required_artifacts = vec![VmEvidenceManifestRequiredArtifact {
            kind: "nixos-vm-topology".to_string(),
            content_ref: topology_ref,
        }];
        let manifest = build_vm_evidence_manifest(&VmEvidenceManifestInput {
            entries: &entries,
            required_artifacts: &required_artifacts,
            caveats: &["manifest remains evidence-only".to_string()],
        })
        .expect("manifest closure");
        assert_eq!(manifest.decision, "pass");
        assert!(manifest.diagnostics.is_empty());
        assert!(crate::preserves_rail::validate_content_ref(&manifest.manifest_ref).is_ok());
    }

    #[test]
    fn manifest_closure_denies_missing_wrong_or_log_only_artifacts() {
        let topology_ref = local_ref("topology-required-as-log-only");
        let entries = vec![
            VmEvidenceManifestEntry {
                path: "topology.log".to_string(),
                kind: "log".to_string(),
                content_ref: topology_ref.clone(),
                diagnostic_only: true,
            },
            VmEvidenceManifestEntry {
                path: "duplicate-a.preserves".to_string(),
                kind: "nixos-vm-node-evidence".to_string(),
                content_ref: local_ref("duplicate-semantic"),
                diagnostic_only: false,
            },
            VmEvidenceManifestEntry {
                path: "duplicate-b.preserves".to_string(),
                kind: "nixos-vm-node-evidence".to_string(),
                content_ref: local_ref("duplicate-semantic"),
                diagnostic_only: false,
            },
        ];
        let required_artifacts = vec![
            VmEvidenceManifestRequiredArtifact {
                kind: "nixos-vm-topology".to_string(),
                content_ref: topology_ref,
            },
            VmEvidenceManifestRequiredArtifact {
                kind: "nixos-vm-test-run".to_string(),
                content_ref: local_ref("missing-test-run"),
            },
        ];
        let manifest = build_vm_evidence_manifest(&VmEvidenceManifestInput {
            entries: &entries,
            required_artifacts: &required_artifacts,
            caveats: &["manifest remains evidence-only".to_string()],
        })
        .expect("manifest closure");
        assert_eq!(manifest.decision, "deny");
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic == "required-artifact-kind-mismatch"));
        assert!(
            manifest
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "required-artifact-only-present-as-diagnostic")
        );
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("required-artifact-missing:")));
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("duplicate-semantic-artifact:")));
    }

    fn fault_descriptor(topology_ref: &str, kind: &str, expected: &str, target_node: &str) -> IoValue {
        crate::nixos_vm::vm_fault_descriptor_value(&crate::nixos_vm::NixosVmFaultDescriptorInput {
            fault_id: "network-partition-node-a-node-b",
            topology_ref,
            target_node,
            target_link: Some("node_a->node_b"),
            fault_kind: kind,
            command_profile: "nixos-test-driver",
            expected_outcome: expected,
            duration_millis: FAULT_DURATION_MILLIS,
            trigger: "during-live-workflow-send",
            preflight_refs: &[local_ref("preflight")],
            caveats: &["VM fault evidence is platform evidence only".to_string()],
        })
        .expect("fault descriptor")
    }

    fn fault_receipt(
        descriptor_ref: &str,
        decision: &str,
        host_support: &str,
        child_refs: &[String],
        diagnostics: &[String],
    ) -> IoValue {
        crate::nixos_vm::vm_fault_receipt_value(&crate::nixos_vm::NixosVmFaultReceiptInput {
            decision,
            descriptor_ref,
            host_support,
            pre_fault_refs: &[local_ref("pre-state")],
            injection_refs: &[local_ref("tc-netem-command")],
            child_refs,
            post_fault_refs: &[local_ref("post-state")],
            replay_status: "bounded-vm-fault-observation",
            diagnostics,
            log_refs: &[local_ref("fault-log")],
            caveats: &["fault receipts do not grant authority".to_string()],
        })
        .expect("fault receipt")
    }

    #[test]
    fn vm_fault_evidence_validates_executable_partition_receipt() {
        // r[verify molten.testing.nixos_vm_fault_injection.fault_descriptors]
        // r[verify molten.testing.nixos_vm_fault_injection.network_faults]
        // r[verify molten.testing.nixos_vm_fault_injection.fault_receipts]
        let topology = topology();
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let descriptor = fault_descriptor(&topology_ref, "network-partition", "idempotent-recovery", NODE_A);
        let descriptor_ref = crate::preserves_rail::canonical_hash(&descriptor).expect("descriptor ref");
        let receipt = fault_receipt(&descriptor_ref, "pass", "supported", &[local_ref("workflow-child")], &[]);

        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceValidationInput {
            topology_value: &topology,
            descriptor_values: &[descriptor],
            receipt_values: &[receipt],
        })
        .expect("fault validation");

        assert_eq!(validation.decision, "pass");
        assert!(validation.diagnostics.is_empty());
    }

    #[test]
    fn vm_fault_validation_rejects_unavailable_log_only_and_wrong_topology() {
        // r[verify molten.testing.nixos_vm_fault_injection.unavailable_boundary]
        // r[verify molten.testing.nixos_vm_fault_injection.negative_fixtures]
        let topology = topology();
        let wrong_topology_ref = local_ref("wrong-topology");
        let descriptor = fault_descriptor(&wrong_topology_ref, "log-only-pass", "unavailable", NODE_A);
        let descriptor_ref = crate::preserves_rail::canonical_hash(&descriptor).expect("descriptor ref");
        let receipt =
            fault_receipt(&descriptor_ref, "pass", "unavailable", &[], &["host feature unavailable".to_string()]);

        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceValidationInput {
            topology_value: &topology,
            descriptor_values: &[descriptor],
            receipt_values: &[receipt],
        })
        .expect("fault validation");

        assert_eq!(validation.decision, "deny");
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vm-fault-descriptor-topology-mismatch")
        );
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "vm-fault-unavailable-cannot-pass"));
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "vm-fault-log-only-pass"));
    }

    #[test]
    fn vm_fault_validation_covers_restart_and_storage_denials() {
        // r[verify molten.testing.nixos_vm_fault_injection.restart_windows]
        // r[verify molten.testing.nixos_vm_fault_injection.storage_state_faults]
        let topology = topology();
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let restart = fault_descriptor(&topology_ref, "duplicate-send-after-restart", "idempotent-recovery", NODE_A);
        let storage =
            fault_descriptor(&topology_ref, "permission-denied-state-root", "deny-before-side-effects", NODE_B);
        let restart_ref = crate::preserves_rail::canonical_hash(&restart).expect("restart ref");
        let storage_ref = crate::preserves_rail::canonical_hash(&storage).expect("storage ref");
        let restart_receipt = fault_receipt(&restart_ref, "pass", "supported", &[local_ref("duplicate-replay")], &[]);
        let storage_receipt =
            fault_receipt(&storage_ref, "deny", "supported", &[], &["permission denied before mutation".to_string()]);

        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceValidationInput {
            topology_value: &topology,
            descriptor_values: &[restart, storage],
            receipt_values: &[restart_receipt, storage_receipt],
        })
        .expect("fault validation");

        assert_eq!(validation.decision, "pass");
        assert!(validation.diagnostics.is_empty());
    }
}
