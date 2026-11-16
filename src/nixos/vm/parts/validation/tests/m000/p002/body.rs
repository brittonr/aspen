
    fn linked_storage_fixture(uid: u32, post_tree_ref: &str) -> (IoValue, IoValue, IoValue, IoValue, IoValue, IoValue) {
        // Structural validator fixture only. It is not evidence that a VM ran.
        let package = "/nix/store/example-molten";
        let topology = crate::nixos_vm::topology_value(&crate::nixos_vm::NixosVmTopologyInput {
            nodes: &[NODE_A.to_string(), NODE_B.to_string()],
            package_ref: &format!("store:{package}"),
            package_path: package,
            network: NETWORK,
            nix_inputs: &["source:locked".to_string()],
            caveats: &["structural fixture, not execution evidence".to_string()],
        }).expect("topology");
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let startup_ref = local_ref("storage-startup");
        let config_ref = local_ref("storage-config");
        let binding_ref = local_ref("storage-binding");
        let executable_ref = local_ref("storage-executable");
        let tree_ref = local_ref("storage-tree");
        let descriptor = crate::nixos_vm::vm_fault_descriptor_value(&crate::nixos_vm::NixosVmFaultDescriptorInput {
            fault_id: "storage-node-b",
            topology_ref: &topology_ref,
            target_node: NODE_B,
            target_link: None,
            fault_kind: "permission-denied-state-root",
            command_profile: "node-status-health-write",
            expected_outcome: "deny-before-side-effects",
            duration_millis: FAULT_DURATION_MILLIS,
            trigger: "protected-status-health-open",
            preflight_refs: &[local_ref("storage-preflight")],
            caveats: &["structural fixture only".to_string()],
        }).expect("descriptor");
        let attempt = node_state_permission_attempt_value(&NodeStateAttemptInput {
            state_root: STATE_ROOT,
            target: "health-receipt.preserves",
            startup_ref: &startup_ref,
            config_ref: &config_ref,
            binding_ref: &binding_ref,
            executable_path: "/nix/store/example-molten/bin/molten",
            executable_ref: &executable_ref,
            effective_uid: uid,
            effective_gid: 987,
            groups: "987",
            cap_eff: "0000000000000000",
            errno: 13,
        }).expect("attempt");
        let observation = |phase: &str, observed_tree: &str| {
            vm_state_observation_value(&VmStateObservationInput {
                phase,
                target_node: NODE_B,
                state_root: STATE_ROOT,
                target: "health-receipt.preserves",
                tree_ref: observed_tree,
                root_uid: 987,
                root_gid: 987,
                root_mode: 0o500,
                target_absent: true,
                package_path: package,
                executable_ref: &executable_ref,
                startup_ref: &startup_ref,
                config_ref: &config_ref,
                binding_ref: &binding_ref,
            }).expect("observation")
        };
        let pre = observation("pre", &tree_ref);
        let post = observation("post", post_tree_ref);
        let receipt = linked_denial_receipt(&descriptor, &attempt, &pre, &post);
        (topology, descriptor, receipt, attempt, pre, post)
    }

    fn linked_denial_receipt(descriptor: &IoValue, attempt: &IoValue, pre: &IoValue, post: &IoValue) -> IoValue {
        let descriptor_ref = crate::preserves_rail::canonical_hash(descriptor).expect("descriptor ref");
        let attempt_ref = crate::preserves_rail::canonical_hash(attempt).expect("attempt ref");
        let pre_ref = crate::preserves_rail::canonical_hash(pre).expect("pre ref");
        let post_ref = crate::preserves_rail::canonical_hash(post).expect("post ref");
        crate::nixos_vm::vm_fault_receipt_value(&crate::nixos_vm::NixosVmFaultReceiptInput {
            decision: "deny",
            descriptor_ref: &descriptor_ref,
            host_support: "supported",
            pre_fault_refs: std::slice::from_ref(&pre_ref),
            injection_refs: std::slice::from_ref(&pre_ref),
            child_refs: &[attempt_ref],
            post_fault_refs: &[post_ref],
            replay_status: "denial-observed-before-mutation",
            diagnostics: &["typed denied open".to_string()],
            log_refs: &[local_ref("diagnostic-log")],
            caveats: &["structural fixture, no VM execution".to_string()],
        }).expect("receipt")
    }

    #[test]
    fn vm_storage_observed_denial_requires_exact_linked_operation_values() {
        // r[verify molten.testing.nixos_vm_fault_injection.storage_state_faults]
        let tree_ref = local_ref("storage-tree");
        let (topology, descriptor, receipt, attempt, pre, post) = linked_storage_fixture(987, &tree_ref);
        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceInput {
            topology_value: &topology,
            descriptor_values: &[descriptor],
            receipt_values: &[receipt],
            attempt_value: Some(&attempt),
            pre_observation_value: Some(&pre),
            post_observation_value: Some(&post),
        }).expect("structural validation");
        assert_eq!(validation.decision, "pass");
    }

    #[test]
    fn vm_storage_wrong_identity_or_changed_state_denies_observed_claim() {
        let tree_ref = local_ref("storage-tree");
        let changed_ref = local_ref("changed-state");
        for (uid, post_tree, expected) in [
            (0, tree_ref.as_str(), "vm-fault-observed-denial-permissions-not-denied"),
            (987, changed_ref.as_str(), "vm-fault-observed-denial-protected-state-changed"),
        ] {
            let (topology, descriptor, receipt, attempt, pre, post) = linked_storage_fixture(uid, post_tree);
            let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceInput {
                topology_value: &topology,
                descriptor_values: &[descriptor],
                receipt_values: &[receipt],
                attempt_value: Some(&attempt),
                pre_observation_value: Some(&pre),
                post_observation_value: Some(&post),
            }).expect("structural validation");
            assert_eq!(validation.decision, "deny");
            assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == expected));
        }
    }

    #[test]
    fn vm_storage_arbitrary_child_ref_cannot_replace_actual_attempt() {
        let tree_ref = local_ref("storage-tree");
        let (topology, descriptor, _, attempt, pre, post) = linked_storage_fixture(987, &tree_ref);
        let descriptor_ref = crate::preserves_rail::canonical_hash(&descriptor).expect("descriptor ref");
        let pre_ref = crate::preserves_rail::canonical_hash(&pre).expect("pre ref");
        let post_ref = crate::preserves_rail::canonical_hash(&post).expect("post ref");
        let receipt = crate::nixos_vm::vm_fault_receipt_value(&crate::nixos_vm::NixosVmFaultReceiptInput {
            decision: "deny",
            descriptor_ref: &descriptor_ref,
            host_support: "supported",
            pre_fault_refs: std::slice::from_ref(&pre_ref),
            injection_refs: std::slice::from_ref(&pre_ref),
            child_refs: &[local_ref("handwritten-child")],
            post_fault_refs: &[post_ref],
            replay_status: "denial-observed-before-mutation",
            diagnostics: &["handwritten denial".to_string()],
            log_refs: &[local_ref("log")],
            caveats: &["structural fixture".to_string()],
        }).expect("receipt");
        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceInput {
            topology_value: &topology,
            descriptor_values: &[descriptor],
            receipt_values: &[receipt],
            attempt_value: Some(&attempt),
            pre_observation_value: Some(&pre),
            post_observation_value: Some(&post),
        }).expect("structural validation");
        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics.iter().any(|diagnostic| diagnostic == "vm-fault-observed-denial-artifact-ref-mismatch"));
    }
    #[test]
    fn vm_fault_arbitrary_child_ref_cannot_qualify_observed_storage_denial() {
        // r[verify molten.testing.nixos_vm_fault_injection.storage_state_faults]
        let topology = topology();
        let topology_ref = crate::preserves_rail::canonical_hash(&topology).expect("topology ref");
        let descriptor =
            fault_descriptor(&topology_ref, "permission-denied-state-root", "deny-before-side-effects", NODE_B);
        let descriptor_ref = crate::preserves_rail::canonical_hash(&descriptor).expect("descriptor ref");
        let receipt = crate::nixos_vm::vm_fault_receipt_value(&crate::nixos_vm::NixosVmFaultReceiptInput {
            decision: "deny",
            descriptor_ref: &descriptor_ref,
            host_support: "supported",
            pre_fault_refs: &[local_ref("pre-state")],
            injection_refs: &[local_ref("handwritten-injection")],
            child_refs: &[local_ref("unrelated-receipt")],
            post_fault_refs: &[local_ref("post-state")],
            replay_status: "denial-observed-before-mutation",
            diagnostics: &["permission denied before mutation".to_string()],
            log_refs: &[local_ref("fault-log")],
            caveats: &["fault receipts do not grant authority".to_string()],
        })
        .expect("fault receipt");

        let validation = validate_nixos_vm_fault_evidence(&NixosVmFaultEvidenceInput {
            topology_value: &topology,
            descriptor_values: &[descriptor],
            receipt_values: &[receipt],
            attempt_value: None,
            pre_observation_value: None,
            post_observation_value: None,
        })
        .expect("fault validation");

        assert_eq!(validation.decision, "deny");
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vm-fault-observed-denial-missing-operation-observations")
        );
    }
