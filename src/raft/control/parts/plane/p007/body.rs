#[cfg(test)]
mod tests {
    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("raft-control-test-ref", vec![string(label)])).expect("test ref")
    }

    fn auth() -> Vec<String> {
        vec![test_ref("authority")]
    }

    fn resources() -> Vec<String> {
        vec![test_ref("resource")]
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-raft-control-{label}-{}-{id}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    const RAFT_DECISION_PASS: &str = "pass";
    const RAFT_DECISION_DENY: &str = "deny";
    const RAFT_TEST_INITIAL_SEQUENCE: u64 = 1;
    const RAFT_TEST_SEQUENCE_STEP: u64 = 1;
    const STALE_RAFT_SEQUENCE_BEFORE_INITIAL: u64 = 0;
    const GENERATED_RAFT_MIN_COMMANDS: u64 = 1;
    const GENERATED_RAFT_MAX_COMMANDS: u64 = 4;
    const SNAPSHOT_RECORD_FIELD_COUNT: usize = 10;
    const SNAPSHOT_CONTENT_REF_FIELD_INDEX: usize = 5;
    const SNAPSHOT_STATE_FIELD_INDEX: usize = 6;
    const EXPECTED_READ_CONSISTENCY_MODE_COUNT: usize = 2;
    const MINORITY_CONNECTED_REPLICAS: usize = 1;
    const EXPERIMENTAL_CONNECTED_REPLICAS: usize = 3;

    fn next_raft_sequence(sequence: &mut u64) -> u64 {
        let current = *sequence;
        *sequence = sequence.saturating_add(RAFT_TEST_SEQUENCE_STEP);
        current
    }

    fn command_for_receipt_index(name: impl Into<String>, target_label: &str) -> IoValue {
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-receipt-index".to_string(),
            namespace: "receipt-index".to_string(),
            name: name.into(),
            target_ref: Some(test_ref(target_label)),
        })
        .expect("receipt-index command")
    }

    fn envelope_for(
        runtime: &ControlRegistryRuntime,
        client_session: &str,
        sequence: u64,
        command: IoValue,
    ) -> IoValue {
        raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: client_session.to_string(),
            sequence,
            command,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("raft command envelope")
    }

    fn assert_matching_pass(
        left: &ControlRegistryRuntime,
        right: &ControlRegistryRuntime,
        left_result: &ControlRegistryProposal,
        right_result: &ControlRegistryProposal,
    ) {
        // r[verify molten.consensus_state_machine_proof.registry_log_determinism]
        assert_eq!(left_result.decision, RAFT_DECISION_PASS);
        assert_eq!(right_result.decision, RAFT_DECISION_PASS);
        let left_entry = left_result.log_entry.as_ref().expect("left log entry");
        let right_entry = right_result.log_entry.as_ref().expect("right log entry");
        assert_eq!(left.state.state_ref, right.state.state_ref);
        assert_eq!(left_entry.entry_ref, right_entry.entry_ref);
        assert_eq!(left_result.commit_receipt.receipt_ref, right_result.commit_receipt.receipt_ref);
        assert_eq!(left_result.registry_receipt.receipt_ref, right_result.registry_receipt.receipt_ref);
        assert_eq!(left_result.commit_receipt.log_entry_ref.as_deref(), Some(left_entry.entry_ref.as_str()));
        assert_eq!(right_result.commit_receipt.log_entry_ref.as_deref(), Some(right_entry.entry_ref.as_str()));
    }

    fn replace_snapshot_field(snapshot: &IoValue, field_index: usize, replacement: IoValue) -> IoValue {
        if let Some(fields) = snapshot.collect_simple_record("raft-snapshot-v1", Some(SNAPSHOT_RECORD_FIELD_COUNT)) {
            let mut fields = (0..SNAPSHOT_RECORD_FIELD_COUNT)
                .map(|index| value_to_iovalue(&fields[index]))
                .collect::<Vec<_>>();
            fields[field_index] = replacement;
            record("raft-snapshot-v1", fields)
        } else {
            snapshot.clone()
        }
    }

    #[test]
    fn local_cluster_applies_reads_snapshots_and_recovers() {
        let runtime = run_control_registry_fixture().expect("run fixture");
        assert_eq!(runtime.committed_index, 3);
        assert_eq!(runtime.state.entries.len(), 3);
        let read = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index,
            read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("read registry");
        assert_eq!(read.decision, "pass");
        assert!(read.target_ref.is_some());
        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value,
            log_entries: Vec::new(),
        })
        .expect("recover");
        assert_eq!(recovery.decision, "pass");
        assert_eq!(recovery.restored_state_ref.as_deref(), Some(runtime.state.state_ref.as_str()));
    }

    #[test]
    fn registry_updates_remove_and_duplicate_sequences_are_idempotent() {
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let command = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-artifact-name".to_string(),
            namespace: "artifact-name".to_string(),
            name: "calculator".to_string(),
            target_ref: Some(test_ref("artifact-v1")),
        })
        .expect("command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:one".to_string(),
            sequence: 7,
            command,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("envelope");
        let first = propose_control_registry_command(&mut runtime, &envelope).expect("first proposal");
        assert_eq!(first.decision, "pass");
        let state_after_first = runtime.state.state_ref.clone();
        let duplicate = propose_control_registry_command(&mut runtime, &envelope).expect("duplicate proposal");
        assert_eq!(duplicate.decision, "pass");
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        assert_eq!(runtime.state.state_ref, state_after_first);
        assert_eq!(runtime.log_entries.len(), 1);

        let remove = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "remove".to_string(),
            namespace: "artifact-name".to_string(),
            name: "calculator".to_string(),
            target_ref: None,
        })
        .expect("remove command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:one".to_string(),
            sequence: 8,
            command: remove,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("remove envelope");
        propose_control_registry_command(&mut runtime, &envelope).expect("remove proposal");
        assert!(find_entry(&runtime.state, "artifact-name", "calculator").is_none());
    }

    #[test]
    fn actor_messages_and_missing_authority_do_not_append() {
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let actor_message = parse_text("<actor-message-v1 \"hello\">").expect("actor message");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:bad".to_string(),
            sequence: 1,
            command: actor_message,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("bad envelope");
        let denied = propose_control_registry_command(&mut runtime, &envelope).expect("deny actor message");
        assert_eq!(denied.decision, "deny");
        assert!(denied.log_entry.is_none());
        assert!(runtime.log_entries.is_empty());
        assert!(denied.registry_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-control")));

        let command = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-policy-version".to_string(),
            namespace: "policy".to_string(),
            name: "runtime".to_string(),
            target_ref: Some(test_ref("policy")),
        })
        .expect("command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:missing-auth".to_string(),
            sequence: 2,
            command,
            authority_refs: Vec::new(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("missing authority envelope");
        let denied = propose_control_registry_command(&mut runtime, &envelope).expect("deny missing authority");
        assert_eq!(denied.decision, "deny");
        assert!(denied.log_entry.is_none());
        assert!(runtime.log_entries.is_empty());
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
        let first_envelope = envelope_for(
            &runtime,
            "client:duplicate-proof",
            first_sequence,
            command_for_receipt_index("duplicate-scope", "duplicate-target-v1"),
        );
        let first = propose_control_registry_command(&mut runtime, &first_envelope).expect("first proposal");
        assert_eq!(first.decision, RAFT_DECISION_PASS);
        let state_after_first = runtime.state.state_ref.clone();
        let log_count_after_first = runtime.log_entries.len();
        let commit_count_after_first = runtime.commit_receipts.len();
        let registry_count_after_first = runtime.registry_receipts.len();

        let replay = propose_control_registry_command(&mut runtime, &first_envelope).expect("duplicate replay");
        assert!(replay.duplicate);
        assert_eq!(replay.decision, RAFT_DECISION_PASS);
        assert_eq!(replay.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        assert_eq!(runtime.state.state_ref, state_after_first);
        assert_eq!(runtime.log_entries.len(), log_count_after_first);
        assert_eq!(runtime.commit_receipts.len(), commit_count_after_first);
        assert_eq!(runtime.registry_receipts.len(), registry_count_after_first);

        let conflicting_envelope = envelope_for(
            &runtime,
            "client:duplicate-proof",
            first_sequence,
            command_for_receipt_index("duplicate-scope", "duplicate-target-v2"),
        );
        let conflict = propose_control_registry_command(&mut runtime, &conflicting_envelope).expect("conflict denial");
        assert!(conflict.duplicate);
        assert_eq!(conflict.decision, RAFT_DECISION_DENY);
        assert_eq!(conflict.registry_receipt.decision, RAFT_DECISION_DENY);
        assert!(conflict.log_entry.is_none());
        assert_eq!(runtime.state.state_ref, state_after_first);
        assert_eq!(runtime.log_entries.len(), log_count_after_first);
        assert!(conflict
            .registry_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("conflicting duplicate client sequence")));

        let second_envelope = envelope_for(
            &runtime,
            "client:duplicate-proof",
            next_raft_sequence(&mut sequence),
            command_for_receipt_index("later-scope", "later-target"),
        );
        let second = propose_control_registry_command(&mut runtime, &second_envelope).expect("second sequence pass");
        assert_eq!(second.decision, RAFT_DECISION_PASS);
        let state_after_second = runtime.state.state_ref.clone();
        let log_count_after_second = runtime.log_entries.len();
        let commit_count_after_second = runtime.commit_receipts.len();
        let registry_count_after_second = runtime.registry_receipts.len();

        let old_replay = propose_control_registry_command(&mut runtime, &first_envelope).expect("old sequence replay");
        assert!(old_replay.duplicate);
        assert_eq!(old_replay.decision, RAFT_DECISION_PASS);
        assert_eq!(old_replay.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        assert!(old_replay.log_entry.is_none());
        assert_eq!(runtime.state.state_ref, state_after_second);
        assert_eq!(runtime.log_entries.len(), log_count_after_second);
        assert_eq!(runtime.commit_receipts.len(), commit_count_after_second);
        assert_eq!(runtime.registry_receipts.len(), registry_count_after_second);

        let old_conflict_envelope = envelope_for(
            &runtime,
            "client:duplicate-proof",
            first_sequence,
            command_for_receipt_index("duplicate-scope", "duplicate-target-v3"),
        );
        let old_conflict =
            propose_control_registry_command(&mut runtime, &old_conflict_envelope).expect("old conflict denial");
        assert!(old_conflict.duplicate);
        assert_eq!(old_conflict.decision, RAFT_DECISION_DENY);
        assert!(old_conflict.log_entry.is_none());
        assert_eq!(runtime.state.state_ref, state_after_second);
        assert_eq!(runtime.log_entries.len(), log_count_after_second);

        let stale_unseen_envelope = envelope_for(
            &runtime,
            "client:duplicate-proof",
            STALE_RAFT_SEQUENCE_BEFORE_INITIAL,
            command_for_receipt_index("stale-scope", "stale-target"),
        );
        let stale_unseen =
            propose_control_registry_command(&mut runtime, &stale_unseen_envelope).expect("stale unseen denial");
        assert!(stale_unseen.duplicate);
        assert_eq!(stale_unseen.decision, RAFT_DECISION_DENY);
        assert!(stale_unseen.log_entry.is_none());
        assert_eq!(runtime.state.state_ref, state_after_second);
        assert_eq!(runtime.log_entries.len(), log_count_after_second);

        let malformed_envelope = envelope_for(
            &runtime,
            "client:malformed-command",
            next_raft_sequence(&mut sequence),
            record("mystery-raft-payload-v1", vec![string("malformed")]),
        );
        let malformed = propose_control_registry_command(&mut runtime, &malformed_envelope).expect("malformed denial");
        assert_eq!(malformed.decision, RAFT_DECISION_DENY);
        assert_eq!(runtime.state.state_ref, state_after_second);
        assert_eq!(runtime.log_entries.len(), log_count_after_second);
        assert_eq!(runtime.commit_receipts.len(), commit_count_after_second);
        assert_eq!(runtime.registry_receipts.len(), registry_count_after_second);
        assert!(malformed
            .registry_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unknown Raft command schema")));

        let stale_read = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(RAFT_TEST_SEQUENCE_STEP),
            read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
            namespace: "receipt-index".to_string(),
            name: "duplicate-scope".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("stale read");
        assert_eq!(stale_read.decision, RAFT_DECISION_DENY);
        assert!(stale_read.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale read-index")));

        let wrong_state_machine_manifest = raft_group_manifest_value(&RaftGroupManifestInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            members: vec![test_ref("member-a")],
            state_machine: "unsupported-state-machine".to_string(),
            command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
            read_mode: READ_MODE_READ_INDEX.to_string(),
            snapshot_policy_ref: test_ref("snapshot-policy"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("wrong state machine manifest");
        let wrong_state_machine = new_control_registry_runtime(&wrong_state_machine_manifest).expect_err("state machine denial");
        assert!(wrong_state_machine.to_string().contains("unsupported raft state machine"));
    }

    #[test]
    fn raft_control_registry_snapshot_restore_equivalence_and_negative_evidence() {
        // r[verify molten.consensus_state_machine_proof.snapshot_restore_equivalence]
        let runtime = run_control_registry_fixture().expect("runtime");
        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value.clone(),
            log_entries: Vec::new(),
        })
        .expect("recover");
        assert_eq!(recovery.decision, RAFT_DECISION_PASS);
        assert_eq!(snapshot.state.state_ref, runtime.state.state_ref);
        assert_eq!(recovery.restored_state_ref.as_deref(), Some(snapshot.state.state_ref.as_str()));
        validate_content_ref(&snapshot.snapshot_ref).expect("snapshot ref");
        validate_content_ref(&recovery.receipt_ref).expect("recovery receipt ref");

        let tampered_snapshot = replace_snapshot_field(
            &snapshot.value,
            SNAPSHOT_CONTENT_REF_FIELD_INDEX,
            record("content-ref", vec![string(test_ref("tampered-content"))]),
        );
        let tampered = parse_raft_snapshot(&tampered_snapshot).expect_err("tampered snapshot denial");
        assert!(tampered.to_string().contains("raft snapshot state/content ref mismatch"));

        let missing_state_snapshot = replace_snapshot_field(
            &snapshot.value,
            SNAPSHOT_STATE_FIELD_INDEX,
            record("state", vec![record("none", Vec::new())]),
        );
        assert!(parse_raft_snapshot(&missing_state_snapshot).is_err());

        let mismatched_group = recover_control_registry(&RaftRecoveryInput {
            group_ref: test_ref("wrong-recovery-group"),
            snapshot: snapshot.value,
            log_entries: Vec::new(),
        })
        .expect("mismatched group recovery");
        assert_eq!(mismatched_group.decision, RAFT_DECISION_DENY);
        assert!(mismatched_group
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("snapshot group does not match recovery group")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_bounded_registry_logs_are_deterministic_and_control_only(tc: hegel::TestCase) {
        let command_count = usize::try_from(tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(4)))
            .expect("command count");
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut left = new_control_registry_runtime(&manifest).expect("left runtime");
        let mut right = new_control_registry_runtime(&manifest).expect("right runtime");
        for index in 0..command_count {
            let target = test_ref(&format!("target-{index}"));
            let command = control_registry_command_value(&ControlRegistryCommandInput {
                operation: "set-receipt-index".to_string(),
                namespace: "receipt-index".to_string(),
                name: format!("scope-{index}"),
                target_ref: Some(target),
            })
            .expect("generated command");
            let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
                group_ref: left.manifest.manifest_ref.clone(),
                client_session: "client:property".to_string(),
                sequence: u64::try_from(index + 1).expect("sequence"),
                command,
                authority_refs: auth(),
                policy_refs: left.manifest.policy_refs.clone(),
                resource_refs: left.manifest.resource_refs.clone(),
                evidence_refs: vec![test_ref("evidence")],
            })
            .expect("generated envelope");
            let left_result = propose_control_registry_command(&mut left, &envelope).expect("left proposal");
            let right_result = propose_control_registry_command(&mut right, &envelope).expect("right proposal");
            assert_eq!(left_result.decision, "pass");
            assert_eq!(left_result.registry_receipt.receipt_ref, right_result.registry_receipt.receipt_ref);
        }
        assert_eq!(left.state.state_ref, right.state.state_ref);
        assert_eq!(left.log_entries.len(), command_count);
        for entry in &left.log_entries {
            let envelope = parse_raft_command_envelope(&entry.command).expect("entry command envelope");
            assert!(parse_control_registry_command(&envelope.command).is_ok());
        }
    }

    #[test]
    fn consensus_profiles_reads_and_non_claims_are_fail_closed() {
        // r[verify molten.consensus.algorithm_profile_manifest]
        // r[verify molten.consensus.leaderless_profile_boundary]
        // r[verify molten.consensus.read_consistency_modes]
        // r[verify molten.consensus.non_claim_boundaries]
        let manifest_value = control_registry_fixture_manifest_value().expect("manifest");
        let manifest = parse_raft_group_manifest(&manifest_value).expect("parse manifest");
        assert_eq!(manifest.algorithm_profile, CONSENSUS_PROFILE_RAFT);
        assert_eq!(manifest.production_status, PRODUCTION_STATUS_ADMITTED);
        assert_eq!(manifest.read_consistency_support.len(), EXPECTED_READ_CONSISTENCY_MODE_COUNT);
        assert!(manifest.placement_ref.is_some());

        let runtime = run_control_registry_fixture().expect("runtime");
        let local_stale = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(RAFT_TEST_SEQUENCE_STEP),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("local stale read");
        assert_eq!(local_stale.decision, RAFT_DECISION_PASS);
        assert_eq!(local_stale.read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert!(to_text(&local_stale.value).expect("read text").contains("local-stale-non-authoritative"));

        let leaderless_profile = leaderless_experimental_algorithm_profile_input(
            vec![test_ref("membership-policy")],
            Some(test_ref("placement")),
            vec![test_ref("proof"), test_ref("simulation")],
        );
        let leaderless_manifest = raft_group_manifest_value_with_profile(&RaftGroupManifestInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            members: vec![test_ref("member-a"), test_ref("member-b"), test_ref("member-c")],
            state_machine: CONTROL_REGISTRY_STATE_MACHINE.to_string(),
            command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
            read_mode: READ_MODE_READ_INDEX.to_string(),
            snapshot_policy_ref: test_ref("snapshot-policy"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        }, &leaderless_profile)
        .expect("leaderless manifest");
        let leaderless_runtime = new_control_registry_runtime(&leaderless_manifest).expect_err("leaderless denied");
        assert!(leaderless_runtime.to_string().contains("not admitted for production runtime"));

        let claim = consensus_claim_boundary_receipt(&ConsensusClaimBoundaryInput {
            group_ref: manifest.manifest_ref.clone(),
            claim: "byzantine-tolerance".to_string(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("claim boundary");
        assert_eq!(claim.decision, RAFT_DECISION_DENY);
        assert!(claim.diagnostics.join(";").contains("Byzantine"));
        assert_eq!(crate::ledger::artifact_kind(&claim.value), "consensus-non-claim-receipt");
    }

    #[test]
    fn consensus_placement_and_simulation_cover_positive_and_negative_paths() {
        // r[verify molten.consensus.replica_placement_evidence]
        // r[verify molten.testing.consensus_fault_matrix]
        // r[verify molten.testing.leaderless_experimental_fixtures]
        // r[verify molten.testing.consensus_placement_fixtures]
        let members = vec![test_ref("member-a"), test_ref("member-b"), test_ref("member-c")];
        let placement = consensus_placement_report(&ConsensusPlacementInput {
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
        })
        .expect("placement pass");
        assert_eq!(placement.decision, RAFT_DECISION_PASS);
        assert_eq!(crate::ledger::artifact_kind(&placement.value), "consensus-placement-report");

        let unsafe_placement = consensus_placement_report(&ConsensusPlacementInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            candidate_members: members.clone(),
            admitted_members: members.clone(),
            fault_domain_refs: vec![test_ref("domain-shared")],
            fault_domain_policy_ref: test_ref("fault-policy"),
            membership_refs: Vec::new(),
            placement_policy_refs: vec![test_ref("placement-policy")],
            majority_reachable: false,
            latency_diagnostics: Vec::new(),
            denied_candidates: Vec::new(),
            refresh_refs: Vec::new(),
        })
        .expect("placement deny");
        assert_eq!(unsafe_placement.decision, RAFT_DECISION_DENY);
        assert!(unsafe_placement.diagnostics.join(";").contains("majority"));

        let majority = run_consensus_simulation(&ConsensusSimulationInput {
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
        })
        .expect("majority simulation");
        assert_eq!(majority.decision, RAFT_DECISION_PASS);
        assert!(majority.final_state_ref.is_some());
        assert_eq!(crate::ledger::artifact_kind(&majority.value), "consensus-simulation-receipt");

        let minority = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_MINORITY_DENIAL.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members.clone(),
            fault_plan_ref: test_ref("minority-fault"),
            operation_ids: vec![test_ref("operation")],
            connected_replicas: MINORITY_CONNECTED_REPLICAS,
            proposer_ref: Some(test_ref("member-a")),
            required_evidence_refs: vec![test_ref("raft-evidence")],
            placement_ref: Some(placement.report_ref.clone()),
            local_state_fresh: false,
            requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        })
        .expect("minority simulation");
        assert_eq!(minority.decision, RAFT_DECISION_PASS);
        assert!(minority.final_state_ref.is_some());

        let stale_linearizable = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_STALE_READ_CLASSIFICATION.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members.clone(),
            fault_plan_ref: test_ref("stale-read"),
            operation_ids: Vec::new(),
            connected_replicas: members.len(),
            proposer_ref: None,
            required_evidence_refs: vec![test_ref("raft-evidence")],
            placement_ref: Some(placement.report_ref.clone()),
            local_state_fresh: false,
            requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        })
        .expect("stale linearizable");
        assert_eq!(stale_linearizable.decision, RAFT_DECISION_DENY);
        assert!(stale_linearizable.diagnostics.join(";").contains("freshness"));

        let local_stale = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_STALE_READ_CLASSIFICATION.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members.clone(),
            fault_plan_ref: test_ref("stale-read"),
            operation_ids: Vec::new(),
            connected_replicas: members.len(),
            proposer_ref: None,
            required_evidence_refs: vec![test_ref("raft-evidence")],
            placement_ref: Some(placement.report_ref.clone()),
            local_state_fresh: false,
            requested_read_consistency: READ_CONSISTENCY_LOCAL_STALE.to_string(),
        })
        .expect("local stale simulation");
        assert_eq!(local_stale.decision, RAFT_DECISION_PASS);

        let leaderless = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_LEADERLESS_NON_LEADER_PROGRESS.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members.clone(),
            fault_plan_ref: test_ref("leaderless-fault"),
            operation_ids: vec![test_ref("leaderless-operation")],
            connected_replicas: members.len(),
            proposer_ref: Some(test_ref("member-b")),
            required_evidence_refs: vec![test_ref("proof"), test_ref("simulation")],
            placement_ref: Some(placement.report_ref),
            local_state_fresh: true,
            requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        })
        .expect("leaderless experimental simulation");
        assert_eq!(leaderless.decision, RAFT_DECISION_PASS);

        let missing_leaderless_evidence = run_consensus_simulation(&ConsensusSimulationInput {
            scenario: SCENARIO_LEADERLESS_NON_LEADER_PROGRESS.to_string(),
            algorithm_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            topology_ref: test_ref("topology"),
            membership_refs: members,
            fault_plan_ref: test_ref("leaderless-fault"),
            operation_ids: vec![test_ref("leaderless-operation")],
            connected_replicas: EXPERIMENTAL_CONNECTED_REPLICAS,
            proposer_ref: Some(test_ref("member-b")),
            required_evidence_refs: Vec::new(),
            placement_ref: None,
            local_state_fresh: true,
            requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        })
        .expect("leaderless missing evidence");
        assert_eq!(missing_leaderless_evidence.decision, RAFT_DECISION_DENY);
        assert!(missing_leaderless_evidence.diagnostics.join(";").contains("missing required evidence"));
    }
}
