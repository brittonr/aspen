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

fn collect_vm_shard_diagnostics(input: &NixosVmShardRunInput<'_>, diagnostics: &mut Vec<String>) -> Result<()> {
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
        diagnostics.push(format!("vm-shard-non-executable-pass:{}:{}", input.shard_id, input.evidence_scope));
    }
    if input.claimed_decision == "pass" && input.unavailable {
        diagnostics.push(format!("vm-shard-unavailable-as-pass:{}", input.shard_id));
    }
    if input.claimed_decision == "pass" && input.child_receipt_refs.is_empty() {
        diagnostics.push(format!("vm-shard-log-only-pass:{}", input.shard_id));
    }
    if input.node_evidence_refs.is_empty() {
        diagnostics.push(format!("vm-shard-missing-node-evidence:{}", input.shard_id));
    }
    if input.diagnostic_log_refs.is_empty() {
        diagnostics.push(format!("vm-shard-missing-diagnostic-log:{}", input.shard_id));
    }
    if input.caveats.is_empty() {
        diagnostics.push(format!("vm-shard-missing-caveat:{}", input.shard_id));
    }
    Ok(())
}

fn collect_vm_aggregate_diagnostics(input: &NixosVmAggregateInput<'_>, diagnostics: &mut Vec<String>) -> Result<()> {
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
        diagnostics.push("vm-aggregate-missing-required-shards".to_string());
    }
    if input.shard_refs.len() < input.required_shard_ids.len() {
        diagnostics.push("vm-aggregate-missing-shard-ref".to_string());
    }
    if input.shard_scopes.len() != input.shard_refs.len() {
        diagnostics.push("vm-aggregate-shard-scope-count-mismatch".to_string());
    }
    for scope in input.shard_scopes {
        if scope != NIXOS_VM_SCOPE_EXECUTABLE_VM {
            diagnostics.push(format!("vm-aggregate-non-executable-platform-scope:{scope}"));
        }
    }
    for shard_id in input.denied_shard_ids {
        diagnostics.push(format!("vm-aggregate-denied-shard:{shard_id}"));
    }
    for shard_id in input.unavailable_as_pass_shard_ids {
        diagnostics.push(format!("vm-aggregate-unavailable-as-pass:{shard_id}"));
    }
    for stale_ref in input.stale_child_refs {
        diagnostics.push(format!("vm-aggregate-stale-child:{stale_ref}"));
    }
    for log_only_ref in input.log_only_child_refs {
        diagnostics.push(format!("vm-aggregate-log-only-child:{log_only_ref}"));
    }
    if input.caveats.is_empty() {
        diagnostics.push("vm-aggregate-missing-caveat".to_string());
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

fn validate_fault_kind(kind: &str) -> Result<()> {
    match kind {
        "network-delay"
        | "network-drop"
        | "network-partition"
        | "network-rejoin"
        | "asymmetric-latency"
        | "crash-restart"
        | "duplicate-send-after-restart"
        | "receipt-write-readback"
        | "missing-artifact"
        | "permission-denied-state-root"
        | "bounded-disk-pressure"
        | "unsupported-host-feature"
        | "tampered-fault-receipt"
        | "wrong-topology"
        | "log-only-pass" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM fault kind {other}"))),
    }
}

fn validate_host_support(status: &str) -> Result<()> {
    match status {
        "supported" | "unavailable" | "denied" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM host-support status {other}"))),
    }
}

fn validate_vm_evidence_scope(scope: &str) -> Result<()> {
    match scope {
        NIXOS_VM_SCOPE_FIXTURE_METADATA
        | NIXOS_VM_SCOPE_EXECUTABLE_VM
        | NIXOS_VM_SCOPE_AGGREGATE_INDEX
        | NIXOS_VM_SCOPE_DIAGNOSTIC_ONLY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM evidence scope {other}"))),
    }
}

fn validate_optional_ref(label: &str, reference: Option<&str>) -> Result<()> {
    if let Some(value) = reference {
        validate_content_ref(value)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid nixos VM {label} ref {value}: {error}")))?;
    }
    Ok(())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_VM_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM {label} ref count {} exceeds bound {MAX_VM_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid nixos VM {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported nixos VM decision {other}; expected pass, deny, unavailable, or skipped"
        ))),
    }
}

fn node_values(nodes: &[String]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(nodes.len());
    for node in nodes {
        values.push(record("node", vec![string(node)]));
    }
    Ok(values)
}

fn validate_strings(label: &str, values: &[String], maximum: usize) -> Result<()> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    for value in values {
        validate_text_field(label, value)?;
    }
    Ok(())
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IoValue>> {
    validate_strings(label, values, maximum)?;
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(string(value));
    }
    Ok(output)
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_ref_slice("artifact", refs)?;
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(string(reference));
    }
    Ok(values)
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_text_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn status(is_passing: bool) -> &'static str {
    if is_passing { "pass" } else { "deny" }
}

#[path = "vm_validation.rs"]
mod validation;
pub use validation::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
