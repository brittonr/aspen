use super::super::*;
use super::support::*;
use crate::world_commit::RootKind;

const EXPECTED_OUTPUT_ROOTS: usize = 2;

struct TestHandler {
    profile: WorldApplicationHandlerProfile,
    requested_effect: bool,
}

impl WorldMergeHandler for TestHandler {
    fn profile(&self) -> &WorldApplicationHandlerProfile {
        &self.profile
    }

    fn merge(&self, input: &WorldApplicationMergeInput<'_>) -> Result<WorldApplicationMergeOutput, &'static str> {
        let mut output = input.left.to_vec();
        output.extend_from_slice(input.right);
        Ok(WorldApplicationMergeOutput {
            canonical_bytes: output,
            requested_effect: self.requested_effect,
        })
    }
}

#[test]
fn ancestor_replacement_and_stable_plan_identity_preserve_all_sources() {
    // r[verify molten.world_merge.admission]
    let input = request();
    let first = plan_world_merge(&input, None).expect("merge plan");
    let second = plan_world_merge(&input, None).expect("stable merge plan");

    assert_eq!(first, second);
    assert_eq!(first.outputs.len(), EXPECTED_OUTPUT_ROOTS);
    assert!(first.conflicts.is_empty());
    assert_eq!(first.source_heads.len(), EXPECTED_OUTPUT_ROOTS);
    assert!(first.plan_ref.as_str().starts_with("blake3:"));
    let durable = first.outputs.iter().find(|output| output.kind == RootKind::DurableState).expect("durable output");
    assert_eq!(durable.selected_root, Some(root(RootKind::DurableState, "left-state")));
}

#[test]
fn keyed_values_merge_disjoint_changes_and_preserve_concurrent_conflicts() {
    // r[verify molten.world_merge.handlers]
    let mut input = request();
    input.profile.root_modes.insert(RootKind::DurableState, WorldMergeMode::KeyedDurableValues);
    {
        let durable = input.roots.iter_mut().find(|root| root.kind == RootKind::DurableState).expect("durable input");
        durable.base = keyed("base", &[("a", "base-a"), ("b", "base-b")]);
        durable.left = keyed("left", &[("a", "left-a"), ("b", "base-b")]);
        durable.right = keyed("right", &[("a", "base-a"), ("b", "right-b")]);
    }
    let merged = plan_world_merge(&input, None).expect("disjoint keyed merge");
    let output = merged
        .outputs
        .iter()
        .find(|output| output.kind == RootKind::DurableState)
        .expect("generated durable output");
    assert_eq!(output.generated_values["a"], b"left-a");
    assert_eq!(output.generated_values["b"], b"right-b");

    input
        .roots
        .iter_mut()
        .find(|root| root.kind == RootKind::DurableState)
        .expect("durable input")
        .right = keyed("right", &[("a", "right-a"), ("b", "base-b")]);
    let conflicted = plan_world_merge(&input, None).expect("conflict plan");
    assert!(conflicted.conflicts.iter().any(|conflict| conflict.key.as_deref() == Some("a")));
    assert!(!conflicted.outputs.iter().any(|output| output.kind == RootKind::DurableState));
}

#[test]
fn schema_change_requires_exact_admitted_migration_binding() {
    // r[verify molten.world_merge.admission]
    let mut input = request();
    let durable = input.roots.iter_mut().find(|root| root.kind == RootKind::DurableState).expect("durable input");
    durable.base.schema_ref = Some(schema("old-schema"));
    durable.right.schema_ref = Some(schema("old-schema"));
    durable.left.schema_ref = Some(schema("new-schema"));
    let missing = plan_world_merge(&input, None).expect_err("missing migration denied");
    assert!(missing.contains(&WorldMergeIssue::MigrationRequired(RootKind::DurableState)));

    input.profile.migrations.insert(RootKind::DurableState, WorldMigrationBinding {
        plan_ref: WorldMigrationPlanRef::new(reference("migration-plan")).expect("migration plan ref"),
        profile_id: "durable-state-v1-to-v2".to_string(),
        source_schema: schema("old-schema"),
        target_schema: schema("new-schema"),
        admitted: true,
    });
    assert!(plan_world_merge(&input, None).is_ok());

    input.profile.migrations.get_mut(&RootKind::DurableState).expect("migration binding").profile_id = String::new();
    assert!(
        plan_world_merge(&input, None)
            .expect_err("invalid migration profile denied")
            .contains(&WorldMergeIssue::MigrationProfileInvalid(RootKind::DurableState))
    );
}

#[test]
fn pure_application_handler_is_exact_and_effect_requests_are_denied() {
    // r[verify molten.world_merge.handlers]
    let mut input = request();
    let handler_profile = WorldApplicationHandlerProfile {
        handler_ref: WorldMergeHandlerRef::new(reference("handler")).expect("handler ref"),
        behavior_ref: WorldMergeHandlerRef::new(reference("handler-behavior")).expect("behavior ref"),
        input_schema: schema("schema-schema"),
        output_schema: schema("schema-output"),
        policy_ref: input.profile.policy_ref.clone(),
        max_output_bytes: input.bounds.max_value_bytes,
        pure: true,
    };
    input.profile.root_modes.insert(RootKind::Schema, WorldMergeMode::ApplicationHandler);
    input.profile.handlers.insert(RootKind::Schema, handler_profile.clone());
    input.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: value(RootKind::Schema, "base"),
        left: value(RootKind::Schema, "left"),
        right: value(RootKind::Schema, "right"),
    });
    let handler = TestHandler {
        profile: handler_profile.clone(),
        requested_effect: false,
    };
    let plan = plan_world_merge(&input, Some(&handler)).expect("application merge");
    assert!(
        plan.outputs
            .iter()
            .any(|output| output.kind == RootKind::Schema && output.generated_bytes.is_some())
    );

    let effectful = TestHandler {
        profile: handler_profile,
        requested_effect: true,
    };
    assert!(
        plan_world_merge(&input, Some(&effectful))
            .expect_err("effectful handler denied")
            .contains(&WorldMergeIssue::HandlerEffectRequested(RootKind::Schema))
    );
}

