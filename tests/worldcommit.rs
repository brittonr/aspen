#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "the integration fixture composes published world-commit ports and DTOs explicitly"
)]
#![allow(
    tigerstyle::no_unwrap,
    reason = "reviewed fixture constants fail only when the harness is broken"
)]
use std::collections::BTreeMap;

use molten::world_commit::*;
use molten_core::world_commit::*;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
const INITIAL_REVISION: u64 = 1;
const DRIFTED_REVISION: u64 = 2;
#[derive(Debug, Default)]
struct Roots(BTreeMap<RootKind, ObservedRootMaterial>);
impl WorldRootObservationPort for Roots {
    fn observe_root(&mut self, kind: RootKind) -> std::result::Result<ObservedRootMaterial, WorldCommitPortError> {
        self.0
            .get(&kind)
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("missing-recorded-root", kind.as_str()))
    }
}
#[derive(Debug, Default)]
struct Store {
    roots: BTreeMap<String, Vec<u8>>,
    commits: BTreeMap<String, Vec<u8>>,
    mutations: Vec<String>,
    outcome: Option<PublicationOutcome>,
}
impl WorldImmutableObjectPort for Store {
    fn contains_root(&self, root: &WorldRootRef) -> std::result::Result<bool, WorldCommitPortError> {
        Ok(self.roots.contains_key(root.as_str()))
    }
    fn persist_root(&mut self, root: &WorldRootRef, bytes: &[u8]) -> std::result::Result<(), WorldCommitPortError> {
        self.roots.insert(root.as_str().to_string(), bytes.to_vec());
        self.mutations.push(format!("root:{}", root.kind().as_str()));
        Ok(())
    }
    fn read_root(&self, root: &WorldRootRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.roots
            .get(root.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("missing-root", root.kind().as_str()))
    }
}
impl WorldCommitPublicationPort for Store {
    fn publish_commit(
        &mut self,
        commit_ref: &WorldCommitRef,
        bytes: &[u8],
    ) -> std::result::Result<PublicationOutcome, WorldCommitPortError> {
        let outcome = self.outcome.unwrap_or(PublicationOutcome::Published);
        if outcome.is_success() {
            self.commits.insert(commit_ref.as_str().to_string(), bytes.to_vec());
            self.mutations.push("commit".to_string());
        }
        Ok(outcome)
    }

    fn read_commit(&self, commit_ref: &WorldCommitRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.commits
            .get(commit_ref.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("missing-commit", commit_ref.as_str()))
    }
}
#[derive(Debug, Default)]
struct Revisions(BTreeMap<String, RevisionRecheck>);
impl WorldRevisionRecheckPort for Revisions {
    fn recheck_revision(
        &mut self,
        fence: &RevisionFence,
    ) -> std::result::Result<RevisionRecheck, WorldCommitPortError> {
        self.0
            .get(&fence.source_id)
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("missing-recheck", &fence.source_id))
    }
}
#[derive(Debug, Default)]
struct Restore(Vec<RestoreStep>);
impl WorldRestorePort for Restore {
    fn execute_restore_step(
        &mut self,
        step: &RestoreStep,
    ) -> std::result::Result<RestoreStepOutcome, WorldCommitPortError> {
        self.0.push(step.clone());
        Ok(RestoreStepOutcome {
            step: step.clone(),
            evidence_ref: fixture_ref(step.kind.as_str()),
        })
    }
}
fn fixture_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
fn bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: MAX_WORLD_COMMIT_PARENTS,
        max_roots: MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}
