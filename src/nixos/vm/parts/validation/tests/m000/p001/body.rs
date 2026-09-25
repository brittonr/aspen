
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
