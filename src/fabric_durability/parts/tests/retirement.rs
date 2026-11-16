// Retirement deduplicates payload deletion without deleting retained snapshot content.
// r[verify molten.fabric_durability.snapshot_compaction]
#[test]
fn redb_retirement_deduplicates_payloads_and_preserves_shared_kept_content() {
    for is_shared_kept in [false, true] {
        let root = crate::test_support::process_workspace("fabric-durability-shared-retirement").expect("workspace");
        let live = profile(DurableAdapterKind::LiveRedb);
        let mut adapter = RedbDurableStateAdapter::open(&root, live.clone(), descriptor()).expect("open adapter");
        adapter.append(&append_request(DurabilityLevel::MachineLoss)).expect("covered log record");
        let snapshot_refs = ["first", "second", "other", "keep"]
            .map(|label| format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex()));
        let kept_bytes: &[u8] = if is_shared_kept {
            b"shared-payload"
        } else {
            b"kept-payload"
        };
        let payloads: [&[u8]; 4] = [b"shared-payload", b"shared-payload", b"other-payload", kept_bytes];
        let content_refs = payloads.map(|bytes| format!("blake3:{}", blake3::hash(bytes).to_hex()));
        for ((snapshot_ref, content_ref), bytes) in snapshot_refs.iter().zip(&content_refs).zip(payloads) {
            adapter
                .create_snapshot(
                    &SnapshotRequest {
                        kind: SnapshotKind::Snapshot,
                        generation: GENERATION,
                        snapshot_ref: snapshot_ref.clone(),
                        content_ref: content_ref.clone(),
                        ordered_state_ref: ORDERED_REF.to_string(),
                        covered_log_sequence: Some(FIRST_SEQUENCE),
                        durability: DurabilityLevel::MachineLoss,
                    },
                    bytes,
                )
                .expect("publish snapshot sharing or owning a payload");
        }
        let kept_ref = snapshot_refs.last().expect("kept snapshot");
        let retired = adapter
            .retire_superseded_snapshots(GENERATION, kept_ref, Some(AUTHORITY_REF))
            .expect("retire each unreferenced payload once");
        assert_eq!(retired.outcome, MutationOutcome::Durable);
        drop(adapter);

        let reopened = RedbDurableStateAdapter::open(&root, live, descriptor()).expect("reopen adapter");
        assert_eq!(reopened.state().snapshots.keys().collect::<Vec<_>>(), vec![kept_ref]);
        let restored = reopened.restore_snapshot(kept_ref, GENERATION).expect("restore kept payload");
        let kept_content_ref = content_refs.last().expect("kept content");
        assert_eq!(&restored.snapshot.content_ref, kept_content_ref);
        for content_ref in content_refs.iter().collect::<std::collections::BTreeSet<_>>() {
            let stem = content_ref.strip_prefix("blake3:").expect("content hash");
            let path = root.join("snapshots").join(format!("{stem}.bin"));
            assert_eq!(
                path.try_exists().expect("observe snapshot payload"),
                content_ref == kept_content_ref,
                "physical retention for {content_ref}, shared keeper={is_shared_kept}",
            );
        }
    }
}