#[test]
fn runtime_sensitive_ambiguous_duplicate_and_bounded_inputs_fail_closed() {
    // r[verify molten.world_merge.verification]
    let mut runtime = request();
    runtime.profile.root_modes.insert(RootKind::Tasks, WorldMergeMode::AncestorReplacement);
    runtime.roots.push(WorldMergeRootInput {
        kind: RootKind::Tasks,
        base: value(RootKind::Tasks, "tasks-base"),
        left: value(RootKind::Tasks, "tasks-left"),
        right: value(RootKind::Tasks, "tasks-right"),
    });
    assert!(
        plan_world_merge(&runtime, None)
            .expect_err("task merge denied")
            .contains(&WorldMergeIssue::RuntimeSensitiveRoot(RootKind::Tasks))
    );

    let mut ambiguous = request();
    ambiguous.common_ancestor_ambiguous = true;
    assert!(
        plan_world_merge(&ambiguous, None)
            .expect_err("ambiguous base denied")
            .contains(&WorldMergeIssue::AmbiguousBase)
    );

    let mut duplicate = request();
    duplicate.source_heads.push(duplicate.source_heads[0].clone());
    assert!(
        plan_world_merge(&duplicate, None)
            .expect_err("duplicate source denied")
            .contains(&WorldMergeIssue::DuplicateSource)
    );

    let mut bounded = request();
    bounded.bounds.max_roots = 1;
    assert!(
        plan_world_merge(&bounded, None)
            .expect_err("root bound denied")
            .contains(&WorldMergeIssue::RootLimitExceeded)
    );
}
