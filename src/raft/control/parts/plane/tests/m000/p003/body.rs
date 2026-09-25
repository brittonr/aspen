
    #[test]
    fn consensus_placement_and_simulation_cover_positive_and_negative_paths() {
        // r[verify molten.consensus.replica_placement_evidence]
        // r[verify molten.testing.consensus_fault_matrix]
        // r[verify molten.testing.leaderless_experimental_fixtures]
        // r[verify molten.testing.consensus_placement_fixtures]
        let members = vec![test_ref("member-a"), test_ref("member-b"), test_ref("member-c")];
        let placement_input = ConsensusPlacementInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            candidate_members: members.clone(),
            admitted_members: members.clone(),
            fault_domain_refs: vec![test_ref("domain-a"), test_ref("domain-b"), test_ref("domain-c")],
            fault_domain_policy_ref: test_ref("fault-policy"),
            membership_refs: vec![test_ref("membership")],
            placement_policy_refs: vec![test_ref("placement-policy")],
            majority_reachable: true,
            latency_diagnostics: vec!["bounded-fixture-latency".to_string()],
            denied_candidates: Vec::new(),
            refresh_refs: vec![test_ref("refresh")],
        };
        let placement = consensus_placement_report(&placement_input).expect("placement pass");
        assert_eq!(placement.decision, RAFT_DECISION_PASS);
        assert_eq!(crate::ledger::artifact_kind(&placement.value), "consensus-placement-report");

        let unsafe_placement = consensus_placement_report(&ConsensusPlacementInput {
            fault_domain_refs: vec![test_ref("domain-shared")],
            membership_refs: Vec::new(),
            majority_reachable: false,
            latency_diagnostics: Vec::new(),
            refresh_refs: Vec::new(),
            ..placement_input
        })
        .expect("placement deny");
        assert_eq!(unsafe_placement.decision, RAFT_DECISION_DENY);
        assert!(unsafe_placement.diagnostics.join(";").contains("majority"));

        let majority_input = ConsensusSimulationInput {
            scenario: SCENARIO_MAJORITY_PROGRESS.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members.clone(),
            fault_plan_ref: test_ref("fault-plan"),
            operation_ids: vec![test_ref("operation")],
            connected_replicas: members.len(),
            proposer_ref: Some(test_ref("member-a")),
            required_evidence_refs: vec![test_ref("raft-evidence")],
            placement_ref: Some(placement.report_ref.clone()),
            local_state_fresh: true,
            requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        };
        let majority = run_consensus_simulation(&majority_input).expect("majority simulation");
        assert_eq!(majority.decision, RAFT_DECISION_PASS);
        assert!(majority.final_state_ref.is_some());
        assert_eq!(crate::ledger::artifact_kind(&majority.value), "consensus-simulation-receipt");

        let minority = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_MINORITY_DENIAL.to_string(),
            fault_plan_ref: test_ref("minority-fault"),
            connected_replicas: MINORITY_CONNECTED_REPLICAS,
            local_state_fresh: false,
            ..majority_input.clone()
        })
        .expect("minority simulation");
        assert_eq!(minority.decision, RAFT_DECISION_PASS);
        assert!(minority.final_state_ref.is_some());

        assert_stale_reads_classified(&majority_input);
        assert_leaderless_progress_requires_evidence(majority_input);
    }

    /// A stale linearizable read is denied for freshness, and the same read at local-stale consistency passes.
    fn assert_stale_reads_classified(majority_input: &ConsensusSimulationInput) {
        let stale_input = ConsensusSimulationInput {
            scenario: SCENARIO_STALE_READ_CLASSIFICATION.to_string(),
            fault_plan_ref: test_ref("stale-read"),
            operation_ids: Vec::new(),
            proposer_ref: None,
            local_state_fresh: false,
            ..majority_input.clone()
        };
        let stale_linearizable = run_consensus_simulation(&stale_input).expect("stale linearizable");
        assert_eq!(stale_linearizable.decision, RAFT_DECISION_DENY);
        assert!(stale_linearizable.diagnostics.join(";").contains("freshness"));

        let local_stale = run_consensus_simulation(&ConsensusSimulationInput {
            requested_read_consistency: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            ..stale_input
        })
        .expect("local stale simulation");
        assert_eq!(local_stale.decision, RAFT_DECISION_PASS);
    }

    /// Leaderless experimental progress from a non-leader passes with proof and simulation evidence, and is denied
    /// without it.
    fn assert_leaderless_progress_requires_evidence(majority_input: ConsensusSimulationInput) {
        let leaderless_input = ConsensusSimulationInput {
            scenario: SCENARIO_LEADERLESS_NON_LEADER_PROGRESS.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            fault_plan_ref: test_ref("leaderless-fault"),
            operation_ids: vec![test_ref("leaderless-operation")],
            proposer_ref: Some(test_ref("member-b")),
            required_evidence_refs: vec![test_ref("proof"), test_ref("simulation")],
            ..majority_input
        };
        let leaderless = run_consensus_simulation(&leaderless_input).expect("leaderless experimental simulation");
        assert_eq!(leaderless.decision, RAFT_DECISION_PASS);

        let missing_leaderless_evidence = run_consensus_simulation(&ConsensusSimulationInput {
            connected_replicas: EXPERIMENTAL_CONNECTED_REPLICAS,
            required_evidence_refs: Vec::new(),
            placement_ref: None,
            ..leaderless_input
        })
        .expect("leaderless missing evidence");
        assert_eq!(missing_leaderless_evidence.decision, RAFT_DECISION_DENY);
        assert!(missing_leaderless_evidence.diagnostics.join(";").contains("missing required evidence"));
    }

    #[test]
    fn engine_registry_admission_and_runtime_selection_are_fail_closed() {
        // r[verify molten.consensus.engine_registry]
        // r[verify molten.consensus.engine_admission_policy]
        // r[verify molten.consensus.runtime_engine_selection]
        let registry = default_consensus_engine_registry().expect("engine registry");
        assert_eq!(registry.entries.len(), DEFAULT_CONSENSUS_ENGINE_REGISTRY_LEN);
        assert_eq!(crate::ledger::artifact_kind(&registry.value), "consensus-engine-registry");

        let required = vec![ENGINE_CAPABILITY_PROPOSAL.to_string(), ENGINE_CAPABILITY_LINEARIZABLE_READ.to_string()];
        let admit = |algorithm_profile: &str, profile_version: &str, environment: &str, required_capabilities| {
            admit_consensus_engine(&registry, &ConsensusEngineAdmissionInput {
                algorithm_profile: algorithm_profile.to_string(),
                profile_version: profile_version.to_string(),
                requested_environment: environment.to_string(),
                requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
                required_capabilities,
            })
        };
        let production_admission = admit(CONSENSUS_PROFILE_RAFT, CONSENSUS_PROFILE_VERSION_RAFT, CONSENSUS_ENVIRONMENT_PRODUCTION, required.clone())
        .expect("production admission receipt");
        assert_eq!(production_admission.decision, ENGINE_DECISION_DENY);
        assert!(production_admission.diagnostics.join(";").contains("not admitted for production runtime"));

        let model_admission = admit(CONSENSUS_PROFILE_RAFT, CONSENSUS_PROFILE_VERSION_RAFT, CONSENSUS_ENVIRONMENT_MODEL, required.clone())
        .expect("model admission");
        assert_eq!(model_admission.decision, ENGINE_DECISION_PASS);
        assert!(model_admission.descriptor.is_some());
        assert_eq!(crate::ledger::artifact_kind(&model_admission.value), "consensus-engine-admission-receipt");
        let descriptor = model_admission.descriptor.as_ref().expect("descriptor");
        assert!(consensus_engine_readback_summary(descriptor).contains("model-only-denied-production"));

        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let runtime = new_control_registry_model_runtime(&manifest).expect("model runtime selected through registry");
        assert!(control_registry_summary(&runtime).contains("engine=in-process-raft-control-registry-v1"));
        let production = new_control_registry_production_runtime(&manifest).expect_err("model profile denied in production");
        assert!(production.to_string().contains("not admitted for production runtime"));

        let unknown = admit("unknown-profile", "unknown-v1", CONSENSUS_ENVIRONMENT_PRODUCTION, required.clone())
        .expect("unknown admission");
        assert_eq!(unknown.decision, ENGINE_DECISION_DENY);
        assert!(unknown.diagnostics.join(";").contains("unsupported consensus engine profile"));

        let disabled = admit("disabled-fixture-engine", "disabled-fixture-v1", CONSENSUS_ENVIRONMENT_PRODUCTION, required.clone())
        .expect("disabled admission");
        assert_eq!(disabled.decision, ENGINE_DECISION_DENY);
        assert!(disabled.diagnostics.join(";").contains("disabled"));

        let leaderless = admit(CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL, CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL, CONSENSUS_ENVIRONMENT_PRODUCTION, required)
        .expect("leaderless admission");
        assert_eq!(leaderless.decision, ENGINE_DECISION_DENY);
        assert!(leaderless.diagnostics.join(";").contains("not admitted for production"));

        let mismatch = admit(CONSENSUS_PROFILE_RAFT, "wrong-version", CONSENSUS_ENVIRONMENT_PRODUCTION, vec![ENGINE_CAPABILITY_PROPOSAL.to_string()])
        .expect("version mismatch");
        assert_eq!(mismatch.decision, ENGINE_DECISION_DENY);
        assert!(mismatch.diagnostics.join(";").contains("version mismatch"));
    }

    #[test]
    fn normalized_receipts_switchover_and_epoch_gates_cover_safe_and_unsafe_paths() {
        // r[verify molten.consensus.engine_interface]
        // r[verify molten.consensus.engine_switchover_receipts]
        // r[verify molten.consensus.engine_switchover_fencing]
        let mut runtime = new_control_registry_runtime(&control_registry_fixture_manifest_value().expect("manifest"))
            .expect("runtime");
        let command = command_for_receipt_index("normalized", "normalized-target");
        let envelope = envelope_for(&runtime, "client:normalized", RAFT_TEST_INITIAL_SEQUENCE, command);
        let proposal = propose_control_registry_command(&mut runtime, &envelope).expect("proposal");
        let normalized_commit = parse_consensus_engine_receipt(
            &normalized_raft_commit_receipt_value(&proposal.commit_receipt, INITIAL_CONSENSUS_ENGINE_EPOCH)
                .expect("normalized commit"),
        )
        .expect("parse normalized commit");
        assert_eq!(normalized_commit.decision, ENGINE_DECISION_PASS);
        assert_eq!(normalized_commit.receipt_kind, NORMALIZED_RECEIPT_KIND_COMMIT);
        assert_eq!(crate::ledger::artifact_kind(&normalized_commit.value), "consensus-engine-receipt");

        let read = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index,
            read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
            namespace: "receipt-index".to_string(),
            name: "normalized".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("read");
        let normalized_read = parse_consensus_engine_receipt(
            &normalized_raft_read_receipt_value(&read, INITIAL_CONSENSUS_ENGINE_EPOCH).expect("normalized read"),
        )
        .expect("parse normalized read");
        assert_eq!(normalized_read.receipt_kind, NORMALIZED_RECEIPT_KIND_READ);
        assert_eq!(normalized_read.engine_epoch, INITIAL_CONSENSUS_ENGINE_EPOCH);

        let next_epoch = INITIAL_CONSENSUS_ENGINE_EPOCH.saturating_add(NEXT_CONSENSUS_ENGINE_EPOCH_STEP);
        let switchover_input = ConsensusEngineSwitchoverInput {
            source_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            source_version: CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL.to_string(),
            target_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            target_version: CONSENSUS_PROFILE_VERSION_RAFT.to_string(),
            active_engine_epoch: INITIAL_CONSENSUS_ENGINE_EPOCH,
            target_engine_epoch: next_epoch,
            source_state_ref: runtime.state.state_ref.clone(),
            target_bootstrap_state_ref: runtime.state.state_ref.clone(),
            membership_refs: vec![test_ref("membership")],
            placement_refs: vec![test_ref("placement")],
            replay_conformance_refs: vec![normalized_commit.receipt_ref.clone()],
            currentness_evidence_refs: vec![normalized_read.receipt_ref.clone()],
            operator_approval_refs: vec![test_ref("operator-approval")],
            rollback_posture: "rollback-supported".to_string(),
        };
        let switchover = consensus_engine_model_switchover_receipt(&switchover_input).expect("model switchover");
        assert_eq!(switchover.decision, ENGINE_DECISION_PASS);
        assert_eq!(crate::ledger::artifact_kind(&switchover.value), "consensus-engine-switchover-receipt");
        let production_switchover =
            consensus_engine_switchover_receipt(&switchover_input).expect("production switchover denial");
        assert_eq!(production_switchover.decision, ENGINE_DECISION_DENY);
        assert!(production_switchover.diagnostics.join(";").contains("target engine admission denied"));

        assert_unsafe_switchover_and_stale_epochs_denied(&runtime.state.state_ref, next_epoch, &switchover.receipt_ref);
    }
