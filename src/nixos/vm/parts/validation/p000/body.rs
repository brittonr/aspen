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
