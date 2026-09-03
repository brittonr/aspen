use super::*;

// r[verify molten.prolly_map.history_independence]
// r[verify molten.prolly_map.diff]
#[test]
fn edits_share_blocks_and_diff_remains_complete_and_policy_neutral() {
    let profile = profile();
    let prior = build();
    let original = entry(UPDATE_INDEX);
    let updated = SemanticEntry {
        key: original.key.clone(),
        value: vec![UPDATED_VALUE_BYTE; VALUE_BYTES],
    };
    let plan = plan_edits(&profile, &prior.snapshot, &[MapEdit::Update(updated.clone())]).expect("edit plan");
    assert!(plan.reused_block_count > 0);
    assert!(plan.staged_blocks.len() < plan.next.snapshot.blocks.len());

    let diff = diff_maps(&profile, &prior.snapshot, &plan.next.snapshot).expect("complete diff");
    assert_eq!(diff.records.len(), 1);
    assert_eq!(diff.records[0].kind, DiffKind::Modified);
    assert_eq!(diff.records[0].key, updated.key);
    assert!(diff.complete);
    assert!(!diff.selects_merge_winner);
    assert!(diff.skipped_equal_nodes > 0);
}

#[test]
fn insert_update_delete_and_diff_failure_paths_are_explicit() {
    let profile = profile();
    let prior = build();
    let existing = entry(0);
    assert!(
        plan_edits(&profile, &prior.snapshot, &[MapEdit::Insert(existing.clone())])
            .expect_err("insert existing")
            .contains(&ProllyIssue::EditInsertExisting(existing.key.clone()))
    );

    let missing = SemanticEntry {
        key: b"missing".to_vec(),
        value: b"value".to_vec(),
    };
    assert!(
        plan_edits(&profile, &prior.snapshot, &[MapEdit::Update(missing.clone())])
            .expect_err("update missing")
            .contains(&ProllyIssue::EditUpdateMissing(missing.key.clone()))
    );
    assert!(
        plan_edits(&profile, &prior.snapshot, &[MapEdit::Delete(missing.key.clone())])
            .expect_err("delete missing")
            .contains(&ProllyIssue::EditDeleteMissing(missing.key))
    );

    let deletion =
        plan_edits(&profile, &prior.snapshot, &[MapEdit::Delete(entry(DELETE_INDEX).key)]).expect("delete existing");
    let diff = diff_maps(&profile, &prior.snapshot, &deletion.next.snapshot).expect("delete diff");
    assert_eq!(diff.records.len(), 1);
    assert_eq!(diff.records[0].kind, DiffKind::Removed);
}

// r[verify molten.prolly_map.benchmark]
#[test]
fn bounded_measurements_do_not_become_correctness_or_extraction_claims() {
    let profile = profile();
    let prior = build();
    let changed = SemanticEntry {
        key: entry(UPDATE_INDEX).key,
        value: vec![UPDATED_VALUE_BYTE; VALUE_BYTES],
    };
    let edit = plan_edits(&profile, &prior.snapshot, &[MapEdit::Update(changed)]).expect("edit");
    let diff = diff_maps(&profile, &prior.snapshot, &edit.next.snapshot).expect("diff");
    let facts = facts_from_snapshot(&profile, &edit.next.snapshot).expect("facts");
    let all_nodes = edit.next.snapshot.blocks.iter().map(|block| block.node_ref.clone()).collect::<Vec<_>>();
    let gc = plan_gc(&profile, &all_nodes, core::slice::from_ref(&edit.next.snapshot.root.top_node_ref), &[], &facts)
        .expect("gc plan");
    let result = benchmark_map(&edit.next, &edit, &diff, &gc, true).expect("benchmark");
    assert!(!result.timing_proves_correctness);
    assert!(result.block_count > 0);
    assert!(result.logical_bytes > 0);

    let mut overclaim = gc;
    overclaim.deletion_authorized = true;
    assert!(
        benchmark_map(&edit.next, &edit, &diff, &overclaim, true)
            .expect_err("deletion overclaim")
            .contains(&ProllyIssue::BenchmarkOverclaim)
    );
}
