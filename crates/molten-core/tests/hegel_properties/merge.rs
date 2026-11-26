//! World merge: three-way keyed durable-value laws.

type KeyedValues = std::collections::BTreeMap<String, Vec<u8>>;

const KEYS: [&str; 5] = ["alpha", "beta", "gamma", "delta", "epsilon"];
const VALUE_VARIANTS: u8 = 3;

/// Expected per-key outcome of a three-way keyed merge.
enum KeyOutcome {
    Value(String, Vec<u8>),
    Removed,
    Conflict(String),
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn content_label(values: &KeyedValues) -> String {
    let entries = values
        .iter()
        .map(|(key, value)| format!("{key}={}", String::from_utf8_lossy(value)))
        .collect::<Vec<_>>();
    format!("keyed-state|{}", entries.join("|"))
}

fn keyed(values: &KeyedValues) -> Option<molten_core::world_merge::WorldMergeValue> {
    Some(molten_core::world_merge::WorldMergeValue {
        // Equal maps name the same root, matching a content-addressed world.
        root: Some(
            molten_core::world_commit::WorldRootRef::parse(
                molten_core::world_commit::RootKind::DurableState,
                reference(&content_label(values)),
            )
            .ok()?,
        ),
        schema_ref: Some(molten_core::world_merge::WorldMergeSchemaRef::new(reference("durable-schema")).ok()?),
        available: true,
        canonical_bytes: None,
        keyed_values: values.clone(),
    })
}

fn request(
    base: &KeyedValues,
    left: &KeyedValues,
    right: &KeyedValues,
) -> Option<molten_core::world_merge::WorldMergeRequest> {
    Some(molten_core::world_merge::WorldMergeRequest {
        base_head: molten_core::world_commit::WorldCommitRef::new(reference("base-head")).ok()?,
        source_heads: vec![
            molten_core::world_commit::WorldCommitRef::new(reference("left-head")).ok()?,
            molten_core::world_commit::WorldCommitRef::new(reference("right-head")).ok()?,
        ],
        common_ancestor_verified: true,
        common_ancestor_ambiguous: false,
        roots: vec![molten_core::world_merge::WorldMergeRootInput {
            kind: molten_core::world_commit::RootKind::DurableState,
            base: keyed(base)?,
            left: keyed(left)?,
            right: keyed(right)?,
        }],
        profile: molten_core::world_merge::WorldMergeProfile {
            profile_ref: molten_core::world_merge::WorldMergeProfileRef::new(reference("merge-profile")).ok()?,
            policy_ref: molten_core::world_merge::WorldMergePolicyRef::new(reference("merge-policy")).ok()?,
            root_modes: std::collections::BTreeMap::from([(
                molten_core::world_commit::RootKind::DurableState,
                molten_core::world_merge::WorldMergeMode::KeyedDurableValues,
            )]),
            migrations: std::collections::BTreeMap::new(),
            handlers: std::collections::BTreeMap::new(),
        },
        bounds: molten_core::world_merge::WorldMergeBounds::standard(),
    })
}

fn draw_side(tc: &hegel::TestCase) -> KeyedValues {
    KEYS.iter()
        .filter_map(|key| {
            // Variant 0 is absence; other variants are small values so sides collide often.
            let variant = tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(VALUE_VARIANTS));
            (variant > 0).then(|| ((*key).to_string(), format!("{key}-{variant}").into_bytes()))
        })
        .collect()
}

fn plan_keyed(
    base: &KeyedValues,
    left: &KeyedValues,
    right: &KeyedValues,
) -> Option<molten_core::world_merge::WorldMergePlan> {
    molten_core::world_merge::plan_world_merge(request(base, left, right)?, Vec::new(), None).ok()
}

fn state_root(values: &KeyedValues) -> Option<molten_core::world_commit::WorldRootRef> {
    keyed(values)?.root
}

/// The merged durable state, named by its content root: a generated map is named by the root of its
/// values, and a selected root (the planner's shortcut when both sides already agree) names itself.
/// `None` means no durable output; `Some(None)` would mean an absent root.
fn merged_root(
    plan: &molten_core::world_merge::WorldMergePlan,
) -> Option<Option<molten_core::world_commit::WorldRootRef>> {
    plan.outputs
        .iter()
        .find(|output| output.kind == molten_core::world_commit::RootKind::DurableState)
        .map(|output| match &output.output {
            molten_core::world_merge::WorldMergeOutput::Generated(generated) => state_root(&generated.values),
            molten_core::world_merge::WorldMergeOutput::Selected(selected) => Some(selected.clone()),
            molten_core::world_merge::WorldMergeOutput::Absent => None,
        })
}

