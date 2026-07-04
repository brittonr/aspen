use super::*;
use crate::preserves_rail::*;

fn local_ref(name: &str) -> String {
    content_ref_from_bytes(name.as_bytes())
}

#[test]
fn vm_test_run_receipts_bind_topology_and_nodes() {
    let topology = topology_value(&NixosVmTopologyInput {
        nodes: &["node_a".to_string(), "node_b".to_string()],
        package_ref: "store:/nix/store/example-molten",
        package_path: "/nix/store/example-molten",
        network: "nixos-test-private",
        nix_inputs: &["self:locked".to_string()],
        caveats: &["vm evidence is platform evidence only".to_string()],
    })
    .expect("topology");
    let topology_ref = canonical_hash(&topology).expect("topology ref");
    let startup_ref = local_ref("startup");
    let health_ref = local_ref("health");
    let loop_ref = local_ref("loop");
    let heartbeat_ref = local_ref("heartbeat");
    let node = node_evidence_value(&NixosVmNodeEvidenceInput {
        node: "node_a",
        state_root: "/var/lib/molten",
        identity_receipt_ref: None,
        startup_receipt_ref: &startup_ref,
        health_receipt_ref: &health_ref,
        control_loop_receipt_ref: &loop_ref,
        heartbeat_receipt_ref: &heartbeat_ref,
        shutdown_receipt_ref: None,
        log_refs: &[local_ref("log")],
    })
    .expect("node evidence");
    let node_ref = canonical_hash(&node).expect("node ref");
    let run = test_run_value(&NixosVmTestRunInput {
        decision: "pass",
        topology_ref: &topology_ref,
        scenario: "phase1-node-service",
        fault_profile: "none",
        node_evidence_refs: &[node_ref],
        child_workflow_refs: &[],
        replay_status: "non-replayable-vm-observations",
        diagnostics: &[],
        log_refs: &[],
        caveats: &["does-not-grant-authority".to_string()],
    })
    .expect("test run");
    let text = to_text(&run).expect("render");
    assert!(text.contains("nixos-vm-test-run-v1"));
    assert!(text.contains("terminal-output-diagnostic-only"));
}

#[test]
fn vm_test_run_binds_framed_stream_and_network_diagnostic_child_refs() {
    let topology = topology_value(&NixosVmTopologyInput {
        nodes: &["node_a".to_string(), "node_b".to_string()],
        package_ref: "store:/nix/store/example-molten",
        package_path: "/nix/store/example-molten",
        network: "nixos-test-private",
        nix_inputs: &["self:locked".to_string()],
        caveats: &["vm evidence is platform evidence only".to_string()],
    })
    .expect("topology");
    let topology_ref = canonical_hash(&topology).expect("topology ref");
    let node_ref = local_ref("node-evidence");
    let framed_ref = local_ref("iroh-framed-envelope-receipt");
    let diagnostics_ref = local_ref("network-diagnostics-report");
    let metrics_ref = local_ref("metrics-snapshot");
    let run = test_run_value(&NixosVmTestRunInput {
        decision: "pass",
        topology_ref: &topology_ref,
        scenario: "phase1-framed-stream-diagnostics",
        fault_profile: "none",
        node_evidence_refs: &[node_ref],
        child_workflow_refs: &[framed_ref.clone(), diagnostics_ref.clone(), metrics_ref.clone()],
        replay_status: "non-replayable-vm-observations",
        diagnostics: &["live stream observations are diagnostic unless separately recorded".to_string()],
        log_refs: &[],
        caveats: &["does-not-grant-authority".to_string()],
    })
    .expect("test run");
    let text = to_text(&run).expect("render");
    assert!(text.contains(&framed_ref));
    assert!(text.contains(&diagnostics_ref));
    assert!(text.contains(&metrics_ref));
    assert!(text.contains("vm-evidence-does-not-grant-authority"));
}

