use super::*;

mod closure;
mod plans;

const INITIAL_REVISION: u64 = 1;
const NEXT_REVISION: u64 = 2;
const TINY_BOUND: usize = 1;
const DURABILITY_ALTERNATION_DIVISOR: usize = 2;

fn content_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn root(kind: RootKind, label: &str) -> WorldRootRef {
    WorldRootRef::parse(kind, content_ref(label)).expect("valid root reference")
}

fn logical_profile() -> SnapshotProfile {
    SnapshotProfile {
        kind: SnapshotProfileKind::Logical,
        profile_ref: SnapshotProfileRef::new(content_ref("logical-profile")).expect("profile ref"),
        cohort_ref: None,
    }
}

fn opaque_profile(kind: SnapshotProfileKind) -> SnapshotProfile {
    SnapshotProfile {
        kind,
        profile_ref: SnapshotProfileRef::new(content_ref("opaque-profile")).expect("profile ref"),
        cohort_ref: Some(SnapshotCohortRef::new(content_ref("exact-cohort")).expect("cohort ref")),
    }
}

fn observations(profile: SnapshotProfileKind) -> Vec<RootObservation> {
    profile
        .required_roots()
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let label = format!("{}-root", kind.as_str());
            let source_id = format!("{}-source", kind.as_str());
            let revision = u64::try_from(index).expect("root index fits in u64") + INITIAL_REVISION;
            RootObservation {
                root: root(*kind, &label),
                source_kind: *kind,
                schema_validated: true,
                stability: ObservationStability::Mutable(
                    RevisionFence::new(*kind, source_id, revision).expect("revision fence"),
                ),
                durable: index % DURABILITY_ALTERNATION_DIVISOR == 0,
                inventory_complete: true,
            }
        })
        .collect()
}

fn logical_request() -> CaptureRequest {
    CaptureRequest {
        version: WorldCommitVersion::V1,
        profile: logical_profile(),
        parents: Vec::new(),
        observations: observations(SnapshotProfileKind::Logical),
        bounds: WorldCommitBounds::default(),
    }
}

fn complete_closure(commit_ref: WorldCommitRef, core: WorldCommitCore) -> ClosureRequest {
    let roots = core
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
    ClosureRequest {
        commit_ref,
        core,
        roots,
        parent_graph: Vec::new(),
        bounds: WorldCommitBounds::default(),
    }
}

#[test]
fn equivalent_capture_input_orders_normalize_to_one_core() {
    // r[verify molten.world_commit.core]
    let first = logical_request();
    let mut second = first.clone();
    second.observations.reverse();

    let first_plan = plan_capture(&first).expect("first capture plan");
    let second_plan = plan_capture(&second).expect("second capture plan");

    assert_eq!(first_plan.core, second_plan.core);
    assert_eq!(first_plan.roots_to_persist, second_plan.roots_to_persist);
    assert_eq!(first_plan.revision_fences, second_plan.revision_fences);
}

#[test]
fn world_commit_identity_is_domain_separated_and_input_sensitive() {
    // r[verify molten.world_commit.core]
    let first = identify_world_commit(b"canonical-preserves-world-a").expect("first identity");
    let repeated = identify_world_commit(b"canonical-preserves-world-a").expect("repeated identity");
    let changed = identify_world_commit(b"canonical-preserves-world-b").expect("changed identity");
    let raw = format!("blake3:{}", blake3::hash(b"canonical-preserves-world-a").to_hex());

    assert_eq!(first, repeated);
    assert_ne!(first, changed);
    assert_ne!(first.as_str(), raw);
}

