use super::super::*;
use super::support::*;
use crate::world_commit::RootKind;

#[test]
fn diff_reports_equal_changed_unavailable_incompatible_and_excluded_roots() {
    // r[verify molten.world_merge.diff]
    let mut input = request();
    input.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: value(RootKind::Schema, "schema"),
        left: value(RootKind::Schema, "schema"),
        right: value(RootKind::Schema, "changed-schema"),
    });
    input.roots.push(WorldMergeRootInput {
        kind: RootKind::RuntimeProfile,
        base: value(RootKind::RuntimeProfile, "runtime"),
        left: value(RootKind::RuntimeProfile, "runtime"),
        right: WorldMergeValue {
            available: false,
            ..value(RootKind::RuntimeProfile, "runtime")
        },
    });
    input.roots.push(WorldMergeRootInput {
        kind: RootKind::Policy,
        base: value(RootKind::Policy, "policy"),
        left: value(RootKind::Policy, "left-policy"),
        right: WorldMergeValue {
            schema_ref: Some(schema("other-policy-schema")),
            ..value(RootKind::Policy, "right-policy")
        },
    });
    input.profile.root_modes.insert(RootKind::Schema, WorldMergeMode::IdenticalOnly);
    input.profile.root_modes.insert(RootKind::RuntimeProfile, WorldMergeMode::IdenticalOnly);
    input.profile.root_modes.insert(RootKind::Policy, WorldMergeMode::IdenticalOnly);

    let report = diff_world_roots(&input).expect("diff report");

    assert!(report.roots.contains(&WorldRootDiff {
        kind: RootKind::Artifact,
        class: WorldRootDiffClass::Equal,
    }));
    assert!(report.roots.contains(&WorldRootDiff {
        kind: RootKind::DurableState,
        class: WorldRootDiffClass::Changed,
    }));
    assert!(report.roots.contains(&WorldRootDiff {
        kind: RootKind::RuntimeProfile,
        class: WorldRootDiffClass::Unavailable,
    }));
    assert!(report.roots.contains(&WorldRootDiff {
        kind: RootKind::Policy,
        class: WorldRootDiffClass::Incompatible,
    }));

    input.profile.root_modes.remove(&RootKind::Schema);
    let excluded = diff_world_roots(&input).expect("excluded diff");
    assert!(excluded.roots.contains(&WorldRootDiff {
        kind: RootKind::Schema,
        class: WorldRootDiffClass::ProfileExcluded,
    }));
}

#[test]
fn absent_and_duplicate_inputs_are_never_reported_as_equal() {
    // r[verify molten.world_merge.diff]
    let mut input = request();
    input.roots[0].left.root = None;
    let report = diff_world_roots(&input).expect("absent diff");
    assert_eq!(report.roots[0].class, WorldRootDiffClass::Absent);

    input.roots.push(input.roots[0].clone());
    assert!(
        diff_world_roots(&input)
            .expect_err("duplicate root denied")
            .contains(&WorldMergeIssue::DuplicateRoot)
    );
}
