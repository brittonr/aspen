use std::collections::BTreeMap;

use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::*;

use super::*;

const MERGE_SCHEMA_COUNT: usize = 4;
const MERGE_SCHEMA_FIELD_COUNT: usize = 6;
const SCHEMA_ID_FIELD_INDEX: usize = 4;
const MERGE_SCHEMA_ARTIFACTS: [&str; MERGE_SCHEMA_COUNT] = [
    include_str!("../../schemas/preserves-boundaries/molten-world-diff-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-merge-plan-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-merge-conflict-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-merge-result-v1.preserves"),
];
const EXPECTED_MERGE_SCHEMA_IDS: [&str; MERGE_SCHEMA_COUNT] = [
    WORLD_DIFF_SCHEMA,
    WORLD_MERGE_PLAN_SCHEMA,
    WORLD_MERGE_CONFLICT_SCHEMA,
    WORLD_MERGE_RESULT_SCHEMA,
];

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}

fn root(kind: RootKind, label: &str) -> WorldRootRef {
    WorldRootRef::parse(kind, reference(label)).expect("root ref")
}

fn schema() -> WorldMergeSchemaRef {
    WorldMergeSchemaRef::new(reference("schema")).expect("schema ref")
}

fn plan_with_generated_output() -> WorldMergePlan {
    WorldMergePlan {
        plan_ref: WorldMergePlanRef::new(reference("plan")).expect("plan ref"),
        base_head: commit("base"),
        source_heads: vec![commit("left"), commit("right")],
        outputs: vec![
            WorldMergedRoot {
                kind: RootKind::Artifact,
                selected_root: Some(root(RootKind::Artifact, "artifact")),
                generated_values: BTreeMap::new(),
                generated_bytes: None,
                output_schema: Some(schema()),
            },
            WorldMergedRoot {
                kind: RootKind::DurableState,
                selected_root: None,
                generated_values: BTreeMap::from([("key".to_string(), b"value".to_vec())]),
                generated_bytes: None,
                output_schema: Some(schema()),
            },
        ],
        conflicts: Vec::new(),
    }
}

#[derive(Default)]
struct TestObjects {
    persisted: Vec<WorldRootRef>,
    fail: bool,
}

impl WorldMergeObjectPort for TestObjects {
    fn load_root(
        &mut self,
        _root: &WorldRootRef,
        _maximum_bytes: u64,
    ) -> std::result::Result<Vec<u8>, WorldMergePortError> {
        Ok(b"loaded-root".to_vec())
    }

    fn persist_generated_root(
        &mut self,
        kind: RootKind,
        _schema_ref: &WorldMergeSchemaRef,
        canonical_bytes: &[u8],
    ) -> std::result::Result<WorldRootRef, WorldMergePortError> {
        if self.fail {
            return Err(WorldMergePortError::new("persist", "injected failure"));
        }
        let root = WorldRootRef::parse(kind, crate::preserves_rail::content_ref_from_bytes(canonical_bytes))
            .map_err(|error| WorldMergePortError::new("identity", format!("{error:?}")))?;
        self.persisted.push(root.clone());
        Ok(root)
    }
}

struct TestMigrations;

impl WorldMergeMigrationPort for TestMigrations {
    fn materialize_migration(
        &mut self,
        _binding: &WorldMigrationBinding,
        source_bytes: &[u8],
    ) -> std::result::Result<Vec<u8>, WorldMergePortError> {
        Ok(source_bytes.to_vec())
    }
}

struct TestHandlers;

impl WorldMergeHandlerPort for TestHandlers {
    fn load_handler(
        &mut self,
        _profile: &WorldApplicationHandlerProfile,
    ) -> std::result::Result<Box<dyn WorldMergeHandler>, WorldMergePortError> {
        Err(WorldMergePortError::new("handler", "no application handler is configured"))
    }
}

#[derive(Default)]
struct TestConflicts {
    records: Vec<String>,
}

impl WorldMergeConflictPort for TestConflicts {
    fn persist_conflict(
        &mut self,
        conflict_ref: &str,
        _canonical_bytes: &[u8],
    ) -> std::result::Result<(), WorldMergePortError> {
        self.records.push(conflict_ref.to_string());
        Ok(())
    }
}

