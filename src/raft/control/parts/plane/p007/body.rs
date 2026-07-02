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
        assert_eq!(status.log_count, 3);
        assert_eq!(status.snapshot_count, 1);
        assert_eq!(status.session_count, 1);
        assert!(status.receipt_count >= 3);
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
}