fn conflict_keys(plan: &molten_core::world_merge::WorldMergePlan) -> Vec<String> {
    plan.conflicts.iter().filter_map(|conflict| conflict.key.clone()).collect()
}

fn expected_outcome(key: &str, base: &KeyedValues, left: &KeyedValues, right: &KeyedValues) -> KeyOutcome {
    let (original, ours, theirs) = (base.get(key), left.get(key), right.get(key));
    let is_left_changed = ours != original;
    let is_right_changed = theirs != original;
    if is_left_changed && is_right_changed && ours != theirs {
        return KeyOutcome::Conflict(key.to_string());
    }
    match if is_left_changed { ours } else { theirs } {
        Some(value) => KeyOutcome::Value(key.to_string(), value.clone()),
        None => KeyOutcome::Removed,
    }
}

// Every one-sided change survives, untouched keys keep the base, and a key conflicts exactly when
// both sides changed it to different results. A conflicted merge publishes no partial durable
// output.
#[hegel::test(test_cases = 150, derandomize = true, database = None)]
fn hegel_keyed_merge_preserves_one_sided_changes_and_reports_exact_conflicts(tc: hegel::TestCase) {
    let base = draw_side(&tc);
    let left = draw_side(&tc);
    let right = draw_side(&tc);
    let outcomes = KEYS.iter().map(|key| expected_outcome(key, &base, &left, &right)).collect::<Vec<_>>();
    let mut expected_conflicts = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            KeyOutcome::Conflict(key) => Some(key.clone()),
            KeyOutcome::Value(..) | KeyOutcome::Removed => None,
        })
        .collect::<Vec<_>>();
    expected_conflicts.sort();
    let expected = outcomes
        .into_iter()
        .filter_map(|outcome| match outcome {
            KeyOutcome::Value(key, value) => Some((key, value)),
            KeyOutcome::Removed | KeyOutcome::Conflict(_) => None,
        })
        .collect::<KeyedValues>();

    let plan = plan_keyed(&base, &left, &right);
    assert_eq!(
        plan.as_ref().map(conflict_keys),
        Some(expected_conflicts.clone()),
        "conflicts must be exactly the divergent keys"
    );
    let merged = plan.as_ref().map(merged_root);
    if expected_conflicts.is_empty() {
        assert_eq!(merged, Some(Some(state_root(&expected))), "merged state must keep every one-sided change");
    } else {
        assert_eq!(merged, Some(None), "a conflicted keyed root must not publish partial output");
    }
}

// Merging is symmetric in its sources and deterministic: swapping sides changes neither the merged
// state nor the conflict set, and replanning the same request yields the same plan identity.
#[hegel::test(test_cases = 100, derandomize = true, database = None)]
fn hegel_keyed_merge_is_symmetric_and_deterministic(tc: hegel::TestCase) {
    let base = draw_side(&tc);
    let left = draw_side(&tc);
    let right = draw_side(&tc);
    let forward = plan_keyed(&base, &left, &right);
    let swapped = plan_keyed(&base, &right, &left);
    assert!(forward.is_some(), "a valid keyed request must plan");
    assert_eq!(forward.as_ref().map(merged_root), swapped.as_ref().map(merged_root));
    assert_eq!(forward.as_ref().map(|plan| &plan.conflicts), swapped.as_ref().map(|plan| &plan.conflicts));
    assert_eq!(plan_keyed(&base, &left, &right), forward, "replanning must reproduce the plan and its identity");
}

// Identity laws: an unchanged side contributes nothing, and agreeing sides merge to their common
// state.
#[hegel::test(test_cases = 100, derandomize = true, database = None)]
fn hegel_keyed_merge_identity_laws_hold(tc: hegel::TestCase) {
    let base = draw_side(&tc);
    let changed = draw_side(&tc);
    for plan in [
        plan_keyed(&base, &changed, &base),
        plan_keyed(&base, &base, &changed),
        plan_keyed(&base, &changed, &changed),
    ] {
        assert_eq!(plan.as_ref().map(conflict_keys), Some(Vec::new()), "identity merges never conflict");
        assert_eq!(plan.as_ref().map(merged_root), Some(Some(state_root(&changed))));
    }
    let unchanged = plan_keyed(&base, &base, &base);
    assert_eq!(unchanged.as_ref().map(merged_root), Some(Some(state_root(&base))));
}
