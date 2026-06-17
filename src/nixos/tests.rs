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