struct TestAuthority {
    calls: usize,
}

impl WorldMergeAuthorityPort for TestAuthority {
    fn recheck_merge_authority(
        &mut self,
        _source_heads: &[WorldCommitRef],
        _policy_ref: &WorldMergePolicyRef,
    ) -> std::result::Result<String, WorldMergePortError> {
        self.calls = self.calls.saturating_add(1);
        Ok(reference("authority"))
    }
}

#[derive(Default)]
struct TestCommits {
    calls: usize,
    observed_roots: Vec<WorldRootRef>,
}

impl WorldMergeCommitPort for TestCommits {
    fn publish_merge_commit(
        &mut self,
        _base_head: &WorldCommitRef,
        _source_heads: &[WorldCommitRef],
        roots: &[WorldRootRef],
    ) -> std::result::Result<WorldCommitRef, WorldMergePortError> {
        self.calls = self.calls.saturating_add(1);
        self.observed_roots = roots.to_vec();
        Ok(commit("merged"))
    }
}

#[test]
fn preparation_loads_root_bytes_before_pure_planning() {
    // r[verify molten.world_merge.handlers]
    let artifact = root(RootKind::Artifact, "artifact");
    let schema_ref = schema();
    let merge_value = WorldMergeValue {
        root: Some(artifact.clone()),
        schema_ref: Some(schema_ref),
        available: true,
        canonical_bytes: None,
        keyed_values: BTreeMap::new(),
    };
    let profile = WorldMergeProfile {
        profile_ref: WorldMergeProfileRef::new(reference("profile")).expect("profile ref"),
        policy_ref: WorldMergePolicyRef::new(reference("policy")).expect("policy ref"),
        root_modes: BTreeMap::from([(RootKind::Artifact, WorldMergeMode::AncestorReplacement)]),
        migrations: BTreeMap::new(),
        handlers: BTreeMap::new(),
    };
    let request = WorldMergeRequest {
        base_head: commit("base"),
        source_heads: vec![commit("left"), commit("right")],
        common_ancestor_verified: true,
        common_ancestor_ambiguous: false,
        roots: vec![WorldMergeRootInput {
            kind: RootKind::Artifact,
            base: merge_value.clone(),
            left: WorldMergeValue {
                root: Some(root(RootKind::Artifact, "left-artifact")),
                ..merge_value.clone()
            },
            right: merge_value,
        }],
        profile,
        bounds: WorldMergeBounds::standard(),
    };
    let mut objects = TestObjects::default();
    let mut migrations = TestMigrations;
    let mut handlers = TestHandlers;

    let plan =
        prepare_world_merge_plan(&mut objects, &mut migrations, &mut handlers, &request).expect("prepared merge plan");

    assert_eq!(plan.outputs[0].selected_root, Some(root(RootKind::Artifact, "left-artifact")));
}

#[test]
fn checked_schema_artifacts_match_merge_schema_ids() {
    // r[verify molten.world_merge.verification]
    for (source, expected) in MERGE_SCHEMA_ARTIFACTS.iter().zip(EXPECTED_MERGE_SCHEMA_IDS) {
        let value = crate::preserves_rail::parse_text(source).expect("merge schema artifact");
        let fields = crate::preserves_rail::simple_record_fields(
            &value,
            "preserves-boundary-schema-artifact-v1",
            MERGE_SCHEMA_FIELD_COUNT,
        )
        .expect("merge schema fields");
        assert_eq!(
            crate::preserves_rail::record_string_field(&fields[SCHEMA_ID_FIELD_INDEX], "schema-id", "schema id",)
                .expect("schema id"),
            expected
        );
    }
}

#[test]
fn generated_roots_persist_before_one_causal_commit() {
    // r[verify molten.world_merge.result]
    let plan = plan_with_generated_output();
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
    .expect("publish merge");

    assert_eq!(objects.persisted.len(), 1);
    assert_eq!(commits.calls, 1);
    assert_eq!(authority.calls, 1);
    assert_eq!(commits.observed_roots, result.output_roots);
    assert_eq!(result.result_commit, Some(commit("merged")));
    assert!(result.conflicts.is_empty());
}

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