#[test]
fn capture_rejects_domain_confusion_incomplete_inventory_and_duplicates() {
    // r[verify molten.world_commit.typed_roots]
    // r[verify molten.world_commit.verification]
    let mut request = logical_request();
    let parent = WorldCommitRef::new(content_ref("duplicate-parent")).expect("parent ref");
    request.parents = vec![parent.clone(), parent.clone()];
    request.observations[0].source_kind = RootKind::Tasks;
    request.observations[1].inventory_complete = false;
    request.observations[2].schema_validated = false;
    request.observations.push(request.observations[3].clone());

    let issues = plan_capture(&request).expect_err("invalid capture must fail");

    assert!(issues.contains(&CaptureIssue::DuplicateParent(parent.as_str().to_string())));
    assert!(issues.contains(&CaptureIssue::RootDomainMismatch {
        expected: RootKind::Artifact,
        actual: RootKind::Tasks,
    }));
    assert!(issues.contains(&CaptureIssue::IncompleteInventory(RootKind::Schema)));
    assert!(issues.contains(&CaptureIssue::RootSchemaNotValidated(RootKind::DurableState)));
    assert!(issues.contains(&CaptureIssue::DuplicateRootKind(RootKind::Tasks)));
}

#[test]
fn capture_recheck_detects_revision_drift_and_missing_inventory() {
    // r[verify molten.world_commit.capture]
    let plan = plan_capture(&logical_request()).expect("capture plan");
    let mut rechecks = plan
        .revision_fences
        .iter()
        .map(|fence| RevisionRecheck {
            root_kind: fence.root_kind,
            source_id: fence.source_id.clone(),
            current_revision: fence.observed_revision,
            inventory_complete: true,
        })
        .collect::<Vec<_>>();
    let current = compare_revision_rechecks(&plan, &rechecks);
    assert!(current.current);

    rechecks[0].current_revision = rechecks[0].current_revision.saturating_add(NEXT_REVISION);
    rechecks[1].inventory_complete = false;
    let drifted = compare_revision_rechecks(&plan, &rechecks);

    assert!(!drifted.current);
    assert!(drifted.issues.iter().any(|issue| matches!(issue, CaptureIssue::RevisionDrift { .. })));
    assert!(drifted.issues.iter().any(|issue| matches!(issue, CaptureIssue::RecheckInventoryIncomplete(_))));
}

#[test]
fn closure_and_capture_issue_codes_are_closed_typed_tags() {
    // r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
    let mut request = logical_request();
    request.bounds.max_closure_objects = 0;
    let capture = plan_capture(&logical_request()).expect("capture");
    let commit_ref = identify_world_commit(b"issue-code-closure").expect("commit ref");
    let mut closure_request = complete_closure(commit_ref, capture.core);
    closure_request.bounds = request.bounds;
    let closure = validate_closure(&closure_request);
    assert_eq!(closure.issues, vec![ClosureIssue::InvalidCore(CaptureIssue::ZeroBound(
        "max-closure-objects"
    ))]);
    assert_eq!(closure.issues.iter().map(ClosureIssue::code).collect::<Vec<_>>(), vec![
        "invalid-core:zero-bound:max-closure-objects".to_string()
    ]);

    let golden = [
        (
            CaptureIssue::BoundExceeded {
                field: "root-observations",
                actual: 3,
                maximum: 2,
            }
            .code(),
            "bound-exceeded:root-observations:3:2",
        ),
        (CaptureIssue::MissingRequiredRoot(RootKind::Schema).code(), "missing-required-root-kind:schema"),
        (
            CaptureIssue::RevisionDrift {
                source_id: "store:a".to_string(),
                expected: 4,
                actual: 5,
            }
            .code(),
            "revision-drift:4:5:store:a",
        ),
        (ClosureIssue::MissingRootObject(RootKind::Policy).code(), "missing-root-object:policy"),
    ];
    for (code, expected) in golden {
        assert_eq!(code, expected);
    }

    let split_left = ClosureIssue::DuplicateParentEdge {
        commit_ref: "a:b".to_string(),
        parent_ref: "c".to_string(),
    };
    let split_right = ClosureIssue::DuplicateParentEdge {
        commit_ref: "a".to_string(),
        parent_ref: "b:c".to_string(),
    };
    assert_ne!(split_left.code(), split_right.code());
    let drift_left = CaptureIssue::RevisionDrift {
        source_id: "1:2".to_string(),
        expected: 3,
        actual: 4,
    };
    let drift_right = CaptureIssue::RevisionDrift {
        source_id: "2".to_string(),
        expected: 3,
        actual: 4,
    };
    assert_ne!(drift_left.code(), drift_right.code());
}
