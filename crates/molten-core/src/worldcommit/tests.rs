use super::*;

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
fn closure_reports_first_missing_root_and_parent_cycles() {
    // r[verify molten.world_commit.restore]
    // r[verify molten.world_commit.verification]
    let plan = plan_capture(&logical_request()).expect("capture plan");
    let commit_ref = identify_world_commit(b"subject").expect("subject ref");
    let parent_ref = identify_world_commit(b"parent").expect("parent ref");
    let mut request = complete_closure(commit_ref.clone(), plan.core);
    request.core.parents.push(parent_ref.clone());
    request.roots.remove(0);
    request.parent_graph.push(ParentClosureObservation {
        commit_ref: parent_ref,
        parents: vec![commit_ref],
        object_present: true,
    });

    let report = validate_closure(&request);

    assert!(!report.complete);
    assert_eq!(report.first_missing_root, Some(RootKind::Artifact));
    assert!(report.issues.contains(&ClosureIssue::MissingRootObject(RootKind::Artifact)));
    assert!(report.issues.iter().any(|issue| matches!(issue, ClosureIssue::ParentCycle(_))));
}

fn parent_observation(commit_ref: &WorldCommitRef, parents: &[&WorldCommitRef]) -> ParentClosureObservation {
    ParentClosureObservation {
        commit_ref: commit_ref.clone(),
        parents: parents.iter().map(|parent| (*parent).clone()).collect(),
        object_present: true,
    }
}

/// Subject `S` with a diamond ancestry `S -> {L, R}`, `L -> B`, `R -> B`.
fn diamond_closure() -> ClosureRequest {
    let plan = plan_capture(&logical_request()).expect("capture plan");
    let subject = identify_world_commit(b"diamond-subject").expect("subject ref");
    let left = identify_world_commit(b"diamond-left").expect("left ref");
    let right = identify_world_commit(b"diamond-right").expect("right ref");
    let base = identify_world_commit(b"diamond-base").expect("base ref");
    let mut request = complete_closure(subject, plan.core);
    request.core.parents = vec![left.clone(), right.clone()];
    request.parent_graph = vec![
        parent_observation(&left, &[&base]),
        parent_observation(&right, &[&base]),
        parent_observation(&base, &[]),
    ];
    request
}

#[test]
fn closure_accepts_valid_diamond_ancestry() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    let request = diamond_closure();
    let report = validate_closure(&request);
    assert!(report.complete, "diamond ancestry is a complete closure: {:?}", report.issues);
    assert!(report.issues.is_empty());
}

#[test]
fn closure_rejects_detached_cycle_and_unreachable_observations() {
    // r[verify molten.world_commit.merge_conservative_precedence_and_full_graph_closure]
    let mut detached = diamond_closure();
    let cycle_x = identify_world_commit(b"detached-x").expect("x ref");
    let cycle_y = identify_world_commit(b"detached-y").expect("y ref");
    detached.parent_graph.push(parent_observation(&cycle_x, &[&cycle_y]));
    detached.parent_graph.push(parent_observation(&cycle_y, &[&cycle_x]));
    let report = validate_closure(&detached);
    assert!(!report.complete);
    assert!(report.issues.iter().any(|issue| matches!(issue, ClosureIssue::ParentCycle(_))));
    for detached_ref in [&cycle_x, &cycle_y] {
        assert!(
            report
                .issues
                .contains(&ClosureIssue::UnreachableParentObservation(detached_ref.as_str().to_string()))
        );
    }
    assert!(!report.issues.iter().any(|issue| matches!(issue, ClosureIssue::MissingParentObservation(_))));

    let mut orphaned = diamond_closure();
    let orphan = identify_world_commit(b"orphan").expect("orphan ref");
    orphaned.parent_graph.push(parent_observation(&orphan, &[]));
    let report = validate_closure(&orphaned);
    assert!(!report.complete);
    assert_eq!(report.issues, vec![ClosureIssue::UnreachableParentObservation(orphan.as_str().to_string())]);
}

