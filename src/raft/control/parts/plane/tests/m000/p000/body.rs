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

    fn cluster_config_manifest_input() -> RaftGroupManifestInput {
        RaftGroupManifestInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            members: vec![test_ref("member-a"), test_ref("member-b"), test_ref("member-c")],
            state_machine: CONTROL_REGISTRY_STATE_MACHINE.to_string(),
            command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
            read_mode: READ_MODE_READ_INDEX.to_string(),
            snapshot_policy_ref: test_ref("snapshot-policy"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        }
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
