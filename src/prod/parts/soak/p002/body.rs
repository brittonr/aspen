
#[cfg(test)]
mod tests {
    use super::*;

    fn content_ref_from_bytes(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn to_text(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn canonical_hash(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(value)
    }

    fn local_ref(name: &str) -> String {
        content_ref_from_bytes(name.as_bytes())
    }

    #[test]
    fn evidence_export_binds_node_and_artifacts() {
        let artifact_ref = local_ref("artifact");
        let value = evidence_export_value(&EvidenceExportInput {
            node: "node-a",
            node_evidence_ref: &local_ref("node-evidence"),
            artifact_refs: std::slice::from_ref(&artifact_ref),
            log_refs: &[local_ref("log")],
        })
        .expect("evidence export");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-evidence-export-v1"));
        assert!(text.contains(&artifact_ref));
    }

    #[test]
    fn run_receipt_binds_phase_one_child_categories() {
        let node_evidence = vec![local_ref("node-a"), local_ref("node-b")];
        let peer_ticket = vec![local_ref("ticket")];
        let node_control = vec![local_ref("protocol-gate")];
        let remote_service = vec![local_ref("remote-deliver")];
        let job = vec![local_ref("job-worker")];
        let coordination = vec![local_ref("coordination")];
        let evidence_export = vec![local_ref("export-a"), local_ref("export-b")];
        let value = run_value(&RunInput {
            decision: "pass",
            scenario: "phase1-soak",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &node_evidence,
            peer_ticket_refs: &peer_ticket,
            control_refs: &node_control,
            remote_service_refs: &remote_service,
            job_refs: &job,
            coordination_refs: &coordination,
            evidence_export_refs: &evidence_export,
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &[],
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[local_ref("log")],
            caveats: &["soak evidence is pilot-scoped".to_string()],
        })
        .expect("soak run");
        let reference = canonical_hash(&value).expect("ref");
        let text = to_text(&value).expect("text");
        assert!(reference.starts_with("blake3:"));
        assert!(text.contains("prod-soak-run-v1"));
        assert!(text.contains("phase1-soak"));
        assert!(text.contains(&remote_service[0]));
    }

    #[test]
    fn run_receipt_binds_network_diagnostics_and_metrics_refs() {
        let node_evidence = vec![local_ref("node-a"), local_ref("node-b")];
        let peer_ticket = vec![local_ref("ticket")];
        let node_control = vec![local_ref("framed-stream")];
        let remote_service = vec![local_ref("remote-service")];
        let job = vec![local_ref("job-worker")];
        let coordination = vec![local_ref("coordination")];
        let evidence_export = vec![local_ref("export-a")];
        let network_diagnostic = local_ref("network-diagnostics");
        let metrics_snapshot = local_ref("metrics-snapshot");
        let resource_refs = vec![network_diagnostic.clone(), metrics_snapshot.clone()];
        let value = run_value(&RunInput {
            decision: "pass",
            scenario: "phase1-network-diagnostics",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &node_evidence,
            peer_ticket_refs: &peer_ticket,
            control_refs: &node_control,
            remote_service_refs: &remote_service,
            job_refs: &job,
            coordination_refs: &coordination,
            evidence_export_refs: &evidence_export,
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &resource_refs,
            replay_status: "non-replayable-live-observations",
            diagnostics: &["network diagnostics are observability evidence only".to_string()],
            log_refs: &[local_ref("log")],
            caveats: &["network diagnostics do not grant side-effect authority".to_string()],
        })
        .expect("soak run");
        let text = to_text(&value).expect("text");
        assert!(text.contains(&network_diagnostic));
        assert!(text.contains(&metrics_snapshot));
        assert!(text.contains("soak-evidence-does-not-grant-authority"));
    }

    #[test]
    fn passing_run_requires_all_phase_one_categories() {
        let error = run_value(&RunInput {
            decision: "pass",
            scenario: "missing-remote",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &[local_ref("node")],
            peer_ticket_refs: &[local_ref("ticket")],
            control_refs: &[local_ref("control")],
            remote_service_refs: &[],
            job_refs: &[local_ref("job")],
            coordination_refs: &[local_ref("coordination")],
            evidence_export_refs: &[local_ref("export")],
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &[],
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[],
            caveats: &["diagnostic only".to_string()],
        })
        .expect_err("missing remote should deny pass");
        assert!(error.to_string().contains("remote service"));
    }

    #[test]
    fn durability_receipt_requires_restart_and_state_refs() {
        let value = durability_value(&DurabilityInput {
            decision: "pass",
            scenario: "restart-durability",
            queued_control_refs: &[local_ref("restart-queue")],
            recovery_refs: &[local_ref("control-loop")],
            ledger_refs: &[local_ref("ledger-readback")],
            chunk_refs: &[local_ref("chunk-put")],
            retention_refs: &[local_ref("retention-pin")],
            diagnostics: &[],
            caveats: &["durability evidence is pilot scoped".to_string()],
        })
        .expect("durability");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-durability-v1"));
        assert!(text.contains("restart-durability"));
    }

    #[test]
    fn resource_envelope_receipt_binds_bounds_and_denials() {
        let value = resource_envelope_value(&ResourceEnvelopeInput {
            decision: "pass",
            scenario: "pilot-resource-envelope",
            queue_depth: 1,
            max_queue_depth: 8,
            receipt_bytes: 4096,
            max_receipt_bytes: 1_000_000,
            store_bytes: 65_536,
            max_store_bytes: 10_000_000,
            delivery_latency_ms: 50,
            max_delivery_latency_ms: 5_000,
            recovery_time_ms: 100,
            max_recovery_time_ms: 10_000,
            pressure_refs: &[local_ref("pressure")],
            denial_refs: &[local_ref("denial")],
            diagnostics: &[],
            caveats: &["resource envelope evidence is pilot scoped".to_string()],
        })
        .expect("resource envelope");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-resource-envelope-v1"));
        assert!(text.contains("queue-depth-bound"));
    }

    #[test]
    fn fault_case_binds_denial_for_stale_ticket() {
        let denial = vec![local_ref("stale-ticket-denial")];
        let evidence = vec![local_ref("ticket")];
        let value = fault_case_value(&FaultCaseInput {
            decision: "pass",
            scenario: "network-faults",
            fault_kind: "stale-ticket",
            injection: "simulated-live-gate",
            expected_outcome: "deny-before-side-effects",
            evidence_refs: &evidence,
            denial_refs: &denial,
            replay_status: "simulated-fault",
            diagnostics: &["stale ticket denied before control side effects".to_string()],
            caveats: &["simulated fault evidence is diagnostic".to_string()],
        })
        .expect("fault case");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-fault-case-v1"));
        assert!(text.contains(&denial[0]));
    }

    #[test]
    fn fault_matrix_requires_all_network_faults_for_pass() {
        let fault_cases = vec![local_ref("case")];
        let incomplete = vec!["delay".to_string()];
        let error = fault_matrix_value(&FaultMatrixInput {
            decision: "pass",
            scenario: "network-faults",
            fault_case_refs: &fault_cases,
            fault_kinds: &incomplete,
            diagnostics: &[],
            caveats: &["simulated faults are diagnostic".to_string()],
        })
        .expect_err("missing faults deny pass");
        assert!(error.to_string().contains("drop"));

        let complete = REQUIRED_NETWORK_FAULTS.iter().map(|kind| (*kind).to_string()).collect::<Vec<_>>();
        let value = fault_matrix_value(&FaultMatrixInput {
            decision: "pass",
            scenario: "network-faults",
            fault_case_refs: &fault_cases,
            fault_kinds: &complete,
            diagnostics: &[],
            caveats: &["simulated faults are diagnostic".to_string()],
        })
        .expect("complete matrix");
        assert!(to_text(&value).expect("text").contains("prod-soak-fault-matrix-v1"));
    }
}
