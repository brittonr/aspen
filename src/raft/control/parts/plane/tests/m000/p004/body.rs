
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

    fn registry_read<'a>(runtime: &'a ControlRegistryRuntime, mode: &str, name: &str) -> ControlRegistryReadInput<'a> {
        ControlRegistryReadInput {
            runtime,
            read_consistency_mode: mode.to_string(),
            namespace: "protocol".to_string(),
            name: name.to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        }
    }

    fn read_receipt_check(read: &RaftReadReceipt, name: &str) -> String {
        let fields = read.value.collect_simple_record("raft-read-receipt-v2", Some(15)).expect("read receipt fields");
        parse_checks(&fields[14])
            .expect("read checks")
            .into_iter()
            .find_map(|(check, status)| (check == name).then_some(status))
            .expect("read check present")
    }

    fn read_receipt_committed_index(read: &RaftReadReceipt) -> u64 {
        let fields = read.value.collect_simple_record("raft-read-receipt-v2", Some(15)).expect("read receipt fields");
        record_u64(&fields[5], "committed-index").expect("committed index")
    }

    fn committed_read(runtime: &ControlRegistryRuntime, name: &str) -> RaftReadReceipt {
        read_control_registry(&registry_read(runtime, READ_CONSISTENCY_LOCAL_COMMITTED, name)).expect("committed read")
    }

    /// Rebuilds a v2 record under another label with the given field replacements.
    fn relabel(
        value: &IoValue,
        labels: (&str, &'static str),
        arity: usize,
        replacements: &[(usize, IoValue)],
    ) -> IoValue {
        let fields = value.collect_simple_record(labels.0, Some(arity)).expect("record fields");
        let mut fields = (0..arity).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
        for (index, replacement) in replacements {
            fields[*index] = replacement.clone();
        }
        record(labels.1, fields)
    }

    #[test]
    fn local_committed_read_binds_the_instance_committed_state() {
        // r[verify molten.consensus.read_consistency_modes]
        let mut runtime = run_control_registry_fixture().expect("runtime");
        let first = committed_read(&runtime, "proto:request-response");
        assert_eq!(first.decision, RAFT_DECISION_PASS);
        assert!(first.diagnostics.is_empty());
        assert_eq!(read_receipt_check(&first, "committed-state-bound"), RAFT_DECISION_PASS);
        assert_eq!(read_receipt_check(&first, "local-stale-non-authoritative"), "diagnostic");
        assert_eq!(read_receipt_committed_index(&first), runtime.committed_index);
        let first_fields = read_receipt_fields(&first.value).expect("first fields");
        assert_eq!(first_fields.state_ref, runtime.state.state_ref);
        assert_eq!(first_fields.group_ref, runtime.manifest.manifest_ref);
        assert!(first_fields.committed_read_predicate_ref.is_some());

        let command = command_for_receipt_index("committed-read", "committed-read-target");
        let envelope = envelope_for(&runtime, "client:committed-read", RAFT_TEST_INITIAL_SEQUENCE, command);
        let proposal = propose_control_registry_command(&mut runtime, &envelope).expect("proposal");
        assert_eq!(proposal.decision, RAFT_DECISION_PASS);
        let second = committed_read(&runtime, "proto:request-response");
        let second_fields = read_receipt_fields(&second.value).expect("second fields");
        assert_eq!(second.decision, RAFT_DECISION_PASS);
        assert_eq!(second_fields.state_ref, runtime.state.state_ref);
        assert_ne!(second_fields.state_ref, first_fields.state_ref);
        assert_eq!(read_receipt_committed_index(&second), read_receipt_committed_index(&first) + 1);
        assert_ne!(second_fields.committed_read_predicate_ref, first_fields.committed_read_predicate_ref);

        let missing = committed_read(&runtime, "proto:absent");
        assert_eq!(missing.decision, RAFT_DECISION_DENY);
        assert_eq!(read_receipt_check(&missing, "committed-state-bound"), RAFT_DECISION_DENY);
        assert!(read_receipt_fields(&missing.value).expect("missing fields").committed_read_predicate_ref.is_none());
    }

    #[test]
    fn version_one_registry_receipts_are_rejected() {
        const PREDICATE_V2: &str = "raft-predicate-receipt-v2";
        let runtime = run_control_registry_fixture().expect("runtime");
        let current_predicate = &runtime.predicate_receipts[0].value;
        assert!(parse_predicate_receipt(current_predicate).is_ok());
        let trellis_checks = checks_value(&[("trellis-predicate", "pass"), ("prior-log-binding", "pass")]);
        let v1_predicate = relabel(current_predicate, (PREDICATE_V2, "raft-predicate-receipt-v1"), 9, &[
            (0, string("molten.raft.predicate-receipt.v1")),
            (8, trellis_checks.clone()),
        ]);
        assert!(parse_predicate_receipt(&v1_predicate).is_err());
        let v2_without_evaluator = relabel(current_predicate, (PREDICATE_V2, PREDICATE_V2), 9, &[(8, trellis_checks)]);
        assert!(parse_predicate_receipt(&v2_without_evaluator).is_err());

        let read = committed_read(&runtime, "proto:request-response");
        assert!(read_receipt_fields(&read.value).is_ok());
        let v1_read = relabel(&read.value, ("raft-read-receipt-v2", "raft-read-receipt-v1"), 15, &[
            (0, string("molten.raft.read-receipt.v1")),
            (10, record("read-index-predicate", vec![record("none", Vec::new())])),
        ]);
        assert!(read_receipt_fields(&v1_read).is_err());
        let v1_read = RaftReadReceipt { value: v1_read, ..read };
        assert!(normalized_raft_read_receipt_value(&v1_read, INITIAL_CONSENSUS_ENGINE_EPOCH).is_err());

        let commit = &runtime.commit_receipts[0].value;
        assert!(parse_commit_receipt(commit).is_ok());
        let v1_commit = relabel(commit, ("raft-commit-receipt-v2", "raft-commit-receipt-v1"), 11, &[(
            0,
            string("molten.raft.commit-receipt.v1"),
        )]);
        assert!(parse_commit_receipt(&v1_commit).is_err());
    }
