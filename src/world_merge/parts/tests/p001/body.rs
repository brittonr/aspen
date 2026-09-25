
#[test]
fn failed_output_publication_never_publishes_a_commit() {
    // r[verify molten.world_merge.result]
    let plan = plan_with_generated_output();
    let policy = WorldMergePolicyRef::new(reference("policy")).expect("policy ref");
    let mut objects = TestObjects {
        fail: true,
        ..Default::default()
    };
    let mut conflicts = TestConflicts::default();
    let mut authority = TestAuthority { calls: 0 };
    let mut commits = TestCommits::default();

    let result = publish_world_merge(
        &mut objects,
        &mut conflicts,
        &mut authority,
        &mut commits,
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &policy,
        },
    );

    assert!(result.is_err());
    assert_eq!(commits.calls, 0);
}

#[test]
fn unresolved_conflicts_publish_only_detached_conflict_artifacts() {
    // r[verify molten.world_merge.conflicts]
    let mut plan = plan_with_generated_output();
    plan.outputs.clear();
    plan.conflicts.push(WorldMergeConflict {
        kind: RootKind::DurableState,
        key: Some("key".to_string()),
        code: "concurrent-key-change",
    });
    let policy = WorldMergePolicyRef::new(reference("policy")).expect("policy ref");
    let mut objects = TestObjects::default();
    let mut conflicts = TestConflicts::default();
    let mut authority = TestAuthority { calls: 0 };
    let mut commits = TestCommits::default();

    let result = publish_world_merge(
        &mut objects,
        &mut conflicts,
        &mut authority,
        &mut commits,
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &policy,
        },
    )
    .expect("conflict result");

    assert_eq!(conflicts.records.len(), 1);
    assert_eq!(commits.calls, 0);
    assert_eq!(authority.calls, 0);
    assert!(result.result_commit.is_none());
    let text = crate::preserves_rail::to_text(&result.receipt.value).expect("merge receipt text");
    assert!(text.contains("conflict-identity-does-not-select-a-winner"));
}

#[test]
fn canonical_diff_plan_conflict_and_result_records_are_stable() {
    // r[verify molten.world_merge.verification]
    let plan = plan_with_generated_output();
    let canonical_plan = canonical_world_merge_plan(&plan).expect("canonical plan");
    assert_eq!(canonical_plan.plan_ref, crate::preserves_rail::content_ref_from_bytes(&canonical_plan.bytes));
    let diff = WorldDiffReport {
        base_head: plan.base_head.clone(),
        source_heads: plan.source_heads.clone(),
        roots: vec![WorldRootDiff {
            kind: RootKind::Artifact,
            class: WorldRootDiffClass::Equal,
        }],
    };
    let first = canonical_world_diff(&diff).expect("canonical diff");
    let second = canonical_world_diff(&diff).expect("stable diff");
    assert_eq!(first.bytes, second.bytes);
}