#[test]
fn complete_logical_closure_produces_deterministic_admission_last_restore() {
    // r[verify molten.world_commit.restore]
    let capture = plan_capture(&logical_request()).expect("capture plan");
    let commit_ref = identify_world_commit(b"logical-commit").expect("commit ref");
    let closure_request = complete_closure(commit_ref.clone(), capture.core.clone());
    let closure = validate_closure(&closure_request);
    let plan = plan_restore(&commit_ref, &capture.core, &closure).expect("restore plan");

    assert!(closure.complete);
    assert_eq!(plan.steps.first().map(|step| step.kind), Some(RestoreStepKind::VerifySchema));
    assert_eq!(plan.steps.last().map(|step| step.kind), Some(RestoreStepKind::ActivateRuntime));
    assert!(plan.current_admission_required);
    assert!(
        plan.replay.iter().any(|item| {
            item.root_kind == RootKind::AuthorityObservation && item.class == RootReplayClass::HistoricalEvidenceOnly
        }) == capture.core.roots.iter().any(|root| root.kind() == RootKind::AuthorityObservation)
    );
}

#[test]
fn opaque_profile_requires_exact_cohort_and_never_claims_logical_replay() {
    // r[verify molten.world_commit.verification]
    let request = CaptureRequest {
        version: WorldCommitVersion::V1,
        profile: opaque_profile(SnapshotProfileKind::Opaque),
        parents: Vec::new(),
        observations: observations(SnapshotProfileKind::Opaque),
        bounds: WorldCommitBounds::default(),
    };
    let capture = plan_capture(&request).expect("opaque capture plan");
    let opaque = capture
        .core
        .roots
        .iter()
        .find(|root| root.kind() == RootKind::OpaqueMachineSnapshot)
        .expect("opaque root");

    assert_eq!(replay_class(opaque.kind()), RootReplayClass::RestoreOpaqueState);
    assert!(!capture.core.roots.iter().any(|root| root.kind() == RootKind::DurableState));

    let mut missing_cohort = request;
    missing_cohort.profile.cohort_ref = None;
    assert!(
        plan_capture(&missing_cohort)
            .expect_err("missing cohort must fail")
            .contains(&CaptureIssue::OpaqueProfileMissingCohort)
    );
}

#[test]
fn bounds_fail_before_unbounded_capture_or_closure_work() {
    // r[verify molten.world_commit.verification]
    let mut request = logical_request();
    request.bounds.max_roots = TINY_BOUND;
    let issues = plan_capture(&request).expect_err("over-bound capture must fail");
    assert!(issues.iter().any(|issue| matches!(issue, CaptureIssue::BoundExceeded {
        field: "root-observations",
        ..
    })));

    request.bounds.max_roots = MAX_WORLD_COMMIT_ROOTS;
    request.bounds.max_closure_objects = 0;
    let issues = plan_capture(&request).expect_err("zero closure bound must fail");
    assert!(issues.contains(&CaptureIssue::ZeroBound("max-closure-objects")));
}

#[test]
fn evidence_and_authority_observations_do_not_enter_current_admission() {
    // r[verify molten.world_commit.detached_evidence]
    let mut request = logical_request();
    request.observations.push(RootObservation {
        root: root(RootKind::AuthorityObservation, "historical-authority"),
        source_kind: RootKind::AuthorityObservation,
        schema_validated: true,
        stability: ObservationStability::Immutable,
        durable: true,
        inventory_complete: true,
    });
    let capture = plan_capture(&request).expect("capture with authority observation");
    let commit_ref = identify_world_commit(b"authority-observation").expect("commit ref");
    let closure_request = complete_closure(commit_ref.clone(), capture.core.clone());
    let closure = validate_closure(&closure_request);
    let restore = plan_restore(&commit_ref, &capture.core, &closure).expect("restore plan");

    assert!(restore.current_admission_required);
    assert!(restore.replay.contains(&RootReplayClassification {
        root_kind: RootKind::AuthorityObservation,
        class: RootReplayClass::HistoricalEvidenceOnly,
    }));
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
