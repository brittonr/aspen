use std::collections::BTreeMap;

use super::super::*;
use super::support::*;
use crate::world_commit::RootKind;

fn migration_request() -> WorldMergeRequest {
    let mut input = request();
    input.roots.retain(|root| root.kind == RootKind::DurableState);
    let target = WorldMergeValue {
        schema_ref: Some(schema("new-schema")),
        ..value(RootKind::DurableState, "modern-base")
    };
    input.roots[0] = WorldMergeRootInput {
        kind: RootKind::DurableState,
        base: target.clone(),
        left: WorldMergeValue {
            schema_ref: Some(schema("old-schema")),
            ..value(RootKind::DurableState, "legacy-left")
        },
        right: target,
    };
    input.profile.migrations.insert(RootKind::DurableState, WorldMigrationBinding {
        plan_ref: WorldMigrationPlanRef::new(reference("migration-plan")).expect("migration plan"),
        profile_id: "durable-state-v1-to-v2".to_string(),
        source_schema: schema("old-schema"),
        target_schema: schema("new-schema"),
        admitted: true,
    });
    input
}

fn converted(input: &WorldMergeRequest, keyed: BTreeMap<String, Vec<u8>>) -> Vec<MaterializedWorldMigration> {
    admit_world_migrations(input)
        .expect("admitted original")
        .into_iter()
        .map(|conversion| {
            let result =
                WorldMigrationResult::new(&conversion, b"modern-left".to_vec(), keyed.clone()).expect("result");
            validate_world_migration_result(&conversion, result).expect("validated result")
        })
        .collect()
}

#[test]
fn mixed_schema_conversion_keeps_original_evidence_and_generates_target_bytes() {
    // r[verify molten.audit_f06.binding]
    let input = migration_request();
    assert!(
        plan_world_merge(input.clone(), Vec::new(), None).is_err(),
        "raw source cannot replace conversion evidence"
    );
    let plan = plan_world_merge(input.clone(), converted(&input, BTreeMap::new()), None).expect("converted plan");
    assert!(plan.conflicts.is_empty());
    assert_eq!(plan.outputs[0].selected_root, None, "old root cannot carry converted bytes");
    assert_eq!(plan.outputs[0].generated_bytes.as_deref(), Some(b"modern-left".as_slice()));
    assert_eq!(plan.outputs[0].output_schema, Some(schema("new-schema")));
    assert_eq!(plan.migrations.len(), 1);
    let conversion = plan.migrations[0].conversion();
    assert_eq!(conversion.side(), WorldMergeInputSide::Left);
    assert_eq!(conversion.original_root(), input.roots[0].left.root.as_ref().expect("original root"));
    assert_eq!(conversion.binding(), &input.profile.migrations[&RootKind::DurableState]);
    assert_eq!(input.roots[0].left.schema_ref, Some(schema("old-schema")));
}

#[test]
fn stale_conversion_cannot_be_reused_after_original_context_changes() {
    // r[verify molten.audit_f06.binding]
    let input = migration_request();
    for case in ["root", "bytes", "head", "base", "policy", "profile", "bound", "binding"] {
        let materialized = converted(&input, BTreeMap::new());
        let mut changed = input.clone();
        match case {
            "root" => changed.roots[0].left.root = Some(root(RootKind::DurableState, "replacement")),
            "bytes" => changed.roots[0].left.canonical_bytes = Some(b"changed-source".to_vec()),
            "head" => changed.source_heads[0] = commit("replacement-head"),
            "base" => changed.base_head = commit("replacement-base"),
            "policy" => {
                changed.profile.policy_ref = WorldMergePolicyRef::new(reference("other-policy")).expect("policy")
            }
            "profile" => {
                changed.profile.profile_ref = WorldMergeProfileRef::new(reference("other-profile")).expect("profile")
            }
            "bound" => changed.bounds.max_value_bytes -= 1,
            "binding" => changed
                .profile
                .migrations
                .get_mut(&RootKind::DurableState)
                .expect("binding")
                .profile_id
                .push_str("-changed"),
            _ => unreachable!(),
        }
        assert!(plan_world_merge(changed, materialized, None).is_err(), "stale {case} conversion admitted");
    }
}

#[test]
fn conversion_replaces_legacy_keyed_values_before_keyed_reduction() {
    let mut input = migration_request();
    input.profile.root_modes.insert(RootKind::DurableState, WorldMergeMode::KeyedDurableValues);
    input.roots[0].base.keyed_values.insert("a".to_string(), b"base".to_vec());
    input.roots[0].right.keyed_values = input.roots[0].base.keyed_values.clone();
    input.roots[0].left.keyed_values.insert("legacy-key".to_string(), b"obsolete".to_vec());
    let result = BTreeMap::from([("a".to_string(), b"converted".to_vec())]);
    let plan = plan_world_merge(input.clone(), converted(&input, result), None).expect("keyed converted plan");
    assert!(plan.conflicts.is_empty());
    assert_eq!(plan.outputs[0].generated_values, BTreeMap::from([("a".to_string(), b"converted".to_vec())]));
}

#[test]
fn result_validation_rejects_target_identity_and_conversion_substitution() {
    let input = migration_request();
    let conversion = admit_world_migrations(&input).expect("admission").remove(0);
    for case in ["empty", "oversized", "target", "content", "conversion", "keyed-content"] {
        let mut result =
            WorldMigrationResult::new(&conversion, b"modern-left".to_vec(), BTreeMap::new()).expect("result");
        match case {
            "empty" => result.canonical_bytes.clear(),
            "oversized" => {
                result.canonical_bytes = vec![0; usize::try_from(conversion.max_output_bytes()).expect("bound") + 1]
            }
            "target" => result.target_schema = schema("other-target"),
            "content" => result.canonical_bytes = b"substituted".to_vec(),
            "conversion" => result.conversion_ref = reference("stale-conversion"),
            "keyed-content" => {
                result.keyed_values.insert("injected".to_string(), b"substitution".to_vec());
            }
            _ => unreachable!(),
        }
        assert!(validate_world_migration_result(&conversion, result).is_err(), "accepted {case} result");
    }
}

#[test]
fn normalized_targets_without_original_conversion_are_not_admission_evidence() {
    let mut input = migration_request();
    input.roots[0].left = WorldMergeValue {
        schema_ref: Some(schema("new-schema")),
        ..value(RootKind::DurableState, "modern-left")
    };
    let issues = plan_world_merge(input, Vec::new(), None)
        .expect_err("normalized raw values cannot stand in for original conversions");
    assert!(issues.contains(&WorldMergeIssue::MigrationResultMissing(RootKind::DurableState)));
}
