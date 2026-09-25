
    /// An unsafe switchover is denied, a writer on the old epoch is fenced, and the target engine cannot serve a
    /// linearizable read before activation.
    fn assert_unsafe_switchover_and_stale_epochs_denied(state_ref: &str, next_epoch: u64, activation_receipt_ref: &str) {
        let unsafe_switchover = consensus_engine_switchover_receipt(&ConsensusEngineSwitchoverInput {
            source_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            source_version: CONSENSUS_PROFILE_VERSION_RAFT.to_string(),
            target_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            target_version: CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL.to_string(),
            active_engine_epoch: INITIAL_CONSENSUS_ENGINE_EPOCH,
            target_engine_epoch: INITIAL_CONSENSUS_ENGINE_EPOCH,
            source_state_ref: state_ref.to_string(),
            target_bootstrap_state_ref: state_ref.to_string(),
            membership_refs: Vec::new(),
            placement_refs: Vec::new(),
            replay_conformance_refs: Vec::new(),
            currentness_evidence_refs: Vec::new(),
            operator_approval_refs: Vec::new(),
            rollback_posture: "unsafe".to_string(),
        })
        .expect("unsafe switchover");
        assert_eq!(unsafe_switchover.decision, ENGINE_DECISION_DENY);
        assert!(unsafe_switchover.diagnostics.join(";").contains("target engine admission denied"));

        let stale_writer = consensus_engine_epoch_gate(&ConsensusEngineEpochGateInput {
            operation: "write".to_string(),
            active_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            active_engine_epoch: next_epoch,
            presented_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            presented_engine_epoch: INITIAL_CONSENSUS_ENGINE_EPOCH,
            activation_receipt_ref: Some(activation_receipt_ref.to_string()),
        })
        .expect("stale writer gate");
        assert_eq!(stale_writer.decision, ENGINE_DECISION_DENY);
        assert!(stale_writer.diagnostics.join(";").contains("stale engine epoch"));

        let target_read_before_activation = consensus_engine_epoch_gate(&ConsensusEngineEpochGateInput {
            operation: "linearizable-read".to_string(),
            active_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            active_engine_epoch: INITIAL_CONSENSUS_ENGINE_EPOCH,
            presented_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            presented_engine_epoch: next_epoch,
            activation_receipt_ref: None,
        })
        .expect("target read gate");
        assert_eq!(target_read_before_activation.decision, ENGINE_DECISION_DENY);
        assert!(target_read_before_activation.diagnostics.join(";").contains("not activated"));
    }

    #[test]
    fn consensus_engine_conformance_fixtures_cover_positive_and_negative_paths() {
        // r[verify molten.testing.consensus_engine_conformance]
        // r[verify molten.testing.consensus_registry_negative_fixtures]
        // r[verify molten.testing.consensus_switchover_fixtures]
        let runtime = run_control_registry_fixture().expect("runtime");
        let normalized_ref = canonical_hash(
            &normalized_raft_commit_receipt_value(
                &runtime.commit_receipts[0],
                INITIAL_CONSENSUS_ENGINE_EPOCH,
            )
            .expect("normalized"),
        )
        .expect("normalized ref");
        let pass = consensus_engine_conformance_receipt(&ConsensusEngineConformanceInput {
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            profile_version: CONSENSUS_PROFILE_VERSION_RAFT.to_string(),
            fixture_id: "raft-control-registry-fixture".to_string(),
            passed_cases: required_conformance_cases().iter().map(|value| (*value).to_string()).collect(),
            expected_state_ref: runtime.state.state_ref.clone(),
            actual_state_ref: runtime.state.state_ref.clone(),
            normalized_receipt_refs: vec![normalized_ref],
        })
        .expect("conformance pass");
        assert_eq!(pass.decision, ENGINE_DECISION_PASS);
        assert_eq!(crate::ledger::artifact_kind(&pass.value), "consensus-engine-conformance-receipt");

        let negative = consensus_engine_conformance_receipt(&ConsensusEngineConformanceInput {
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            profile_version: CONSENSUS_PROFILE_VERSION_RAFT.to_string(),
            fixture_id: "raft-negative-fixture".to_string(),
            passed_cases: vec![CONFORMANCE_CASE_PROPOSAL.to_string()],
            expected_state_ref: runtime.state.state_ref.clone(),
            actual_state_ref: test_ref("wrong-state"),
            normalized_receipt_refs: Vec::new(),
        })
        .expect("conformance deny");
        assert_eq!(negative.decision, ENGINE_DECISION_DENY);
        assert!(negative.diagnostics.join(";").contains("missing consensus engine conformance case"));
        assert!(negative.diagnostics.join(";").contains("replay state mismatch"));
    }
