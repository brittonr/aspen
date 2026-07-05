    use super::*;

    const SIMULATION_MAX_TICKS: u64 = 32;
    const FAULT_START_TICK: u64 = 1;
    const FAULT_DURATION_TICKS: u64 = 2;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn topology() -> Topology {
        Topology {
            peers: vec![
                Peer {
                    id: "peer-a".to_string(),
                    roles: vec!["sender".to_string()],
                },
                Peer {
                    id: "peer-b".to_string(),
                    roles: vec!["receiver".to_string()],
                },
            ],
            channels: vec![Channel {
                id: "a-to-b".to_string(),
                from_peer: "peer-a".to_string(),
                to_peer: "peer-b".to_string(),
                topic: "node-control".to_string(),
            }],
            caveats: vec!["simulation evidence is review evidence only".to_string()],
        }
    }

    fn scheduler() -> SchedulerProfile {
        SchedulerProfile {
            id: "round-robin".to_string(),
            policy: "deterministic-virtual-clock".to_string(),
            max_ticks: SIMULATION_MAX_TICKS,
        }
    }

    fn seed() -> SimulationSeed {
        SimulationSeed {
            id: "seed-1".to_string(),
            entropy_ref: local_ref("seed-1"),
        }
    }

    fn command(operation_id: &str) -> SimulationCommand {
        SimulationCommand {
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

    fn input_with(plan: FaultPlan, commands: Vec<SimulationCommand>) -> SimulationInput {
        SimulationInput {
            topology: topology(),
            scheduler: scheduler(),
            seed: seed(),
            fault_plan: plan,
            source_ref: local_ref("source-tree"),
            test_binary_ref: local_ref("test-binary"),
            commands,
            child_workflow_refs: vec![local_ref("child-workflow")],
            allowed_variance_refs: vec![local_ref("variance:none")],
        }
    }

    fn fault(kind: &str, operation_id: &str, diagnostic: &str) -> FaultEvent {
        FaultEvent {
            kind: kind.to_string(),
            target_kind: "operation".to_string(),
            target: operation_id.to_string(),
            operation_id: Some(operation_id.to_string()),
            start_tick: FAULT_START_TICK,
            duration_ticks: FAULT_DURATION_TICKS,
            diagnostic: diagnostic.to_string(),
        }
    }

    fn run_twice(input: &SimulationInput) -> (SimulationRun, SimulationRun) {
        let first = run_simulation(input).expect("first distributed simulation run");
        let second = run_simulation(input).expect("second distributed simulation run");
        (first, second)
    }

    fn assert_stable_run_refs(first: &SimulationRun, second: &SimulationRun) {
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.final_state_ref, second.final_state_ref);
        assert_eq!(first.event_refs, second.event_refs);
        assert_eq!(first.fault_plan_ref, second.fault_plan_ref);
    }

    fn event_for<'a>(run: &'a SimulationRun, operation_id: &str) -> &'a SimulationEventOutcome {
        run.event_outcomes
            .iter()
            .find(|event| event.operation_id == operation_id)
            .expect("event for operation")
    }

    fn assert_event_outcome(
        run: &SimulationRun,
        operation_id: &str,
        expected_kind: &str,
        expected_decision: &str,
        expected_diagnostic: &str,
    ) {
        let event = event_for(run, operation_id);
        let recomputed_ref = canonical_ref(&event.value).expect("event canonical ref");
        assert_eq!(event.kind, expected_kind);
        assert_eq!(event.decision, expected_decision);
        assert_eq!(event.diagnostic, expected_diagnostic);
        assert_eq!(event.event_ref, recomputed_ref);
        assert!(run.event_refs.iter().any(|event_ref| event_ref == &event.event_ref));
    }

    fn assert_diagnostic(run: &SimulationRun, expected_diagnostic: &str) {
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == expected_diagnostic));
    }

    #[test]
    fn simulation_is_deterministic_and_parseable() {
        // r[verify molten.testing.distributed_simulation.fault_plan_schema]
        // r[verify molten.testing.distributed_simulation.simulator_core]
        // r[verify molten.testing.distributed_simulation.run_receipts]
        let input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_DELAY, "op-1", "bounded-delay")],
                caveats: vec!["delay is virtual".to_string()],
            },
            vec![command("op-1")],
        );

        let first = run_simulation(&input).expect("first run");
        let second = run_simulation(&input).expect("second run");
        let parsed = parse_test_run(&first.value).expect("parse run");

        assert_eq!(first.decision, PASS_DECISION);
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.final_state_ref, second.final_state_ref);
        assert_eq!(parsed.topology_ref, first.topology_ref);
        assert_eq!(parsed.event_refs, first.event_refs);
    }

    #[test]
    fn changing_fault_plan_changes_identity() {
        // r[verify molten.testing.distributed_simulation.fault_plan_schema]
        let base = FaultPlan {
            events: vec![fault(FAULT_DROP, "op-1", "drop-one")],
            caveats: Vec::new(),
        };
        let changed = FaultPlan {
            events: vec![fault(FAULT_REORDER, "op-1", "reorder-one")],
            caveats: Vec::new(),
        };
        let base_ref = canonical_ref(&fault_plan_value(&base).expect("base plan")).expect("base ref");
        let changed_ref = canonical_ref(&fault_plan_value(&changed).expect("changed plan")).expect("changed ref");

        assert_ne!(base_ref, changed_ref);
    }

    #[test]
    fn direct_positive_fault_fixtures_cover_benign_faults() {
        // r[verify molten.testing.distributed_simulation.direct_fault_fixtures]
        let fixtures = [
            (FAULT_DELAY, "op-delay", "bounded-delay", COMMIT_EVENT_KIND),
            (FAULT_DROP, "op-drop", "declared-drop", COMMIT_EVENT_KIND),
            (FAULT_REORDER, "op-reorder", "stable-reorder", COMMIT_EVENT_KIND),
            (FAULT_REJOIN, "op-rejoin", "peer-rejoined", COMMIT_EVENT_KIND),
            (FAULT_CRASH, "op-crash", "crash-replayed", REPLAY_EVENT_KIND),
            (FAULT_RESTART, "op-restart", "restart-replayed", REPLAY_EVENT_KIND),
        ];

        for (fault_kind, operation_id, diagnostic, event_kind) in fixtures {
            let input = input_with(
                FaultPlan {
                    events: vec![fault(fault_kind, operation_id, diagnostic)],
                    caveats: vec!["direct benign fixture".to_string()],
                },
                vec![command(operation_id)],
            );
            let (first, second) = run_twice(&input);
            let expected_diagnostic = format!("{fault_kind}:{diagnostic}");

            assert_stable_run_refs(&first, &second);
            assert_eq!(first.decision, PASS_DECISION);
            assert_eq!(first.committed_operation_ids, vec![operation_id.to_string()]);
            assert!(first.denied_operation_ids.is_empty());
            assert_event_outcome(&first, operation_id, event_kind, PASS_DECISION, &expected_diagnostic);
            assert_diagnostic(&first, &expected_diagnostic);
        }

        let duplicate_input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_DUPLICATE, "op-duplicate", "duplicate-delivery")],
                caveats: vec!["direct duplicate fixture".to_string()],
            },
            vec![command("op-duplicate")],
        );
        let (duplicate_first, duplicate_second) = run_twice(&duplicate_input);

        assert_stable_run_refs(&duplicate_first, &duplicate_second);
        assert_eq!(duplicate_first.decision, PASS_DECISION);
        assert!(duplicate_first.committed_operation_ids.is_empty());
        assert!(duplicate_first.denied_operation_ids.is_empty());
        assert_event_outcome(
            &duplicate_first,
            "op-duplicate",
            DUPLICATE_EVENT_KIND,
            PASS_DECISION,
            DUPLICATE_SUPPRESSED_DECISION,
        );
        assert_diagnostic(&duplicate_first, DUPLICATE_SUPPRESSED_DECISION);
    }

    #[test]
    fn direct_negative_fault_fixtures_deny_before_side_effects() {
        // r[verify molten.testing.distributed_simulation.direct_fault_fixtures]
        struct NegativeFaultFixture {
            fault_kind: &'static str,
            operation_id: &'static str,
            fault_diagnostic: &'static str,
            expected_diagnostic: &'static str,
            requires_quorum: bool,
            drop_authority: bool,
        }

        let fixtures = [
            NegativeFaultFixture {
                fault_kind: FAULT_STALE_EVIDENCE,
                operation_id: "op-stale",
                fault_diagnostic: "stale-ledger-ref",
                expected_diagnostic: "stale-evidence-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_CORRUPTED_RECEIPT,
                operation_id: "op-corrupt",
                fault_diagnostic: "tampered-receipt",
                expected_diagnostic: "corrupted-receipt-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_RESOURCE_PRESSURE,
                operation_id: "op-pressure",
                fault_diagnostic: "budget-exhausted",
                expected_diagnostic: "resource-pressure-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_UNAUTHORIZED_TRANSPORT,
                operation_id: "op-transport",
                fault_diagnostic: "transport-only",
                expected_diagnostic: "transport-evidence-does-not-grant-authority",
                requires_quorum: false,
                drop_authority: true,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_AMBIENT_STATE_DRIFT,
                operation_id: "op-ambient",
                fault_diagnostic: "host-path-drift",
                expected_diagnostic: "undeclared-ambient-state",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_PARTITION,
                operation_id: "op-quorum",
                fault_diagnostic: "partition-window",
                expected_diagnostic: "partitioned-quorum-denied-before-side-effects",
                requires_quorum: true,
                drop_authority: false,
            },
        ];

        for fixture in fixtures {
            let mut candidate = command(fixture.operation_id);
            candidate.requires_quorum = fixture.requires_quorum;
            if fixture.drop_authority {
                candidate.authority_ref = None;
            }
            let input = input_with(
                FaultPlan {
                    events: vec![fault(
                        fixture.fault_kind,
                        fixture.operation_id,
                        fixture.fault_diagnostic,
                    )],
                    caveats: vec!["direct negative fixture".to_string()],
                },
                vec![candidate],
            );
            let (first, second) = run_twice(&input);

            assert_stable_run_refs(&first, &second);
            assert_eq!(first.decision, DENY_DECISION);
            assert!(first.committed_operation_ids.is_empty());
            assert_eq!(first.denied_operation_ids, vec![fixture.operation_id.to_string()]);
            assert_event_outcome(
                &first,
                fixture.operation_id,
                DENY_EVENT_KIND,
                DENY_DECISION,
                fixture.expected_diagnostic,
            );
            assert_diagnostic(&first, fixture.expected_diagnostic);
        }
    }
