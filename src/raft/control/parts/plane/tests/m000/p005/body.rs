
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
            runtime: &runtime,
            read_consistency_mode: READ_CONSISTENCY_LOCAL_COMMITTED.to_string(),
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
