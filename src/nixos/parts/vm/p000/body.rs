type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const NIXOS_VM_NODE_EVIDENCE_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_NODE_EVIDENCE_SCHEMA;
const NIXOS_VM_TEST_RUN_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_TEST_RUN_SCHEMA;
const NIXOS_VM_TOPOLOGY_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_TOPOLOGY_SCHEMA;
const NIXOS_VM_FAULT_DESCRIPTOR_SCHEMA: &str = "molten.testing.nixos-vm.fault-descriptor.v1";
const NIXOS_VM_FAULT_RECEIPT_SCHEMA: &str = "molten.testing.nixos-vm.fault-receipt.v1";
const NIXOS_VM_NETWORK_CONTROL_PROBE_SCHEMA: &str = "molten.testing.nixos-vm.network-control-probe.v1";
const NIXOS_VM_SHARD_RUN_SCHEMA: &str = "molten.testing.nixos-vm.shard-run.v1";
const NIXOS_VM_MULTINODE_AGGREGATE_SCHEMA: &str = "molten.testing.nixos-vm.multinode-aggregate.v1";
pub const NIXOS_VM_SCOPE_FIXTURE_METADATA: &str = "fixture-metadata";
pub const NIXOS_VM_SCOPE_EXECUTABLE_VM: &str = "executable-vm";
pub const NIXOS_VM_SCOPE_AGGREGATE_INDEX: &str = "aggregate-index";
pub const NIXOS_VM_SCOPE_DIAGNOSTIC_ONLY: &str = "diagnostic-only";

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

const MAX_VM_NODES: usize = 16;
const MAX_VM_REFS: usize = 256;
const MAX_VM_TEXT_FIELDS: usize = 128;
const NIXOS_VM_FAULT_DESCRIPTOR_MIN_DURATION_MILLIS: u64 = 1;
const _: () = assert!(MAX_VM_NODES <= 100_000);
const _: () = assert!(MAX_VM_REFS <= 100_000);
const _: () = assert!(MAX_VM_TEXT_FIELDS <= 100_000);
const _: () = assert!(NIXOS_VM_FAULT_DESCRIPTOR_MIN_DURATION_MILLIS > 0);

pub struct NixosVmTopologyInput<'a> {
    pub nodes: &'a [String],
    pub package_ref: &'a str,
    pub package_path: &'a str,
    pub network: &'a str,
    pub nix_inputs: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmNodeEvidenceInput<'a> {
    pub node: &'a str,
    pub state_root: &'a str,
    pub identity_receipt_ref: Option<&'a str>,
    pub startup_receipt_ref: &'a str,
    pub health_receipt_ref: &'a str,
    pub control_loop_receipt_ref: &'a str,
    pub heartbeat_receipt_ref: &'a str,
    pub shutdown_receipt_ref: Option<&'a str>,
    pub log_refs: &'a [String],
}

pub struct NixosVmTestRunInput<'a> {
    pub decision: &'a str,
    pub topology_ref: &'a str,
    pub scenario: &'a str,
    pub fault_profile: &'a str,
    pub node_evidence_refs: &'a [String],
    pub child_workflow_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub log_refs: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmFaultDescriptorInput<'a> {
    pub fault_id: &'a str,
    pub topology_ref: &'a str,
    pub target_node: &'a str,
    pub target_link: Option<&'a str>,
    pub fault_kind: &'a str,
    pub command_profile: &'a str,
    pub expected_outcome: &'a str,
    pub duration_millis: u64,
    pub trigger: &'a str,
    pub preflight_refs: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmFaultReceiptInput<'a> {
    pub decision: &'a str,
    pub descriptor_ref: &'a str,
    pub host_support: &'a str,
    pub pre_fault_refs: &'a [String],
    pub injection_refs: &'a [String],
    pub child_refs: &'a [String],
    pub post_fault_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub log_refs: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmNetworkControlProbeInput<'a> {
    pub backend: &'a str,
    pub target_link: &'a str,
    pub topology_ref: &'a str,
    pub host_support: &'a str,
    pub cleanup_strategy: &'a str,
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmShardRunInput<'a> {
    pub shard_id: &'a str,
    pub scenario_fixture_ref: &'a str,
    pub topology_ref: &'a str,
    pub package_ref: &'a str,
    pub evidence_scope: &'a str,
    pub node_evidence_refs: &'a [String],
    pub child_receipt_refs: &'a [String],
    pub diagnostic_log_refs: &'a [String],
    pub unavailable: bool,
    pub claimed_decision: &'a str,
    pub caveats: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmShardRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub shard_ref: String,
    pub value: IoValue,
}

pub struct NixosVmAggregateInput<'a> {
    pub topology_ref: &'a str,
    pub package_ref: &'a str,
    pub manifest_ref: &'a str,
    pub required_shard_ids: &'a [String],
    pub shard_refs: &'a [String],
    pub shard_scopes: &'a [String],
    pub denied_shard_ids: &'a [String],
    pub unavailable_as_pass_shard_ids: &'a [String],
    pub stale_child_refs: &'a [String],
    pub log_only_child_refs: &'a [String],
    pub caveats: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixosVmAggregateReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub aggregate_ref: String,
    pub value: IoValue,
}