#[test]
fn vm_network_control_probe_records_supported_and_unavailable_backends() {
    // r[verify molten.testing.nixos_vm_fault_injection.network_control_probe]
    let supported = network_control_probe_value(&NixosVmNetworkControlProbeInput {
        backend: "test-driver-network",
        target_link: "node_a->node_b",
        topology_ref: &local_ref("topology"),
        host_support: "supported",
        cleanup_strategy: "restore-link-state",
        diagnostics: &[],
        caveats: &["network fault support is host scoped".to_string()],
    })
    .expect("supported network probe");
    let unavailable = network_control_probe_value(&NixosVmNetworkControlProbeInput {
        backend: "none",
        target_link: "node_a->node_b",
        topology_ref: &local_ref("topology"),
        host_support: "unavailable",
        cleanup_strategy: "no-op",
        diagnostics: &["no-network-control-backend".to_string()],
        caveats: &["unavailable network support is not pass evidence".to_string()],
    })
    .expect("unavailable network probe");
    let supported_text = to_text(&supported).expect("render supported probe");
    let unavailable_text = to_text(&unavailable).expect("render unavailable probe");

    assert!(supported_text.contains("nixos-vm-network-control-probe-v1"));
    assert!(supported_text.contains("restore-link-state"));
    assert!(unavailable_text.contains("unavailable-is-not-pass-evidence"));
    assert!(unavailable_text.contains("no-network-control-backend"));
}

struct VmShardFixture {
    scenario_fixture_ref: String,
    topology_ref: String,
    package_ref: String,
    node_evidence_refs: Vec<String>,
    child_receipt_refs: Vec<String>,
    diagnostic_log_refs: Vec<String>,
    caveats: Vec<String>,
}

fn vm_shard_fixture() -> VmShardFixture {
    VmShardFixture {
        scenario_fixture_ref: local_ref("scenario-fixture"),
        topology_ref: local_ref("topology"),
        package_ref: local_ref("package"),
        node_evidence_refs: vec![local_ref("node-a"), local_ref("node-b")],
        child_receipt_refs: vec![local_ref("child-live-control")],
        diagnostic_log_refs: vec![local_ref("vm-log")],
        caveats: vec!["VM shard evidence is platform-scoped".to_string()],
    }
}

fn vm_shard_input<'a>(fixture: &'a VmShardFixture, unavailable: bool) -> NixosVmShardRunInput<'a> {
    NixosVmShardRunInput {
        shard_id: "vm-live-control",
        scenario_fixture_ref: &fixture.scenario_fixture_ref,
        topology_ref: &fixture.topology_ref,
        package_ref: &fixture.package_ref,
        node_evidence_refs: &fixture.node_evidence_refs,
        child_receipt_refs: &fixture.child_receipt_refs,
        diagnostic_log_refs: &fixture.diagnostic_log_refs,
        unavailable,
        claimed_decision: "pass",
        caveats: &fixture.caveats,
    }
}

#[test]
fn vm_shard_run_binds_scenario_child_receipts_and_logs() {
    // r[verify molten.testing.nixos_vm_multinode.sharded_checks]
    let fixture = vm_shard_fixture();
    let input = vm_shard_input(&fixture, false);
    let shard = evaluate_vm_shard_run(&input).expect("VM shard");
    let rendered = to_text(&shard.value).expect("render VM shard");

    assert_eq!(shard.decision, "pass");
    assert!(shard.diagnostics.is_empty());
    assert!(shard.shard_ref.starts_with("blake3:"));
    assert!(rendered.contains("nixos-vm-shard-run-v1"));
    assert!(rendered.contains("logs-diagnostic-only"));
}

#[test]
fn vm_shard_run_denies_unavailable_or_log_only_pass_claim() {
    // r[verify molten.testing.nixos_vm_multinode.sharded_checks]
    let mut fixture = vm_shard_fixture();
    fixture.child_receipt_refs = Vec::new();
    fixture.diagnostic_log_refs = Vec::new();
    let input = vm_shard_input(&fixture, true);
    let shard = evaluate_vm_shard_run(&input).expect("VM shard");

    assert_eq!(shard.decision, "deny");
    assert!(shard.diagnostics.iter().any(|item| item == "vm-shard-unavailable-as-pass:vm-live-control"));
    assert!(shard.diagnostics.iter().any(|item| item == "vm-shard-log-only-pass:vm-live-control"));
    assert!(shard.diagnostics.iter().any(|item| item == "vm-shard-missing-diagnostic-log:vm-live-control"));
}

