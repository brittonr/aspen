    fn executable_run_input() -> LocalMultiprocessExecutableRunInput {
        LocalMultiprocessExecutableRunInput {
            plan: local_plan_input(),
            startup_refs: vec![local_ref("startup-a"), local_ref("startup-b")],
            workflow_refs: vec![local_ref("workflow")],
            shutdown_refs: vec![local_ref("shutdown-a"), local_ref("shutdown-b")],
            cleanup_refs: vec![local_ref("cleanup")],
            ticket_status: TICKET_STATUS_CURRENT.to_string(),
            child_timed_out: false,
            orphaned_processes: Vec::new(),
            cleanup_succeeded: true,
            diagnostics: Vec::new(),
            caveats: vec!["local executable runner evidence is not VM evidence".to_string()],
        }
    }

    #[test]
    fn local_multiprocess_executable_runner_binds_shell_observations() {
        // r[verify molten.testing.multinode.local_multiprocess_executable_runner]
        // r[verify molten.testing.local_multiprocess_cluster_tier.middle_tier]
        let receipt = build_local_multiprocess_executable_run(&executable_run_input()).expect("executable local run");
        let rendered = crate::preserves_rail::to_text(&receipt.value).expect("render executable run");

        assert_eq!(receipt.decision, PASS_DECISION);
        assert!(receipt.diagnostics.is_empty());
        assert!(receipt.plan_ref.starts_with("blake3:"));
        assert!(receipt.run_ref.starts_with("blake3:"));
        assert!(rendered.contains("local-multiprocess-executable-run-v1"));
        assert!(rendered.contains("local-evidence-not-vm-evidence"));
    }

    #[test]
    fn local_multiprocess_executable_runner_denies_stale_timeout_orphan_and_missing_cleanup() {
        // r[verify molten.testing.multinode.local_multiprocess_runner_negatives]
        // r[verify molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
        let mut input = executable_run_input();
        input.ticket_status = "stale".to_string();
        input.child_timed_out = true;
        input.orphaned_processes = vec!["node-b".to_string()];
        input.cleanup_succeeded = false;
        input.workflow_refs = Vec::new();
        input.cleanup_refs = Vec::new();
        let receipt = build_local_multiprocess_executable_run(&input).expect("denied executable local run");

        assert_eq!(receipt.decision, DENY_DECISION);
        assert!(receipt.diagnostics.iter().any(|item| item == "local-executable-stale-ticket"));
        assert!(receipt.diagnostics.iter().any(|item| item == "local-executable-child-timeout"));
        assert!(receipt.diagnostics.iter().any(|item| item == "local-executable-orphaned-process:node-b"));
        assert!(receipt.diagnostics.iter().any(|item| item == "local-executable-cleanup-failed"));
        assert!(receipt.diagnostics.iter().any(|item| item == "local-run-missing-workflow-receipts"));
        assert!(receipt.diagnostics.iter().any(|item| item == "local-run-missing-cleanup-receipts"));
    }

    fn three_node_input() -> ThreeNodeQuorumEvidenceInput {
        ThreeNodeQuorumEvidenceInput {
            topology_ref: local_ref("three-node-topology"),
            scenario_fixture_ref: local_ref("fixture"),
            membership_gate_ref: local_ref("membership-gate"),
            reconciliation_gate_ref: local_ref("reconciliation-gate"),
            node_summary_refs: vec![local_ref("node-a"), local_ref("node-b"), local_ref("node-c")],
            quorum_refs: vec![local_ref("quorum-majority"), local_ref("duplicate-suppression")],
            restarting_member: "node-b".to_string(),
            duplicate_semantic_commit: false,
            log_only_quorum: false,
            caveats: vec!["three-node VM evidence is topology-scoped".to_string()],
        }
    }

    #[test]
    fn three_node_quorum_gate_binds_membership_restart_and_reconciliation_refs() {
        // r[verify molten.testing.multinode.three_node_quorum_topology]
        // r[verify molten.testing.three_node_quorum_vm.executable_shard]
        let profile = default_topology_profiles()
            .into_iter()
            .find(|profile| profile.id == PROFILE_THREE_NODE_QUORUM)
            .expect("three-node profile");
        let gate = evaluate_three_node_quorum_evidence(&three_node_input()).expect("three-node gate");
        let rendered = crate::preserves_rail::to_text(&gate.value).expect("render three-node gate");

        assert_eq!(profile.roles.len(), THREE_NODE_PROFILE_ROLE_COUNT);
        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
        assert!(rendered.contains("three-node-quorum-gate-v1"));
        assert!(rendered.contains("topology-scoped"));
    }

    #[test]
    fn three_node_quorum_gate_denies_missing_quorum_duplicate_and_log_only_claims() {
        // r[verify molten.testing.multinode.three_node_vm_membership_negatives]
        // r[verify molten.testing.three_node_quorum_vm.negatives]
        let mut input = three_node_input();
        input.quorum_refs = Vec::new();
        input.duplicate_semantic_commit = true;
        input.log_only_quorum = true;
        let gate = evaluate_three_node_quorum_evidence(&input).expect("three-node gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "three-node-missing-quorum-refs"));
        assert!(gate.diagnostics.iter().any(|item| item == "three-node-duplicate-semantic-commit"));
        assert!(gate.diagnostics.iter().any(|item| item == "three-node-log-only-quorum"));
    }

    fn vm_scenario_gate_input() -> VmScenarioGateInput {
        VmScenarioGateInput {
            scenario_metadata_ref: local_ref("scenario-metadata"),
            topology_membership_gate_ref: local_ref("membership-gate"),
            reconciliation_gate_ref: local_ref("reconciliation-gate"),
            live_transport_gate_ref: Some(local_ref("live-transport-gate")),
            expected_artifact_kinds: vec![
                "nixos-vm-test-run-v1".to_string(),
                "multinode-reconciliation-gate-v1".to_string(),
            ],
            observed_artifact_kinds: vec![
                "nixos-vm-test-run-v1".to_string(),
                "multinode-reconciliation-gate-v1".to_string(),
            ],
            unsupported_pass_claim: false,
            log_only_reconciliation: false,
            caveats: vec!["VM scenario gate evidence does not grant authority".to_string()],
        }
    }

    #[test]
    fn vm_scenario_gate_binds_metadata_membership_reconciliation_and_live_transport() {
        // r[verify molten.testing.multinode.vm_scenario_metadata_gate]
        // r[verify molten.testing.multinode.vm_reconciliation_gate]
        // r[verify molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
        let gate = evaluate_vm_scenario_gate(&vm_scenario_gate_input()).expect("VM scenario gate");
        let rendered = crate::preserves_rail::to_text(&gate.value).expect("render VM scenario gate");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
        assert!(rendered.contains("vm-scenario-gate-v1"));
        assert!(rendered.contains("reconciliation-gate-required"));
    }

    #[test]
    fn vm_scenario_gate_denies_wrong_fixture_shape_and_log_only_reconciliation() {
        // r[verify molten.testing.multinode.vm_scenario_metadata_gate]
        // r[verify molten.testing.multinode.vm_reconciliation_gate]
        // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
        let mut input = vm_scenario_gate_input();
        input.observed_artifact_kinds = vec!["log".to_string()];
        input.unsupported_pass_claim = true;
        input.log_only_reconciliation = true;
        input.live_transport_gate_ref = Some("not-a-ref".to_string());
        let gate = evaluate_vm_scenario_gate(&input).expect("VM scenario gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "vm-scenario-artifact-kind-mismatch"));
        assert!(gate.diagnostics.iter().any(|item| item == "vm-scenario-unsupported-pass-claim"));
        assert!(gate.diagnostics.iter().any(|item| item == "vm-scenario-log-only-reconciliation"));
        assert!(gate.diagnostics.iter().any(|item| item == "invalid-VM scenario live transport gate-ref"));
    }

    fn vm_failure_export_input() -> VmFailureReproExportInput {
        VmFailureReproExportInput {
            scenario_fixture_ref: local_ref("fixture"),
            topology_ref: local_ref("topology"),
            scheduler_ref: local_ref("scheduler"),
            seed_ref: local_ref("seed"),
            fault_plan_ref: local_ref("fault-plan"),
            command_refs: vec![local_ref("command")],
            node_summary_refs: vec![local_ref("node-summary")],
            child_receipt_refs: vec![local_ref("child")],
            validation_refs: vec![local_ref("validation")],
            diagnostic_log_refs: vec![local_ref("vm-log")],
            redaction_policy_ref: local_ref("redaction-policy"),
            private_attachment_refs: Vec::new(),
            reveal_receipt_refs: Vec::new(),
            unavailable_host_support: true,
            denied_or_failed_validation: false,
            caveats: vec!["VM failure bundles are diagnostic-only".to_string()],
        }
    }

    #[test]
    fn vm_failure_repro_export_seals_non_replayable_diagnostic_bundle() {
        // r[verify molten.testing.multinode.vm_failure_repro_export]
        // r[verify molten.testing.cluster_failure_repro_bundles.bundle_schema]
        let export = export_vm_failure_repro(&vm_failure_export_input()).expect("VM failure repro export");
        let rendered = crate::preserves_rail::to_text(&export.value).expect("render VM failure export");

        assert_eq!(export.decision, PASS_DECISION);
        assert!(export.diagnostics.is_empty());
        assert!(export.bundle_ref.starts_with("blake3:"));
        assert!(export.verification_ref.starts_with("blake3:"));
        assert!(rendered.contains("vm-failure-repro-export-v1"));
        assert!(rendered.contains("non-replayable-vm-observation"));
    }

    #[test]
    fn vm_failure_repro_export_denies_private_without_reveal_and_pass_condition_absence() {
        // r[verify molten.testing.multinode.vm_failure_repro_privacy_gate]
        // r[verify molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass]
        let mut input = vm_failure_export_input();
        input.private_attachment_refs = vec![local_ref("private-log")];
        input.unavailable_host_support = false;
        input.denied_or_failed_validation = false;
        let export = export_vm_failure_repro(&input).expect("VM failure repro export");

        assert_eq!(export.decision, DENY_DECISION);
        assert!(export.diagnostics.iter().any(|item| item == "failure-repro-private-without-reveal"));
        assert!(export.diagnostics.iter().any(|item| item == "vm-failure-repro-missing-failure-condition"));
    }

    fn distributed_topology() -> crate::distributed_core::Topology {
        crate::distributed_core::Topology {
            peers: vec![
                crate::distributed_core::Peer {
                    id: "peer-a".to_string(),
                    roles: vec!["sender".to_string()],
                },
                crate::distributed_core::Peer {
                    id: "peer-b".to_string(),
                    roles: vec!["receiver".to_string()],
                },
            ],
            channels: vec![crate::distributed_core::Channel {
                id: "a-to-b".to_string(),
                from_peer: "peer-a".to_string(),
                to_peer: "peer-b".to_string(),
                topic: "node-control".to_string(),
            }],
            caveats: vec!["generated simulation evidence only".to_string()],
        }
    }

    fn generated_command(operation_id: &str) -> crate::distributed_core::SimulationCommand {
        crate::distributed_core::SimulationCommand {
            operation_id: operation_id.to_string(),
            from_peer: "peer-a".to_string(),
            to_peer: "peer-b".to_string(),
            payload_ref: local_ref(&format!("payload:{operation_id}")),
            commit_ref: local_ref(&format!("commit:{operation_id}")),
            authority_ref: Some(local_ref("authority")),
            policy_ref: Some(local_ref("policy")),
            resource_ref: Some(local_ref("resource")),
            transport_ref: Some(local_ref("transport")),
            requires_authority: true,
            requires_quorum: false,
        }
    }

    fn generated_case_with_fault(
        case_id: &str,
        invariant: &str,
        fault_kind: &str,
        operation_id: &str,
    ) -> GeneratedDistributedCase {
        GeneratedDistributedCase {
            case_id: case_id.to_string(),
            invariant_name: invariant.to_string(),
            simulation: crate::distributed_core::SimulationInput {
                topology: distributed_topology(),
                scheduler: crate::distributed_core::SchedulerProfile {
                    id: "generated-round-robin".to_string(),
                    policy: "deterministic-virtual-clock".to_string(),
                    max_ticks: SIMULATION_MAX_TICKS,
                },
                seed: crate::distributed_core::SimulationSeed {
                    id: format!("seed:{case_id}"),
                    entropy_ref: local_ref(&format!("seed:{case_id}")),
                },
                fault_plan: crate::distributed_core::FaultPlan {
                    events: vec![crate::distributed_core::FaultEvent {
                        kind: fault_kind.to_string(),
                        target_kind: "operation".to_string(),
                        target: operation_id.to_string(),
                        operation_id: Some(operation_id.to_string()),
                        start_tick: FAULT_START_TICK,
                        duration_ticks: FAULT_DURATION_TICKS,
                        diagnostic: format!("generated:{fault_kind}"),
                    }],
                    caveats: vec!["generated bounded fault plan".to_string()],
                },
                source_ref: local_ref("source"),
                test_binary_ref: local_ref("test-binary"),
                commands: vec![generated_command(operation_id)],
                child_workflow_refs: vec![local_ref("child")],
                allowed_variance_refs: vec![local_ref("variance:none")],
            },
        }
    }
