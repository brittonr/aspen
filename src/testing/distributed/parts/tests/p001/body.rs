    #[test]
    fn direct_fault_fixture_mutations_change_evidence_or_fail_closed() {
        // r[verify molten.testing.distributed_simulation.direct_fault_fixtures]
        let base_input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_DELAY, "op-mutation", "bounded-delay")],
                caveats: Vec::new(),
            },
            vec![command("op-mutation")],
        );
        let base_run = run_simulation(&base_input).expect("base fixture run");

        let mut peer_mutation = base_input.clone();
        peer_mutation.topology.peers[0].id = "peer-mutated".to_string();
        peer_mutation.topology.channels[0].from_peer = "peer-mutated".to_string();
        peer_mutation.commands[0].from_peer = "peer-mutated".to_string();
        let peer_run = run_simulation(&peer_mutation).expect("peer mutation run");

        let mut operation_mutation = base_input.clone();
        operation_mutation.fault_plan.events[0] = fault(FAULT_DELAY, "op-mutated", "bounded-delay");
        operation_mutation.commands[0].operation_id = "op-mutated".to_string();
        let operation_run = run_simulation(&operation_mutation).expect("operation mutation run");

        let mut schedule_mutation = base_input.clone();
        schedule_mutation.fault_plan.events[0].duration_ticks += 1;
        let schedule_run = run_simulation(&schedule_mutation).expect("schedule mutation run");

        let mut payload_mutation = base_input.clone();
        payload_mutation.commands[0].payload_ref = local_ref("payload:mutated");
        let payload_run = run_simulation(&payload_mutation).expect("payload mutation run");

        let mut missing_evidence = base_input.clone();
        missing_evidence.commands[0].authority_ref = None;
        missing_evidence.commands[0].transport_ref = None;
        let missing_evidence_run = run_simulation(&missing_evidence).expect("missing evidence run");

        assert_ne!(base_run.topology_ref, peer_run.topology_ref);
        assert_ne!(base_run.final_state_ref, operation_run.final_state_ref);
        assert_ne!(base_run.fault_plan_ref, schedule_run.fault_plan_ref);
        assert_ne!(base_run.event_refs, payload_run.event_refs);
        assert_eq!(missing_evidence_run.decision, DENY_DECISION);
        assert_ne!(base_run.receipt_ref, missing_evidence_run.receipt_ref);
        assert_diagnostic(&missing_evidence_run, "missing-authority");
    }

    #[test]
    fn unauthorized_transport_denies_before_side_effects() {
        // r[verify molten.testing.distributed_simulation.fixtures]
        // r[verify molten.testing.distributed_simulation.property_invariants]
        let mut unauthorized = command("op-transport");
        unauthorized.authority_ref = None;
        let input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_UNAUTHORIZED_TRANSPORT, "op-transport", "transport-only")],
                caveats: Vec::new(),
            },
            vec![unauthorized],
        );

        let run = run_simulation(&input).expect("run");

        assert_eq!(run.decision, DENY_DECISION);
        assert!(run.committed_operation_ids.is_empty());
        assert_eq!(run.denied_operation_ids, vec!["op-transport".to_string()]);
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "transport-evidence-does-not-grant-authority"));
    }

    #[test]
    fn duplicate_delivery_does_not_double_commit_and_restart_replays_stably() {
        // r[verify molten.testing.distributed_simulation.property_invariants]
        let input = input_with(
            FaultPlan {
                events: vec![
                    fault(FAULT_RESTART, "op-restart", "restart-window"),
                    fault(FAULT_DUPLICATE, "op-duplicate", "duplicate-delivery"),
                ],
                caveats: Vec::new(),
            },
            vec![command("op-restart"), command("op-duplicate")],
        );

        let run = run_simulation(&input).expect("run");

        assert_eq!(run.decision, PASS_DECISION);
        assert_eq!(run.committed_operation_ids, vec!["op-restart".to_string()]);
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == DUPLICATE_SUPPRESSED_DECISION));
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic.contains("restart:restart-window")));
    }

    #[test]
    fn partitioned_quorum_and_ambient_state_deny() {
        // r[verify molten.testing.distributed_simulation.fixtures]
        let mut quorum = command("op-quorum");
        quorum.requires_quorum = true;
        let ambient = command("op-ambient");
        let input = input_with(
            FaultPlan {
                events: vec![
                    fault(FAULT_PARTITION, "op-quorum", "partition-window"),
                    fault(FAULT_AMBIENT_STATE_DRIFT, "op-ambient", "host-path-drift"),
                ],
                caveats: Vec::new(),
            },
            vec![quorum, ambient],
        );

        let run = run_simulation(&input).expect("run");

        assert_eq!(run.decision, DENY_DECISION);
        assert_eq!(run.denied_operation_ids, vec!["op-ambient".to_string(), "op-quorum".to_string()]);
        assert!(
            run.diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "partitioned-quorum-denied-before-side-effects")
        );
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "undeclared-ambient-state"));
    }

    fn composite_case(
        case_id: &str,
        invariant_name: &str,
        faults: Vec<FaultEvent>,
        commands: Vec<SimulationCommand>,
        expected_decision: &str,
    ) -> CompositeFaultCase {
        CompositeFaultCase {
            case_id: case_id.to_string(),
            invariant_name: invariant_name.to_string(),
            simulation: input_with(
                FaultPlan {
                    events: faults,
                    caveats: vec!["composite deterministic fault plan".to_string()],
                },
                commands,
            ),
            expected_decision: expected_decision.to_string(),
            profile_eligibility: vec![PROFILE_PROTOCOL.to_string()],
            cost_class: COST_FAST.to_string(),
            caveats: vec!["composite simulation evidence does not satisfy VM or production claims".to_string()],
        }
    }

    fn composite_suite_cases() -> Vec<CompositeFaultCase> {
        let mut partitioned_quorum = command("op-partition-quorum");
        partitioned_quorum.requires_quorum = true;
        let mut pressure_quorum = command("op-pressure-quorum");
        pressure_quorum.requires_quorum = true;
        vec![
            composite_case(
                "duplicate-after-restart",
                "duplicate commit suppression after restart",
                vec![
                    fault(FAULT_RESTART, "op-restart", "restart-window"),
                    fault(FAULT_DUPLICATE, "op-duplicate", "duplicate-delivery"),
                ],
                vec![command("op-restart"), command("op-duplicate")],
                PASS_DECISION,
            ),
            composite_case(
                "partition-with-stale-evidence",
                "partitioned quorum denies stale evidence",
                vec![
                    fault(FAULT_PARTITION, "op-partition-quorum", "minority-partition"),
                    fault(FAULT_STALE_EVIDENCE, "op-partition-quorum", "stale-ledger"),
                ],
                vec![partitioned_quorum],
                DENY_DECISION,
            ),
            composite_case(
                "reorder-with-reconcile",
                "reorder still reconciles ack evidence",
                vec![fault(FAULT_REORDER, "op-reorder", "ack-reordered")],
                vec![command("op-reorder")],
                PASS_DECISION,
            ),
            composite_case(
                "crash-during-dispatch",
                "crash during dispatch replays deterministically",
                vec![fault(FAULT_CRASH, "op-crash", "dispatch-crash")],
                vec![command("op-crash")],
                PASS_DECISION,
            ),
            composite_case(
                "resource-pressure-during-quorum",
                "resource pressure denies before quorum side effects",
                vec![fault(FAULT_RESOURCE_PRESSURE, "op-pressure-quorum", "budget-pressure")],
                vec![pressure_quorum],
                DENY_DECISION,
            ),
        ]
    }

    #[test]
    fn composite_fault_suite_accepts_named_positive_and_negative_regressions() {
        // r[verify molten.testing.distributed_simulation.composite_fault_regression_suite]
        let suite = evaluate_composite_fault_suite(&composite_suite_cases()).expect("composite suite");
        let rendered = crate::preserves_rail::to_text(&suite.value).expect("render composite suite");

        assert_eq!(suite.decision, PASS_DECISION);
        assert_eq!(suite.case_refs.len(), composite_suite_cases().len());
        assert_eq!(suite.run_refs.len(), suite.case_refs.len());
        assert!(suite.diagnostics.is_empty());
        assert!(rendered.contains("composite-fault-suite-v1"));
        assert!(rendered.contains("simulation-evidence-not-vm-evidence"));
    }

    #[test]
    fn composite_fault_suite_denies_mismatched_expected_decision() {
        // r[verify molten.testing.distributed_simulation.composite_fault_regression_suite]
        let mut stale_case = composite_case(
            "stale-evidence-claimed-pass",
            "stale evidence cannot pass by retry",
            vec![fault(FAULT_STALE_EVIDENCE, "op-stale", "stale-ledger")],
            vec![command("op-stale")],
            PASS_DECISION,
        );
        stale_case.profile_eligibility = Vec::new();
        let suite = evaluate_composite_fault_suite(&[stale_case]).expect("composite suite");

        assert_eq!(suite.decision, DENY_DECISION);
        assert!(
            suite
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "composite-case-decision-mismatch:stale-evidence-claimed-pass")
        );
        assert!(
            suite
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "composite-case-missing-profile:stale-evidence-claimed-pass")
        );
    }

    fn promotion_input() -> GeneratedCasePromotionInput {
        GeneratedCasePromotionInput {
            case_id: "generated-partition-stale".to_string(),
            invariant_name: "partitioned stale evidence denies".to_string(),
            seed_ref: local_ref("seed"),
            topology_ref: local_ref("topology"),
            scheduler_ref: local_ref("scheduler"),
            fault_plan_ref: local_ref("fault-plan"),
            command_refs: vec![local_ref("command")],
            replay_ref: local_ref("replay"),
            diagnostic_refs: vec![local_ref("diagnostic")],
            profile_eligibility: vec![PROFILE_PROTOCOL.to_string()],
            traceability_refs: vec![local_ref("traceability")],
            retry_attempts: 0,
            variance_refs: vec![local_ref("variance:none")],
            cost_class: COST_FAST.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
            diagnostic_only: false,
            caveats: vec!["promotion evidence remains simulation scoped".to_string()],
        }
    }