pub fn topology_value(input: &NixosVmTopologyInput<'_>) -> Result<IoValue> {
    validate_nodes(input.nodes)?;
    validate_text_field("package ref", input.package_ref)?;
    validate_text_field("package path", input.package_path)?;
    validate_text_field("network", input.network)?;
    Ok(record("nixos-vm-topology-v1", vec![
        string(NIXOS_VM_TOPOLOGY_SCHEMA),
        record("nodes", vec![sequence(node_values(input.nodes)?)]),
        record("package", vec![record("molten-package", vec![
            record("ref", vec![string(input.package_ref)]),
            record("path", vec![string(input.package_path)]),
        ])]),
        record("network", vec![string(input.network)]),
        record("nix-inputs", vec![sequence(string_values("nix input", input.nix_inputs, MAX_VM_REFS)?)]),
        record("caveats", vec![sequence(string_values(
            "topology caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("headless-topology", "pass"),
            check_value("explicit-state-roots", "pass"),
            check_value("no-undeclared-host-state", "pass"),
        ])]),
    ]))
}

pub fn node_evidence_value(input: &NixosVmNodeEvidenceInput<'_>) -> Result<IoValue> {
    validate_text_field("node", input.node)?;
    validate_text_field("state root", input.state_root)?;
    validate_optional_ref("identity receipt", input.identity_receipt_ref)?;
    validate_content_ref(input.startup_receipt_ref)?;
    validate_content_ref(input.health_receipt_ref)?;
    validate_content_ref(input.control_loop_receipt_ref)?;
    validate_content_ref(input.heartbeat_receipt_ref)?;
    validate_optional_ref("shutdown receipt", input.shutdown_receipt_ref)?;
    validate_ref_slice("node log", input.log_refs)?;
    Ok(record("nixos-vm-node-evidence-v1", vec![
        string(NIXOS_VM_NODE_EVIDENCE_SCHEMA),
        record("node", vec![string(input.node)]),
        record("state-root", vec![string(input.state_root)]),
        record("identity-receipt", vec![optional_ref_value(input.identity_receipt_ref)]),
        record("startup-receipt", vec![string(input.startup_receipt_ref)]),
        record("health-receipt", vec![string(input.health_receipt_ref)]),
        record("control-loop-receipt", vec![string(input.control_loop_receipt_ref)]),
        record("heartbeat-receipt", vec![string(input.heartbeat_receipt_ref)]),
        record("shutdown-receipt", vec![optional_ref_value(input.shutdown_receipt_ref)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("checks", vec![sequence(vec![
            check_value("startup-receipt-bound", "pass"),
            check_value("health-receipt-bound", "pass"),
            check_value("control-loop-under-systemd", "pass"),
            check_value("logs-diagnostic-only", "pass"),
        ])]),
    ]))
}

pub fn test_run_value(input: &NixosVmTestRunInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("scenario", input.scenario)?;
    validate_text_field("fault profile", input.fault_profile)?;
    validate_ref_slice("node evidence", input.node_evidence_refs)?;
    validate_ref_slice("child workflow", input.child_workflow_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_ref_slice("log", input.log_refs)?;
    Ok(record("nixos-vm-test-run-v1", vec![
        string(NIXOS_VM_TEST_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("topology", vec![string(input.topology_ref)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-profile", vec![string(input.fault_profile)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("child-workflows", vec![sequence(ref_values(input.child_workflow_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "diagnostic",
            input.diagnostics,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "test caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("terminal-output-diagnostic-only", "pass"),
            check_value("vm-evidence-does-not-grant-authority", "pass"),
            check_value("skip-is-not-pass-evidence", "pass"),
        ])]),
    ]))
}

pub fn vm_fault_descriptor_value(input: &NixosVmFaultDescriptorInput<'_>) -> Result<IoValue> {
    validate_text_field("fault id", input.fault_id)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("fault target node", input.target_node)?;
    validate_optional_text("fault target link", input.target_link)?;
    validate_fault_kind(input.fault_kind)?;
    validate_text_field("fault command profile", input.command_profile)?;
    validate_text_field("fault expected outcome", input.expected_outcome)?;
    if input.duration_millis < NIXOS_VM_FAULT_DESCRIPTOR_MIN_DURATION_MILLIS {
        return Err(MoltenError::invalid_harness("nixos VM fault duration must be positive"));
    }
    validate_text_field("fault trigger", input.trigger)?;
    validate_ref_slice("fault preflight", input.preflight_refs)?;
    Ok(record("nixos-vm-fault-descriptor-v1", vec![
        string(NIXOS_VM_FAULT_DESCRIPTOR_SCHEMA),
        record("id", vec![string(input.fault_id)]),
        record("topology", vec![string(input.topology_ref)]),
        record("target-node", vec![string(input.target_node)]),
        record("target-link", vec![optional_text_value(input.target_link)]),
        record("fault-kind", vec![string(input.fault_kind)]),
        record("command-profile", vec![string(input.command_profile)]),
        record("expected-outcome", vec![string(input.expected_outcome)]),
        record("duration-millis", vec![crate::preserves_rail::u64_value(input.duration_millis)]),
        record("trigger", vec![string(input.trigger)]),
        record("preflight", vec![sequence(ref_values(input.preflight_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "fault caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("fault-target-explicit", "pass"),
            check_value("duration-bounded", "pass"),
            check_value("fault-receipt-evidence-only", "pass"),
        ])]),
    ]))
}
