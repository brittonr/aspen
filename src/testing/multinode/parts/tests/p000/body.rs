    const REPLAYABLE_SIMULATION: &str = "deterministic-simulation-replayable";

    use super::*;

    const SIMULATION_MAX_TICKS: u64 = 32;
    const FAULT_START_TICK: u64 = 1;
    const FAULT_DURATION_TICKS: u64 = 2;
    const THREE_NODE_PROFILE_ROLE_COUNT: usize = 3;

    fn local_ref(label: &str) -> String {
        content_ref_from_text(label)
    }

    fn execution_profiles() -> Vec<crate::distributed_core::CiProfile> {
        crate::distributed_core::default_ci_profiles()
    }

    fn protocol_profile() -> crate::distributed_core::CiProfile {
        execution_profiles().into_iter().find(|profile| profile.id == "protocol").expect("protocol profile")
    }

    fn valid_fixture() -> ScenarioFixture {
        let profile = protocol_profile();
        ScenarioFixture {
            scenario_id: "protocol-pairwise-smoke".to_string(),
            purpose: "review pairwise protocol metadata".to_string(),
            evidence_scope: "simulated distributed protocol evidence".to_string(),
            topology_profile_id: PROFILE_PAIRWISE_TRANSPORT.to_string(),
            execution_profile_id: profile.id,
            command_surface: profile.command,
            expected_artifact_kinds: profile.expected_artifact_kinds,
            topology_ref: local_ref("topology"),
            seed_ref: local_ref("seed"),
            fault_plan_ref: local_ref("fault-plan"),
            receipt_refs: vec![local_ref("receipt")],
            variance_refs: vec![local_ref("variance:none")],
            diagnostic_log_refs: vec![local_ref("log")],
            unavailable_policy: "unavailable-is-deny".to_string(),
            unsupported_claims_pass: false,
            caveats: vec!["fixture evidence is review evidence only".to_string()],
        }
    }

    #[test]
    fn declarative_scenario_fixture_derives_stable_metadata() {
        // r[verify molten.testing.multinode.declarative_scenario_fixtures]
        // r[verify molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
        let fixture = valid_fixture();
        let topology_profiles = default_topology_profiles();
        let first =
            derive_scenario_metadata(&fixture, &execution_profiles(), &topology_profiles).expect("first metadata");
        let second =
            derive_scenario_metadata(&fixture, &execution_profiles(), &topology_profiles).expect("second metadata");
        let rendered = crate::preserves_rail::to_text(&first.value).expect("render metadata");

        assert_eq!(first.decision, PASS_DECISION);
        assert_eq!(first.metadata_ref, second.metadata_ref);
        assert_eq!(first.fixture_ref, second.fixture_ref);
        assert!(first.diagnostics.is_empty());
        assert!(rendered.contains("multinode-scenario-metadata-v1"));
        assert!(rendered.contains("profile"));
    }

    #[test]
    fn declarative_scenario_fixture_validation_denies_bad_bindings() {
        // r[verify molten.testing.multinode.scenario_fixture_validation]
        // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
        let mut fixture = valid_fixture();
        fixture.command_surface = "cargo test wrong-profile".to_string();
        fixture.receipt_refs = Vec::new();
        fixture.variance_refs = Vec::new();
        fixture.unsupported_claims_pass = true;
        fixture.expected_artifact_kinds = vec!["wrong-kind".to_string()];
        let metadata = derive_scenario_metadata(&fixture, &execution_profiles(), &default_topology_profiles())
            .expect("denied metadata");

        assert_eq!(metadata.decision, DENY_DECISION);
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-command-profile-mismatch"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-artifact-kind-mismatch"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-missing-receipt-ref"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-missing-variance-ref"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-unsupported-pass-claim"));
    }

    #[test]
    fn topology_profile_matrix_binds_profiles_and_membership_scope() {
        // r[verify molten.testing.multinode.topology_profile_matrix]
        let profiles = default_topology_profiles();
        let matrix = build_topology_matrix(&profiles).expect("topology matrix");
        let rendered = crate::preserves_rail::to_text(&matrix.value).expect("render matrix");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert_eq!(matrix.profile_refs.len(), REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT);
        assert!(matrix.diagnostics.is_empty());
        assert!(rendered.contains(PROFILE_CONTROL_QUORUM));
        assert!(rendered.contains("role-membership-explicit"));
    }

    #[test]
    fn topology_membership_negatives_deny_role_confusion() {
        // r[verify molten.testing.multinode.role_membership_negatives]
        let profile = default_topology_profiles()
            .into_iter()
            .find(|profile| profile.id == PROFILE_SUBSCRIBER_PEER)
            .expect("subscriber profile");
        let claim = TopologyMembershipClaim {
            profile_id: profile.id.clone(),
            topology_ref: local_ref("topology-a"),
            scenario_topology_ref: local_ref("topology-b"),
            node_roles: vec![role("subscriber", ROLE_VOTER, MEMBERSHIP_VOTER)],
            quorum_ref: None,
            transport_only_authority_claim: true,
            caveats: vec!["negative fixture".to_string()],
        };
        let gate = evaluate_topology_membership_claim(&profile, &claim).expect("membership gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "wrong-topology"));
        assert!(gate.diagnostics.iter().any(|item| item == "subscriber-promoted-to-voter:subscriber"));
        assert!(gate.diagnostics.iter().any(|item| item == "transport-only-authority-claim"));
    }

    fn node_summary(node: &str, queue_ref: String, commit_ref: String) -> NodeSummary {
        NodeSummary {
            node_id: node.to_string(),
            topology_ref: local_ref("topology"),
            scenario_fixture_ref: local_ref("fixture"),
            receipt_refs: vec![local_ref("receipt:workflow"), local_ref(&format!("receipt:{node}"))],
            queue_ref,
            ledger_ref: local_ref("ledger"),
            dispatch_ref: local_ref("dispatch"),
            ack_ref: local_ref("ack"),
            protocol_ref: local_ref("protocol"),
            semantic_commits: vec![SemanticCommitEvidence {
                operation_id: "op-1".to_string(),
                commit_ref,
            }],
            diagnostic_log_refs: vec![local_ref(&format!("log:{node}"))],
        }
    }

    fn reconciliation_input() -> ReconciliationInput {
        let shared_queue = local_ref("queue");
        let shared_commit = local_ref("commit:op-1");
        ReconciliationInput {
            topology_ref: local_ref("topology"),
            scenario_fixture_ref: local_ref("fixture"),
            required_receipt_refs: vec![local_ref("receipt:workflow")],
            node_summaries: vec![
                node_summary("node-a", shared_queue.clone(), shared_commit.clone()),
                node_summary("node-b", shared_queue.clone(), shared_commit),
            ],
            equality_classes: vec![ReconciliationEqualityClass {
                name: "queue".to_string(),
                refs: vec![shared_queue.clone(), shared_queue],
                variance_ref: None,
            }],
            allowed_variance_refs: vec![local_ref("variance:clock")],
            caveats: vec!["reconciliation evidence is scoped".to_string()],
        }
    }

    #[test]
    fn reconciliation_gate_passes_converged_nodes_and_declared_variance() {
        // r[verify molten.testing.multinode.cross_node_reconciliation_gate]
        let mut input = reconciliation_input();
        input.equality_classes.push(ReconciliationEqualityClass {
            name: "runtime-log".to_string(),
            refs: vec![local_ref("log-a"), local_ref("log-b")],
            variance_ref: Some(local_ref("variance:clock")),
        });
        let gate = evaluate_reconciliation(&input).expect("reconciliation gate");
        let rendered = crate::preserves_rail::to_text(&gate.value).expect("render reconciliation");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
        assert!(rendered.contains("multinode-reconciliation-gate-v1"));
        assert!(rendered.contains("allowed-variance"));
    }

    #[test]
    fn reconciliation_gate_denies_divergence_and_duplicate_commit() {
        // r[verify molten.testing.multinode.reconciliation_deny_drift]
        let mut input = reconciliation_input();
        input.node_summaries[1].queue_ref = local_ref("queue-divergent");
        input.node_summaries[1].semantic_commits[0].commit_ref = local_ref("commit:op-1-duplicate");
        input.equality_classes[0].refs = vec![local_ref("queue"), local_ref("queue-divergent")];
        let gate = evaluate_reconciliation(&input).expect("reconciliation gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "divergent-ref-class:queue"));
        assert!(gate.diagnostics.iter().any(|item| item == "duplicate-semantic-commit:op-1"));
    }

    fn local_plan_input() -> LocalMultiprocessPlanInput {
        LocalMultiprocessPlanInput {
            fixture_ref: local_ref("fixture"),
            nodes: vec![
                LocalProcessNodePlan {
                    node_id: "node-a".to_string(),
                    state_root_handle: "state-a".to_string(),
                    transport_handle: "transport-a".to_string(),
                },
                LocalProcessNodePlan {
                    node_id: "node-b".to_string(),
                    state_root_handle: "state-b".to_string(),
                    transport_handle: "transport-b".to_string(),
                },
            ],
            command_plan_ref: local_ref("command-plan"),
            expected_receipt_refs: vec![local_ref("startup"), local_ref("workflow"), local_ref("cleanup")],
            cleanup_policy: CLEANUP_POLICY_REQUIRED.to_string(),
            caveats: vec!["local integration evidence only".to_string()],
        }
    }

    #[test]
    fn local_multiprocess_plan_and_run_bind_isolated_process_evidence() {
        // r[verify molten.testing.multinode.local_multiprocess_harness]
        // r[verify molten.testing.local_multiprocess_cluster_tier.middle_tier]
        let plan = build_local_multiprocess_plan(&local_plan_input()).expect("local plan");
        let run = build_local_multiprocess_run_receipt(&LocalMultiprocessRunInput {
            plan_ref: plan.plan_ref.clone(),
            startup_refs: vec![local_ref("startup-a"), local_ref("startup-b")],
            workflow_refs: vec![local_ref("workflow")],
            shutdown_refs: vec![local_ref("shutdown-a"), local_ref("shutdown-b")],
            cleanup_refs: vec![local_ref("cleanup")],
            diagnostics: Vec::new(),
            caveats: vec!["local multiprocess evidence is not VM evidence".to_string()],
        })
        .expect("local run");

        assert_eq!(plan.decision, PASS_DECISION);
        assert_eq!(run.decision, PASS_DECISION);
        assert!(run.receipt_ref.starts_with("blake3:"));
    }

    #[test]
    fn local_multiprocess_plan_denies_collisions_and_missing_cleanup() {
        // r[verify molten.testing.multinode.process_isolation_cleanup]
        // r[verify molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
        let mut input = local_plan_input();
        input.nodes[1].state_root_handle = input.nodes[0].state_root_handle.clone();
        input.nodes[1].transport_handle = input.nodes[0].transport_handle.clone();
        input.cleanup_policy = String::new();
        let plan = build_local_multiprocess_plan(&input).expect("denied local plan");

        assert_eq!(plan.decision, DENY_DECISION);
        assert!(plan.diagnostics.iter().any(|item| item.contains("state-root-collision")));
        assert!(plan.diagnostics.iter().any(|item| item.contains("transport-collision")));
        assert!(plan.diagnostics.iter().any(|item| item == "local-plan-missing-cleanup-policy"));
    }

