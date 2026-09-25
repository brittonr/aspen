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
