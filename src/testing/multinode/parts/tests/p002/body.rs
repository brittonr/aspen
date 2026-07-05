    #[test]
    fn generated_distributed_cases_replay_benign_interleavings_stably() {
        // r[verify molten.testing.distributed_simulation.generated_fault_interleavings]
        let cases = [
            generated_case_with_fault("delay", "deterministic replay", "delay", "op-delay"),
            generated_case_with_fault("restart", "restart stability", "restart", "op-restart"),
            generated_case_with_fault("duplicate", "idempotent duplicate", "duplicate", "op-duplicate"),
        ];

        for case in cases {
            let repro = run_generated_distributed_case(&case).expect("generated repro");
            assert_eq!(repro.decision, PASS_DECISION);
            assert_eq!(repro.run_ref, repro.replay_run_ref);
            assert!(repro.repro_ref.starts_with("blake3:"));
        }
    }

    #[test]
    fn generated_distributed_cases_preserve_deny_repro_seed() {
        // r[verify molten.testing.distributed_simulation.generated_fault_interleavings]
        // r[verify molten.testing.distributed_simulation.generated_repro_seed]
        let mut case =
            generated_case_with_fault("missing-authority", "missing authority denies", "stale-evidence", "op-deny");
        case.simulation.commands[0].authority_ref = None;
        let repro = run_generated_distributed_case(&case).expect("generated deny repro");
        let rendered = crate::preserves_rail::to_text(&repro.value).expect("render repro");

        assert_eq!(repro.decision, PASS_DECISION);
        assert_eq!(repro.run_ref, repro.replay_run_ref);
        assert!(rendered.contains("generated-distributed-repro-v1"));
        assert!(rendered.contains(DIAGNOSTIC_ONLY));
    }

    fn repro_input() -> FailureReproBundleInput {
        FailureReproBundleInput {
            scenario_fixture_ref: local_ref("fixture"),
            topology_ref: local_ref("topology"),
            scheduler_ref: local_ref("scheduler"),
            seed_ref: local_ref("seed"),
            fault_plan_ref: local_ref("fault-plan"),
            command_refs: vec![local_ref("command")],
            node_summary_refs: vec![local_ref("node-summary")],
            receipt_refs: vec![local_ref("receipt")],
            diagnostic_refs: vec![local_ref("diagnostic")],
            log_refs: vec![local_ref("redacted-log")],
            redaction_policy_ref: local_ref("redaction-policy"),
            replay_status: REPLAYABLE_SIMULATION.to_string(),
            diagnostic_only: true,
            sealed: true,
            private_attachment_refs: Vec::new(),
            reveal_receipt_refs: Vec::new(),
            claimed_payload_ref: None,
            caveats: vec!["failure repro is not pass evidence".to_string()],
        }
    }

    #[test]
    fn failure_repro_bundle_verifies_sealed_simulation_payload() {
        // r[verify molten.testing.multinode.failure_repro_bundle]
        let input = repro_input();
        let bundle = build_failure_repro_bundle(&input).expect("bundle");
        let verification = verify_failure_repro_bundle(&input).expect("verification");

        assert_eq!(verification.decision, PASS_DECISION);
        assert_eq!(verification.payload_ref, bundle.payload_ref);
        assert!(verification.diagnostics.is_empty());
    }

    #[test]
    fn failure_repro_bundle_privacy_and_pass_gate_fail_closed() {
        // r[verify molten.testing.multinode.failure_repro_privacy_and_replay]
        let mut input = repro_input();
        let payload_ref = canonical_hash(&failure_repro_payload_value(&input).expect("payload")).expect("payload ref");
        input.claimed_payload_ref = Some(local_ref("tampered-payload"));
        input.private_attachment_refs = vec![local_ref("private-log")];
        input.replay_status = NON_REPLAYABLE_VM.to_string();
        let verification = verify_failure_repro_bundle(&input).expect("verification");
        let valid_verification = verify_failure_repro_bundle(&repro_input()).expect("valid verification");
        let pass_gate = gate_failure_repro_as_pass(&valid_verification, true).expect("pass gate");

        assert_ne!(verification.payload_ref, payload_ref);
        assert_eq!(verification.decision, DENY_DECISION);
        assert!(verification.diagnostics.iter().any(|item| item == "failure-repro-seal-mismatch"));
        assert!(verification.diagnostics.iter().any(|item| item == "failure-repro-private-without-reveal"));
        assert_eq!(pass_gate.decision, DENY_DECISION);
        assert!(pass_gate.diagnostics.iter().any(|item| item == "diagnostic-bundle-cannot-satisfy-pass"));
    }

    fn live_transport_input() -> LiveTransportVmEvidenceInput {
        LiveTransportVmEvidenceInput {
            expected_sender_node: "sender".to_string(),
            actual_sender_node: "sender".to_string(),
            expected_receiver_node: "receiver".to_string(),
            actual_receiver_node: "receiver".to_string(),
            expected_peer: "peer:operator".to_string(),
            actual_peer: "peer:operator".to_string(),
            topic: "node-control".to_string(),
            operation_id: "blake3:operation".to_string(),
            ticket_ref: local_ref("ticket"),
            peer_admission_ref: local_ref("peer-admission"),
            authority_ref: local_ref("authority"),
            send_ref: local_ref("send"),
            receive_ref: local_ref("receive"),
            ingress_ref: local_ref("ingress"),
            queue_ref: local_ref("queue"),
            dispatch_ref: local_ref("dispatch"),
            reconcile_ref: local_ref("reconcile"),
            ack_ref: local_ref("ack"),
            protocol_gate_ref: local_ref("protocol-gate"),
            log_refs: vec![local_ref("vm-log")],
            caveats: vec!["live VM transport evidence is topology-scoped".to_string()],
        }
    }

    #[test]
    fn live_transport_vm_gate_accepts_complete_receipt_chain() {
        // r[verify molten.testing.nixos_vm.cross_node_live_transport]
        let gate = evaluate_live_transport_vm_gate(&live_transport_input()).expect("live transport gate");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
    }

    #[test]
    fn live_transport_vm_gate_denies_wrong_peer_and_log_only_receive() {
        // r[verify molten.testing.nixos_vm.live_transport_negative_gate]
        let mut input = live_transport_input();
        input.actual_peer = "peer:wrong".to_string();
        input.receive_ref = String::new();
        input.protocol_gate_ref = String::new();
        let gate = evaluate_live_transport_vm_gate(&input).expect("live transport gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-peer-mismatch"));
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-missing-receive"));
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-missing-protocol-gate"));
    }

    fn supported_fault_case(kind: &str) -> VmFaultSupportCase {
        VmFaultSupportCase {
            fault_kind: kind.to_string(),
            required_capability: "test-driver-control".to_string(),
            target: "node-a".to_string(),
            command_profile: "nixos-vm-multinode".to_string(),
            expected_outcome: PASS_DECISION.to_string(),
            host_support: SUPPORTED.to_string(),
            preflight_refs: vec![local_ref("preflight")],
            injection_refs: vec![local_ref("injection")],
            child_refs: vec![local_ref("child")],
            post_fault_refs: vec![local_ref("post")],
            diagnostic_refs: vec![local_ref("diagnostic")],
            caveats: vec!["VM fault evidence is platform-scoped".to_string()],
        }
    }

    #[test]
    fn executable_vm_fault_support_matrix_records_supported_and_unavailable_cases() {
        // r[verify molten.testing.nixos_vm.executable_fault_support_matrix]
        let mut unavailable = supported_fault_case("bounded-disk-pressure");
        unavailable.expected_outcome = UNAVAILABLE.to_string();
        unavailable.host_support = UNAVAILABLE.to_string();
        unavailable.injection_refs = Vec::new();
        unavailable.child_refs = Vec::new();
        let matrix = build_vm_fault_support_matrix(&[
            supported_fault_case("network-partition"),
            supported_fault_case("crash-restart"),
            unavailable,
        ])
        .expect("fault matrix");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert!(matrix.diagnostics.is_empty());
    }

    #[test]
    fn executable_vm_fault_support_matrix_denies_invalid_claims() {
        // r[verify molten.testing.nixos_vm.executable_fault_validation_negatives]
        let mut unsupported_pass = supported_fault_case("unsupported-host-feature");
        unsupported_pass.host_support = UNAVAILABLE.to_string();
        unsupported_pass.injection_refs = Vec::new();
        unsupported_pass.child_refs = Vec::new();
        unsupported_pass.diagnostic_refs = Vec::new();
        let matrix = build_vm_fault_support_matrix(&[unsupported_pass]).expect("fault matrix");

        assert_eq!(matrix.decision, DENY_DECISION);
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-unsupported-pass:unsupported-host-feature"));
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-missing-injection:unsupported-host-feature"));
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-missing-child:unsupported-host-feature"));
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|item| item == "vm-fault-unavailable-missing-diagnostic:unsupported-host-feature")
        );
    }
