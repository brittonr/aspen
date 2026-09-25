
    #[test]
    fn pure_transition_core_denies_invalid_command_without_mutating_runtime() {
        const TEST_SEQUENCE: u64 = 1;
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let prior_state_ref = runtime.state.state_ref.clone();
        let prior_committed_index = runtime.committed_index;
        let actor_message = parse_text("<actor-message-v1 \"hello\">").expect("actor message");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:pure-bad".to_string(),
            sequence: TEST_SEQUENCE,
            command: actor_message,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("bad envelope");
        let transition = propose_control_registry_transition_core(&runtime, &envelope).expect("pure deny transition");
        assert_eq!(transition.proposal.decision, "deny");
        assert!(transition.state_after.is_none());
        assert_eq!(runtime.state.state_ref, prior_state_ref);
        assert_eq!(runtime.committed_index, prior_committed_index);
        assert!(runtime.log_entries.is_empty());
    }

    #[test]
    fn consensus_capability_traits_and_unsupported_denials_are_explicit() {
        const TEST_SEQUENCE: u64 = 1;
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let command = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-policy-version".to_string(),
            namespace: "policy".to_string(),
            name: "runtime".to_string(),
            target_ref: Some(test_ref("policy")),
        })
        .expect("command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:trait".to_string(),
            sequence: TEST_SEQUENCE,
            command,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("envelope");
        let engine = RaftControlPlaneEngine;
        let transition = engine.propose_transition(&runtime, &envelope).expect("trait transition");
        assert_eq!(transition.proposal.decision, "pass");
        assert!(transition.state_after.is_some());
        let error = unsupported_consensus_capability("read-only-test-engine", ENGINE_CAPABILITY_RECOVERY)
            .expect_err("unsupported recovery denies");
        assert!(error.to_string().contains("does not support capability"));
    }

    #[test]
    fn stale_read_bad_snapshot_log_gap_and_redb_store_are_detected() {
        let runtime = run_control_registry_fixture().expect("runtime");
        let stale = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(1),
            read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("stale read");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));

        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let mut bad_snapshot = snapshot.value.clone();
        if let Some(fields) = bad_snapshot.collect_simple_record("raft-snapshot-v1", Some(10)) {
            let mut fields = (0..10).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
            fields[5] = record("content-ref", vec![string(test_ref("wrong-content"))]);
            bad_snapshot = record("raft-snapshot-v1", fields);
        }
        assert!(parse_raft_snapshot(&bad_snapshot).is_err());

        let gap_entry = runtime.log_entries[0].clone();
        let mut gap_value = gap_entry.value.clone();
        if let Some(fields) = gap_value.collect_simple_record("raft-log-entry-v1", Some(9)) {
            let mut fields = (0..9).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
            fields[3] = record("index", vec![u64_value(snapshot.index + 2)]);
            gap_value = record("raft-log-entry-v1", fields);
        }
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value.clone(),
            log_entries: vec![gap_value],
        })
        .expect("gap recovery");
        assert_eq!(recovery.decision, "deny");
        assert!(recovery.diagnostics.iter().any(|diagnostic| diagnostic.contains("log gap")));

        let root = temp_dir("redb-store");
        persist_control_registry_runtime(&root, &runtime, &snapshot).expect("persist runtime");
        let status = control_registry_store_status(&root).expect("store status");
        let expected_log_count = u64::try_from(runtime.log_entries.len()).expect("log count");
        let expected_session_count = u64::try_from(runtime.state.client_sessions.len()).expect("session count");
        assert_eq!(status.log_count, expected_log_count);
        assert_eq!(status.snapshot_count, 1);
        assert_eq!(status.session_count, expected_session_count);
        assert!(status.receipt_count >= expected_log_count);
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_raft_artifacts() {
        let runtime = run_control_registry_fixture().expect("runtime");
        assert_eq!(crate::ledger::artifact_kind(&runtime.manifest.value), "raft-group-manifest");
        assert_eq!(crate::ledger::artifact_kind(&runtime.log_entries[0].value), "raft-log-entry");
        assert_eq!(crate::ledger::artifact_kind(&runtime.registry_receipts[0].value), "control-registry-receipt");
        let ledger_root = temp_dir("ledger");
        crate::ledger::import_artifact(&ledger_root, &runtime.registry_receipts[0].value)
            .expect("import registry receipt");
        let registry = temp_dir("catalog");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("control-registry-receipt".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let request = crate::catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "control-registry-receipt",
        )])])
        .expect("mcp request");
        let mcp = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render mcp").contains("control-registry-receipt"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_raft_control_registry_generated_logs_match_after_each_commit(tc: hegel::TestCase) {
        // r[verify molten.consensus_state_machine_proof.registry_log_determinism]
        let command_count = usize::try_from(
            tc.draw(
                hegel::generators::integers::<u64>()
                    .min_value(GENERATED_RAFT_MIN_COMMANDS)
                    .max_value(GENERATED_RAFT_MAX_COMMANDS),
            ),
        )
        .expect("command count");
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut left = new_control_registry_runtime(&manifest).expect("left runtime");
        let mut right = new_control_registry_runtime(&manifest).expect("right runtime");
        let mut sequence = RAFT_TEST_INITIAL_SEQUENCE;
        for index in 0..command_count {
            let envelope = envelope_for(
                &left,
                "client:generated-raft-log",
                next_raft_sequence(&mut sequence),
                command_for_receipt_index(format!("scope-{index}"), &format!("target-{index}")),
            );
            let left_result = propose_control_registry_command(&mut left, &envelope).expect("left proposal");
            let right_result = propose_control_registry_command(&mut right, &envelope).expect("right proposal");
            assert_matching_pass(&left, &right, &left_result, &right_result);
            assert_eq!(left.committed_index, right.committed_index);
            assert_eq!(left.log_entries.len(), right.log_entries.len());
            assert_eq!(left.commit_receipts.len(), right.commit_receipts.len());
            assert_eq!(left.registry_receipts.len(), right.registry_receipts.len());
        }
    }

    #[test]
    fn raft_control_registry_duplicate_and_negative_inputs_do_not_advance() {
        // r[verify molten.consensus_state_machine_proof.duplicate_client_sequence]
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let mut sequence = RAFT_TEST_INITIAL_SEQUENCE;
        let first_sequence = next_raft_sequence(&mut sequence);
        let first_envelope = duplicate_proof_envelope(&runtime, first_sequence, "duplicate-scope", "duplicate-target-v1");
        let first = propose_control_registry_command(&mut runtime, &first_envelope).expect("first proposal");
        assert_eq!(first.decision, RAFT_DECISION_PASS);
        let after_first = RuntimeMark::of(&runtime);

        let replay = propose_control_registry_command(&mut runtime, &first_envelope).expect("duplicate replay");
        assert!(replay.duplicate);
        assert_eq!(replay.decision, RAFT_DECISION_PASS);
        assert_eq!(replay.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        after_first.assert_all_unchanged(&runtime);

        let conflicting_envelope = duplicate_proof_envelope(&runtime, first_sequence, "duplicate-scope", "duplicate-target-v2");
        let conflict = propose_control_registry_command(&mut runtime, &conflicting_envelope).expect("conflict denial");
        assert!(conflict.duplicate);
        assert_eq!(conflict.decision, RAFT_DECISION_DENY);
        assert_eq!(conflict.registry_receipt.decision, RAFT_DECISION_DENY);
        assert!(conflict.log_entry.is_none());
        after_first.assert_state_and_log_unchanged(&runtime);
        assert!(conflict
            .registry_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("conflicting duplicate client sequence")));

        let second_envelope = duplicate_proof_envelope(&runtime, next_raft_sequence(&mut sequence), "later-scope", "later-target");
        let second = propose_control_registry_command(&mut runtime, &second_envelope).expect("second sequence pass");
        assert_eq!(second.decision, RAFT_DECISION_PASS);
        let after_second = RuntimeMark::of(&runtime);

        let old_replay = propose_control_registry_command(&mut runtime, &first_envelope).expect("old sequence replay");
        assert!(old_replay.duplicate);
        assert_eq!(old_replay.decision, RAFT_DECISION_PASS);
        assert_eq!(old_replay.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        assert!(old_replay.log_entry.is_none());
        after_second.assert_all_unchanged(&runtime);

        let old_conflict_envelope = duplicate_proof_envelope(&runtime, first_sequence, "duplicate-scope", "duplicate-target-v3");
        assert_duplicate_denied_without_advance(&mut runtime, &old_conflict_envelope, &after_second);

        let stale_unseen_envelope = duplicate_proof_envelope(&runtime, STALE_RAFT_SEQUENCE_BEFORE_INITIAL, "stale-scope", "stale-target");
        assert_duplicate_denied_without_advance(&mut runtime, &stale_unseen_envelope, &after_second);

        let malformed_envelope = envelope_for(
            &runtime,
            "client:malformed-command",
            next_raft_sequence(&mut sequence),
            record("mystery-raft-payload-v1", vec![string("malformed")]),
        );
        let malformed = propose_control_registry_command(&mut runtime, &malformed_envelope).expect("malformed denial");
        assert_eq!(malformed.decision, RAFT_DECISION_DENY);
        after_second.assert_all_unchanged(&runtime);
        assert!(malformed
            .registry_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unknown Raft command schema")));

        assert_stale_read_and_unsupported_state_machine_denied(&runtime);
    }

    /// The runtime state ref and log, commit, and registry receipt counts at one point of a test.
    struct RuntimeMark {
        state_ref: String,
        log_entries: usize,
        commit_receipts: usize,
        registry_receipts: usize,
    }

    impl RuntimeMark {
        fn of(runtime: &ControlRegistryRuntime) -> Self {
            Self {
                state_ref: runtime.state.state_ref.clone(),
                log_entries: runtime.log_entries.len(),
                commit_receipts: runtime.commit_receipts.len(),
                registry_receipts: runtime.registry_receipts.len(),
            }
        }

        fn assert_state_and_log_unchanged(&self, runtime: &ControlRegistryRuntime) {
            assert_eq!(runtime.state.state_ref, self.state_ref);
            assert_eq!(runtime.log_entries.len(), self.log_entries);
        }

        fn assert_all_unchanged(&self, runtime: &ControlRegistryRuntime) {
            self.assert_state_and_log_unchanged(runtime);
            assert_eq!(runtime.commit_receipts.len(), self.commit_receipts);
            assert_eq!(runtime.registry_receipts.len(), self.registry_receipts);
        }
    }

    fn duplicate_proof_envelope(runtime: &ControlRegistryRuntime, sequence: u64, scope: &str, target: &str) -> IoValue {
        envelope_for(runtime, "client:duplicate-proof", sequence, command_for_receipt_index(scope, target))
    }

    /// A duplicate-sequence proposal is denied without a log entry and without advancing state or the log.
    fn assert_duplicate_denied_without_advance(runtime: &mut ControlRegistryRuntime, envelope: &IoValue, mark: &RuntimeMark) {
        let proposal = propose_control_registry_command(runtime, envelope).expect("duplicate denial");
        assert!(proposal.duplicate);
        assert_eq!(proposal.decision, RAFT_DECISION_DENY);
        assert!(proposal.log_entry.is_none());
        mark.assert_state_and_log_unchanged(runtime);
    }