struct VmAggregateFixture {
    topology_ref: String,
    package_ref: String,
    manifest_ref: String,
    required_shard_ids: Vec<String>,
    shard_refs: Vec<String>,
    denied_shard_ids: Vec<String>,
    unavailable_as_pass_shard_ids: Vec<String>,
    stale_child_refs: Vec<String>,
    log_only_child_refs: Vec<String>,
    caveats: Vec<String>,
}

fn vm_aggregate_fixture() -> VmAggregateFixture {
    VmAggregateFixture {
        topology_ref: local_ref("topology"),
        package_ref: local_ref("package"),
        manifest_ref: local_ref("manifest"),
        required_shard_ids: vec!["vm-smoke".to_string(), "vm-live-control".to_string()],
        shard_refs: vec![local_ref("vm-smoke-shard"), local_ref("vm-live-control-shard")],
        denied_shard_ids: Vec::new(),
        unavailable_as_pass_shard_ids: Vec::new(),
        stale_child_refs: Vec::new(),
        log_only_child_refs: Vec::new(),
        caveats: vec!["aggregate indexes child shard receipts without promoting diagnostics".to_string()],
    }
}

fn vm_aggregate_input<'a>(fixture: &'a VmAggregateFixture) -> NixosVmAggregateInput<'a> {
    NixosVmAggregateInput {
        topology_ref: &fixture.topology_ref,
        package_ref: &fixture.package_ref,
        manifest_ref: &fixture.manifest_ref,
        required_shard_ids: &fixture.required_shard_ids,
        shard_refs: &fixture.shard_refs,
        denied_shard_ids: &fixture.denied_shard_ids,
        unavailable_as_pass_shard_ids: &fixture.unavailable_as_pass_shard_ids,
        stale_child_refs: &fixture.stale_child_refs,
        log_only_child_refs: &fixture.log_only_child_refs,
        caveats: &fixture.caveats,
    }
}

#[test]
fn vm_aggregate_preserves_child_shard_evidence() {
    // r[verify molten.testing.nixos_vm_multinode.shard_aggregate]
    let fixture = vm_aggregate_fixture();
    let input = vm_aggregate_input(&fixture);
    let aggregate = evaluate_vm_aggregate(&input).expect("VM aggregate");
    let rendered = to_text(&aggregate.value).expect("render aggregate");

    assert_eq!(aggregate.decision, "pass");
    assert!(aggregate.diagnostics.is_empty());
    assert!(aggregate.aggregate_ref.starts_with("blake3:"));
    assert!(rendered.contains("nixos-vm-multinode-aggregate-v1"));
    assert!(rendered.contains("unavailable-not-promoted"));
}

#[test]
fn vm_aggregate_denies_missing_unavailable_stale_or_log_only_children() {
    // r[verify molten.testing.nixos_vm_multinode.shard_aggregate]
    let mut fixture = vm_aggregate_fixture();
    fixture.shard_refs = vec![local_ref("vm-smoke-shard")];
    fixture.denied_shard_ids = vec!["vm-live-control".to_string()];
    fixture.unavailable_as_pass_shard_ids = vec!["vm-fault".to_string()];
    fixture.stale_child_refs = vec![local_ref("stale-child")];
    fixture.log_only_child_refs = vec![local_ref("log-only-child")];
    let input = vm_aggregate_input(&fixture);
    let aggregate = evaluate_vm_aggregate(&input).expect("VM aggregate");

    assert_eq!(aggregate.decision, "deny");
    assert!(aggregate.diagnostics.iter().any(|item| item == "vm-aggregate-missing-shard-ref"));
    assert!(aggregate.diagnostics.iter().any(|item| item == "vm-aggregate-denied-shard:vm-live-control"));
    assert!(aggregate.diagnostics.iter().any(|item| item == "vm-aggregate-unavailable-as-pass:vm-fault"));
    assert!(aggregate.diagnostics.iter().any(|item| item.starts_with("vm-aggregate-stale-child:")));
    assert!(aggregate.diagnostics.iter().any(|item| item.starts_with("vm-aggregate-log-only-child:")));
}
