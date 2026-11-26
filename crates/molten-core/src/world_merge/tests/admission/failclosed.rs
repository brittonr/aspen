//! Runtime-sensitive, unavailable, undeclared and schema-mismatched merge inputs fail closed.

use super::*;

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
        plan_world_merge(runtime, Vec::new(), None)
            .expect_err("task merge denied")
            .contains(&WorldMergeIssue::RuntimeSensitiveRoot(RootKind::Tasks))
    );

    let mut ambiguous = request();
    ambiguous.common_ancestor_ambiguous = true;
    assert!(
        plan_world_merge(ambiguous, Vec::new(), None)
            .expect_err("ambiguous base denied")
            .contains(&WorldMergeIssue::AmbiguousBase)
    );

    let mut duplicate = request();
    duplicate.source_heads.push(duplicate.source_heads[0].clone());
    assert!(
        plan_world_merge(duplicate, Vec::new(), None)
            .expect_err("duplicate source denied")
            .contains(&WorldMergeIssue::DuplicateSource)
    );

    let mut bounded = request();
    bounded.bounds.max_roots = 1;
    assert!(
        plan_world_merge(bounded, Vec::new(), None)
            .expect_err("root bound denied")
            .contains(&WorldMergeIssue::RootLimitExceeded)
    );
}

fn merge_issues(input: WorldMergeRequest, handler: Option<&dyn WorldMergeHandler>) -> Vec<WorldMergeIssue> {
    plan_world_merge(input, Vec::new(), handler).expect_err("merge denied")
}

#[test]
fn equal_roots_do_not_bypass_runtime_sensitivity_availability_or_profile_modes() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    for kind in [RootKind::Tasks, RootKind::Entropy] {
        let mut runtime = request();
        runtime.profile.root_modes.insert(kind, WorldMergeMode::IdenticalOnly);
        runtime.roots.push(WorldMergeRootInput {
            kind,
            base: value(kind, "runtime-base"),
            left: value(kind, "runtime-equal"),
            right: value(kind, "runtime-equal"),
        });
        assert!(merge_issues(runtime, None).contains(&WorldMergeIssue::RuntimeSensitiveRoot(kind)));
    }

    let mut excluded = request();
    excluded.roots.push(WorldMergeRootInput {
        kind: RootKind::Policy,
        base: value(RootKind::Policy, "policy"),
        left: value(RootKind::Policy, "policy"),
        right: value(RootKind::Policy, "policy"),
    });
    assert!(!excluded.profile.root_modes.contains_key(&RootKind::Policy));
    assert!(merge_issues(excluded.clone(), None).contains(&WorldMergeIssue::ModeNotDeclared(RootKind::Policy)));

    let mut unavailable = request();
    unavailable.roots[0].right.available = false;
    assert_eq!(unavailable.roots[0].left.root, unavailable.roots[0].right.root);
    assert!(merge_issues(unavailable, None).contains(&WorldMergeIssue::UnavailableRoot(RootKind::Artifact)));

    excluded.profile.root_modes.insert(RootKind::Policy, WorldMergeMode::IdenticalOnly);
    let admitted = plan_world_merge(excluded, Vec::new(), None).expect("declared equal root merges");
    assert!(admitted.outputs.iter().any(|output| {
        output.kind == RootKind::Policy
            && matches!(&output.output, WorldMergeOutput::Selected(selected) if selected == &root(RootKind::Policy, "policy"))
    }));
}

#[test]
fn application_handler_enforces_declared_input_schema() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    let mut input = request();
    let handler_profile = WorldApplicationHandlerProfile {
        handler_ref: WorldMergeHandlerRef::new(reference("handler")).expect("handler ref"),
        behavior_ref: WorldMergeHandlerRef::new(reference("handler-behavior")).expect("behavior ref"),
        input_schema: schema("declared-handler-input"),
        output_schema: schema("schema-output"),
        policy_ref: input.profile.policy_ref.clone(),
        max_output_bytes: input.bounds.max_value_bytes,
        pure: true,
    };
    input.profile.root_modes.insert(RootKind::Schema, WorldMergeMode::ApplicationHandler);
    input.profile.handlers.insert(RootKind::Schema, handler_profile.clone());
    let declared = |label: &str| WorldMergeValue {
        schema_ref: Some(schema("declared-handler-input")),
        ..value(RootKind::Schema, label)
    };
    let handler = TestHandler {
        profile: handler_profile,
        requested_effect: false,
    };

    let mut matching = input.clone();
    matching.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: declared("base"),
        left: declared("left"),
        right: declared("right"),
    });
    plan_world_merge(matching, Vec::new(), Some(&handler)).expect("declared input schema merges");

    // Base, left, and right agree with each other but not with the handler's declared input.
    let mut mismatched = input.clone();
    mismatched.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: value(RootKind::Schema, "base"),
        left: value(RootKind::Schema, "left"),
        right: value(RootKind::Schema, "right"),
    });
    assert!(
        merge_issues(mismatched, Some(&handler))
            .contains(&WorldMergeIssue::HandlerInputSchemaMismatch(RootKind::Schema))
    );

    // The equal-root path of a handler-mode root is held to the same declared input schema.
    let mut equal_mismatched = input;
    equal_mismatched.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: value(RootKind::Schema, "base"),
        left: value(RootKind::Schema, "same"),
        right: value(RootKind::Schema, "same"),
    });
    assert!(
        merge_issues(equal_mismatched, Some(&handler))
            .contains(&WorldMergeIssue::HandlerInputSchemaMismatch(RootKind::Schema))
    );
}