#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "closed profile limits the loop to MAX_WORLD_COMMIT_ROOTS"
)]
fn inputs() -> (CaptureShellInput, Roots, Revisions) {
    let profile = SnapshotProfile {
        kind: SnapshotProfileKind::Logical,
        profile_ref: SnapshotProfileRef::new(fixture_ref("profile")).expect("profile ref"),
        cohort_ref: None,
    };
    let root_kinds = profile.kind.required_roots().to_vec();
    let mut roots = BTreeMap::new();
    let mut revisions = BTreeMap::new();
    for kind in &root_kinds {
        let value = molten::preserves_rail::record("root-v1", vec![molten::preserves_rail::string(kind.as_str())]);
        let bytes = molten::preserves_rail::canonical_bytes(&value).expect("root bytes");
        let root =
            WorldRootRef::parse(*kind, molten::preserves_rail::content_ref_from_bytes(&bytes)).expect("root ref");
        let fence = RevisionFence::new(*kind, format!("{}-source", kind.as_str()), INITIAL_REVISION).expect("fence");
        revisions.insert(fence.source_id.clone(), RevisionRecheck {
            root_kind: *kind,
            source_id: fence.source_id.clone(),
            current_revision: fence.observed_revision,
            inventory_complete: true,
        });
        roots.insert(*kind, ObservedRootMaterial {
            observation: RootObservation {
                root,
                source_kind: *kind,
                schema_validated: true,
                stability: ObservationStability::Mutable(fence),
                durable: false,
                inventory_complete: true,
            },
            canonical_bytes: bytes,
        });
    }
    (
        CaptureShellInput {
            version: WorldCommitVersion::V1,
            profile,
            parents: Vec::new(),
            root_kinds,
            bounds: bounds(),
        },
        Roots(roots),
        Revisions(revisions),
    )
}
#[test]
fn capture_persists_roots_rechecks_and_publishes_last() {
    // r[verify molten.world_commit.capture]
    let (input, mut roots, mut revisions) = inputs();
    let mut store = Store::default();
    let execution = capture_world_commit(&input, &mut roots, &mut store, &mut revisions).expect("capture");
    assert_eq!(execution.receipt.receipt.decision, CaptureDecision::Published);
    assert_eq!(store.mutations.last().map(String::as_str), Some("commit"));
    assert_eq!(store.roots.len(), input.root_kinds.len());
    assert!(execution.commit.is_some());
}
#[test]
fn capture_denies_drift_and_uncertain_publication() {
    // r[verify molten.world_commit.verification]
    let (input, mut roots, mut revisions) = inputs();
    revisions.0.values_mut().next().expect("revision").current_revision = DRIFTED_REVISION;
    let mut store = Store::default();
    let drifted = capture_world_commit(&input, &mut roots, &mut store, &mut revisions).expect("drift receipt");
    assert_eq!(drifted.receipt.receipt.decision, CaptureDecision::Denied);
    assert!(drifted.receipt.receipt.commit_ref.is_none());
    let (input, mut roots, mut revisions) = inputs();
    let mut store = Store {
        outcome: Some(PublicationOutcome::Uncertain),
        ..Store::default()
    };
    let uncertain = capture_world_commit(&input, &mut roots, &mut store, &mut revisions).expect("uncertain receipt");
    assert_eq!(uncertain.receipt.receipt.decision, CaptureDecision::Denied);
    assert!(uncertain.receipt.receipt.commit_ref.is_none());

    let (input, mut roots, mut revisions) = inputs();
    let root = roots.0.values().next().expect("first root").observation.root.clone();
    let mut store = Store::default();
    store.roots.insert(root.as_str().to_string(), b"corrupt-preexisting-root".to_vec());
    let corrupt = capture_world_commit(&input, &mut roots, &mut store, &mut revisions).expect("corrupt receipt");
    assert_eq!(corrupt.receipt.receipt.decision, CaptureDecision::Denied);
    assert!(!store.mutations.as_slice().iter().any(|mutation| mutation == "commit"));
}
#[test]
fn closure_accepts_shared_ancestor_and_requires_transitive_parent_objects() {
    // r[verify molten.world_commit.restore]
    let (input, roots, _) = inputs();
    let request = CaptureRequest {
        version: input.version,
        profile: input.profile,
        parents: Vec::new(),
        observations: roots.0.into_values().map(|material| material.observation).collect(),
        bounds: bounds(),
    };
    let mut core = plan_capture(&request).expect("capture plan").core;
    let subject = WorldCommitRef::new(fixture_ref("subject")).expect("subject");
    let left = WorldCommitRef::new(fixture_ref("left")).expect("left");
    let right = WorldCommitRef::new(fixture_ref("right")).expect("right");
    let shared = WorldCommitRef::new(fixture_ref("shared")).expect("shared");
    core.parents = vec![left.clone(), right.clone()];
    let root_observations = core
        .roots
        .iter()
        .cloned()
        .map(|root| RootClosureObservation {
            root,
            object_present: true,
            identity_matches: true,
            schema_matches: true,
        })
        .collect();
    let graph = vec![
        ParentClosureObservation {
            commit_ref: left,
            parents: vec![shared.clone()],
            object_present: true,
        },
        ParentClosureObservation {
            commit_ref: right,
            parents: vec![shared.clone()],
            object_present: true,
        },
        ParentClosureObservation {
            commit_ref: shared.clone(),
            parents: Vec::new(),
            object_present: true,
        },
    ];
    let mut closure_request = ClosureRequest {
        commit_ref: subject,
        core,
        roots: root_observations,
        parent_graph: graph,
        bounds: bounds(),
    };
    assert!(validate_closure(&closure_request).complete);
    closure_request.parent_graph.retain(|observation| observation.commit_ref != shared);
    assert!(
        validate_closure(&closure_request)
            .issues
            .iter()
            .any(|issue| matches!(issue, ClosureIssue::MissingParentObservation(_)))
    );
}
#[test]
fn local_store_closure_and_restore_roundtrip() {
    // r[verify molten.world_commit.restore]
    let temporary = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary root");
    let storage =
        NodeStateNamespace::from_dir(NodeStateNamespaceKind::Storage, temporary.try_clone().expect("clone root"))
            .expect("storage");
    let mut store = LocalWorldCommitStore::open(&storage).expect("store");
    let (input, mut roots, mut revisions) = inputs();
    let execution = capture_world_commit(&input, &mut roots, &mut store, &mut revisions).expect("capture");
    let commit = execution.commit.expect("commit");
    store.write_capture_receipt(&execution.receipt).expect("receipt");

    let observed = commit
        .core
        .roots
        .iter()
        .cloned()
        .map(|root| RootClosureObservation {
            root,
            object_present: true,
            identity_matches: true,
            schema_matches: true,
        })
        .collect();
    let closure = validate_closure(&ClosureRequest {
        commit_ref: commit.commit_ref.clone(),
        core: commit.core.clone(),
        roots: observed,
        parent_graph: Vec::new(),
        bounds: bounds(),
    });
    let plan = plan_restore(&commit.commit_ref, &commit.core, &closure).expect("plan");
    let wrong_commit = WorldCommitRef::new(fixture_ref("wrong-commit")).expect("wrong commit ref");
    assert_eq!(plan_restore(&wrong_commit, &commit.core, &closure), Err(RestoreIssue::ClosureCommitMismatch));
    let mut restore = Restore::default();
    let restored = execute_restore_plan(&plan, &mut restore).expect("restore");

    assert!(closure.complete);
    assert_eq!(restore.0, plan.steps);
    assert_eq!(restored.evidence_refs.len(), plan.steps.len());
    assert_eq!(store.read_commit(&commit.commit_ref).expect("read commit"), commit.bytes);
}
