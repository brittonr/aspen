use super::*;
use crate::world_commit::SnapshotCohortRef;
use crate::world_commit::SnapshotProfileRef;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

const DIGEST_HEX_LENGTH: usize = 64;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn root(kind: SnapshotComponentKind, byte: char) -> Option<WorldRootRef> {
    kind.root_kind().map(|root_kind| WorldRootRef::parse(root_kind, digest(byte)).expect("valid root"))
}

fn component(kind: SnapshotComponentKind, owner: ComponentOwner, byte: char) -> SnapshotComponent {
    SnapshotComponent {
        kind,
        identity: digest(byte),
        root: root(kind, byte),
        owner,
    }
}

fn cohort(class: SnapshotClass, byte: char) -> SnapshotCohort {
    let required = match class {
        SnapshotClass::Logical => LOGICAL_COHORT_FACTS,
        SnapshotClass::Opaque => OPAQUE_COHORT_FACTS,
    };
    SnapshotCohort {
        cohort_ref: SnapshotCohortRef::new(digest(byte)).expect("valid cohort"),
        facts: required
            .iter()
            .map(|kind| CohortFact {
                kind: *kind,
                identity: digest(byte),
            })
            .collect(),
    }
}

fn descriptor(class: SnapshotClass) -> SnapshotDescriptor {
    let required = match class {
        SnapshotClass::Logical => LOGICAL_COMPONENTS,
        SnapshotClass::Opaque => OPAQUE_COMPONENTS,
    };
    SnapshotDescriptor {
        class,
        commit_ref: WorldCommitRef::new(digest('a')).expect("valid commit"),
        profile_ref: SnapshotProfileRef::new(digest('b')).expect("valid profile"),
        cohort: cohort(class, 'c'),
        components: required
            .iter()
            .map(|kind| {
                let owner = if class == SnapshotClass::Opaque
                    && matches!(
                        kind,
                        SnapshotComponentKind::MachineDescriptor
                            | SnapshotComponentKind::CpuState
                            | SnapshotComponentKind::Memory
                            | SnapshotComponentKind::DeviceState
                            | SnapshotComponentKind::DiskState
                            | SnapshotComponentKind::BackendState
                    ) {
                    ComponentOwner::ChaosControl
                } else {
                    ComponentOwner::Molten
                };
                component(*kind, owner, 'd')
            })
            .collect(),
        contains_live_handle: false,
        synchronized_representations: false,
    }
}

#[test]
fn complete_logical_snapshot_has_closed_restore_order() {
    let snapshot = descriptor(SnapshotClass::Logical);
    let plan = plan_restore(&snapshot, &snapshot.cohort, true).expect("logical restore plan");
    assert_eq!(plan.class, SnapshotClass::Logical);
    assert_eq!(plan.steps.first(), Some(&SnapshotRestoreStep::VerifyClosure));
    assert_eq!(plan.steps.last(), Some(&SnapshotRestoreStep::ActivateRuntime));
    assert!(plan.activation_permitted);
}

#[test]
fn missing_task_and_current_authority_fail_closed() {
    let mut snapshot = descriptor(SnapshotClass::Logical);
    snapshot.components.retain(|component| component.kind != SnapshotComponentKind::Tasks);
    let report = validate_snapshot(&snapshot, &snapshot.cohort);
    assert_eq!(report.verdict, CompatibilityVerdict::Incomplete);
    assert!(report.issues.contains(&SnapshotIssue::MissingComponent(SnapshotComponentKind::Tasks)));

    let complete = descriptor(SnapshotClass::Logical);
    let denial = plan_restore(&complete, &complete.cohort, false).expect_err("authority denial");
    assert!(denial.issues.contains(&SnapshotIssue::CurrentAdmissionDenied));
}

#[test]
fn opaque_snapshot_requires_exact_cohort_and_rejects_live_handles() {
    let mut snapshot = descriptor(SnapshotClass::Opaque);
    let mut target = snapshot.cohort.clone();
    let architecture = target
        .facts
        .iter_mut()
        .find(|fact| fact.kind == CohortFactKind::Architecture)
        .expect("architecture fact");
    architecture.identity = digest('e');
    let report = validate_snapshot(&snapshot, &target);
    assert_eq!(report.verdict, CompatibilityVerdict::Incompatible);
    assert!(report.issues.contains(&SnapshotIssue::CohortMismatch(CohortFactKind::Architecture)));

    snapshot.contains_live_handle = true;
    let unsafe_report = validate_snapshot(&snapshot, &snapshot.cohort);
    assert_eq!(unsafe_report.verdict, CompatibilityVerdict::Unsafe);
}

#[test]
fn divergent_opaque_roots_never_enter_semantic_merge() {
    assert_eq!(admit_semantic_merge(SnapshotClass::Opaque, false), Err(SnapshotIssue::OpaqueMergeDenied));
    assert_eq!(admit_semantic_merge(SnapshotClass::Opaque, true), Ok(()));
    assert_eq!(admit_semantic_merge(SnapshotClass::Logical, false), Ok(()));
}

#[test]
fn clone_plans_bind_parent_and_isolate_every_overlay() {
    let parent = WorldCommitRef::new(digest('f')).expect("valid parent");
    let child = CloneChild {
        parent_ref: parent.clone(),
        memory_overlay: OverlayIdentity("memory-a".into()),
        device_overlay: OverlayIdentity("device-a".into()),
        disk_overlay: OverlayIdentity("disk-a".into()),
        endpoint_overlay: OverlayIdentity("endpoint-a".into()),
    };
    let valid = ClonePlanRequest {
        parent_ref: parent.clone(),
        children: vec![child.clone()],
    };
    assert_eq!(validate_clone_plan(&valid), Ok(()));

    let conflicting = ClonePlanRequest {
        parent_ref: parent,
        children: vec![child.clone(), child],
    };
    assert!(
        validate_clone_plan(&conflicting)
            .expect_err("overlay collision")
            .contains(&SnapshotIssue::OverlayCollision)
    );
}
