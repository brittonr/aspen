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
    let first = plan_world_merge(input.clone(), Vec::new(), None).expect("merge plan");
    let second = plan_world_merge(input, Vec::new(), None).expect("stable merge plan");

    assert_eq!(first, second);
    assert_eq!(first.outputs.len(), EXPECTED_OUTPUT_ROOTS);
    assert!(first.conflicts.is_empty());
    assert_eq!(first.source_heads.len(), EXPECTED_OUTPUT_ROOTS);
    assert!(first.plan_ref.as_str().starts_with("blake3:"));
    let durable = first.outputs.iter().find(|output| output.kind == RootKind::DurableState).expect("durable output");
    assert!(matches!(
        &durable.output,
        WorldMergeOutput::Selected(selected) if selected == &root(RootKind::DurableState, "left-state")
    ));
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
    let merged = plan_world_merge(input.clone(), Vec::new(), None).expect("disjoint keyed merge");
    let output = merged
        .outputs
        .iter()
        .find(|output| output.kind == RootKind::DurableState)
        .expect("generated durable output");
    let WorldMergeOutput::Generated(generated) = &output.output else {
        panic!("keyed durable output is generated data");
    };
    assert_eq!(generated.values["a"], b"left-a");
    assert_eq!(generated.values["b"], b"right-b");

    input
        .roots
        .iter_mut()
        .find(|root| root.kind == RootKind::DurableState)
        .expect("durable input")
        .right = keyed("right", &[("a", "right-a"), ("b", "base-b")]);
    let conflicted = plan_world_merge(input, Vec::new(), None).expect("conflict plan");
    assert!(conflicted.conflicts.iter().any(|conflict| conflict.key.as_deref() == Some("a")));
    assert!(!conflicted.outputs.iter().any(|output| output.kind == RootKind::DurableState));
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
    let plan = plan_world_merge(input.clone(), Vec::new(), Some(&handler)).expect("application merge");
    assert!(plan.outputs.iter().any(|output| {
        output.kind == RootKind::Schema
            && matches!(&output.output, WorldMergeOutput::Generated(generated) if generated.bytes.is_some())
    }));

    let effectful = TestHandler {
        profile: handler_profile,
        requested_effect: true,
    };
    assert!(
        plan_world_merge(input, Vec::new(), Some(&effectful))
            .expect_err("effectful handler denied")
            .contains(&WorldMergeIssue::HandlerEffectRequested(RootKind::Schema))
    );
}

#[test]
fn absent_outputs_are_admitted_only_under_present_root_profiles() {
    // r[verify molten.audit_f13.output_kind]
    // r[verify molten.audit_f13.absence]
    let mut absent_request = request();
    {
        let artifact = absent_request.roots.iter_mut().find(|root| root.kind == RootKind::Artifact).expect("artifact");
        artifact.base = absent(Some(schema("artifact-schema")));
        artifact.left = absent(Some(schema("artifact-schema")));
        artifact.right = absent(Some(schema("artifact-schema")));
    }
    let plan = plan_world_merge(absent_request.clone(), Vec::new(), None).expect("absent artifact output");
    let artifact = plan.outputs.iter().find(|output| output.kind == RootKind::Artifact).expect("artifact output");
    assert_eq!(artifact.output, WorldMergeOutput::Absent);

    let mut incompatible = absent_request.clone();
    incompatible.roots[0].right = absent(None);
    assert!(
        plan_world_merge(incompatible, Vec::new(), None)
            .expect_err("incompatible absent metadata denied")
            .contains(&WorldMergeIssue::IncompatibleSchema(RootKind::Artifact))
    );

    let mut content_mode = absent_request.clone();
    content_mode.profile.root_modes.insert(RootKind::Artifact, WorldMergeMode::KeyedDurableValues);
    assert!(
        plan_world_merge(content_mode, Vec::new(), None)
            .expect_err("content-producing profile requires a present root")
            .contains(&WorldMergeIssue::UnsupportedAbsence(RootKind::Artifact))
    );

    let mut undeclared = absent_request.clone();
    undeclared.roots[0].kind = RootKind::Policy;
    assert!(
        plan_world_merge(undeclared, Vec::new(), None)
            .expect_err("undeclared kind cannot be absent")
            .contains(&WorldMergeIssue::ModeNotDeclared(RootKind::Policy))
    );

    let mut runtime = absent_request;
    runtime.roots[0].kind = RootKind::Tasks;
    runtime.profile.root_modes.insert(RootKind::Tasks, WorldMergeMode::AncestorReplacement);
    assert!(
        plan_world_merge(runtime, Vec::new(), None)
            .expect_err("runtime-sensitive absence denied")
            .contains(&WorldMergeIssue::RuntimeSensitiveRoot(RootKind::Tasks))
    );
}

#[test]
fn absent_and_generated_empty_outputs_have_distinct_plan_identities() {
    // r[verify molten.audit_f13.compatibility]
    let mut input = request();
    input.profile.root_modes.insert(RootKind::DurableState, WorldMergeMode::KeyedDurableValues);
    {
        let durable = input.roots.iter_mut().find(|root| root.kind == RootKind::DurableState).expect("durable");
        durable.base = keyed("base", &[]);
        durable.left = keyed("left", &[]);
        durable.right = keyed("right", &[]);
    }
    let mut generated = plan_world_merge(input, Vec::new(), None).expect("generated empty plan");
    let is_empty_generated = generated
        .outputs
        .iter()
        .find(|output| output.kind == RootKind::DurableState)
        .is_some_and(|output| {
            matches!(&output.output, WorldMergeOutput::Generated(output) if output.values.is_empty() && output.bytes.is_none())
        });
    assert!(is_empty_generated, "keyed empty result is generated data");
    assert!(validate_world_merge_plan(&generated).is_ok());

    let generated_output = generated
        .outputs
        .iter_mut()
        .find(|output| output.kind == RootKind::DurableState)
        .expect("generated output");
    generated_output.output = WorldMergeOutput::Absent;
    assert_eq!(
        validate_world_merge_plan(&generated).expect_err("absence cannot reuse generated identity"),
        WorldMergeIssue::PlanIdentityMismatch
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
