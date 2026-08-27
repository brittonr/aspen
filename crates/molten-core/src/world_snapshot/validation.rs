use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::model::*;

pub fn validate_snapshot(descriptor: &SnapshotDescriptor, destination: &SnapshotCohort) -> CompatibilityReport {
    let mut issues = Vec::new();
    validate_components(descriptor, &mut issues);
    validate_cohort(descriptor, destination, &mut issues);
    if descriptor.contains_live_handle {
        issues.push(SnapshotIssue::LiveHandleCaptured);
    }
    issues.sort();
    issues.dedup();
    CompatibilityReport {
        verdict: verdict(&issues),
        issues,
    }
}

pub fn plan_restore(
    descriptor: &SnapshotDescriptor,
    destination: &SnapshotCohort,
    current_admission: bool,
) -> Result<SnapshotRestorePlan, CompatibilityReport> {
    let mut report = validate_snapshot(descriptor, destination);
    if !current_admission {
        report.issues.push(SnapshotIssue::CurrentAdmissionDenied);
        report.verdict = CompatibilityVerdict::Unsafe;
        return Err(report);
    }
    if report.verdict != CompatibilityVerdict::Compatible {
        return Err(report);
    }
    let steps = match descriptor.class {
        SnapshotClass::Logical => vec![
            SnapshotRestoreStep::VerifyClosure,
            SnapshotRestoreStep::VerifyCohort,
            SnapshotRestoreStep::MaterializeArtifacts,
            SnapshotRestoreStep::RestoreDurableState,
            SnapshotRestoreStep::RestoreHistory,
            SnapshotRestoreStep::RestoreTasks,
            SnapshotRestoreStep::RestoreScheduler,
            SnapshotRestoreStep::RestoreTime,
            SnapshotRestoreStep::RestoreEntropy,
            SnapshotRestoreStep::RestoreEffects,
            SnapshotRestoreStep::RecreateHostHandles,
            SnapshotRestoreStep::RecheckCurrentAdmission,
            SnapshotRestoreStep::ActivateRuntime,
        ],
        SnapshotClass::Opaque => vec![
            SnapshotRestoreStep::VerifyClosure,
            SnapshotRestoreStep::VerifyCohort,
            SnapshotRestoreStep::MaterializeArtifacts,
            SnapshotRestoreStep::RestoreOpaqueMachine,
            SnapshotRestoreStep::RecreateHostHandles,
            SnapshotRestoreStep::RecheckCurrentAdmission,
            SnapshotRestoreStep::ActivateRuntime,
        ],
    };
    Ok(SnapshotRestorePlan {
        commit_ref: descriptor.commit_ref.clone(),
        class: descriptor.class,
        steps,
        activation_permitted: true,
    })
}

pub fn admit_semantic_merge(class: SnapshotClass, roots_equal: bool) -> Result<(), SnapshotIssue> {
    if class == SnapshotClass::Opaque && !roots_equal {
        return Err(SnapshotIssue::OpaqueMergeDenied);
    }
    Ok(())
}

pub fn validate_clone_plan(request: &ClonePlanRequest) -> Result<(), Vec<SnapshotIssue>> {
    let mut issues = Vec::new();
    if request.children.len() > MAX_CLONE_CHILDREN {
        issues.push(SnapshotIssue::ChildBoundExceeded);
    }
    let mut overlays = BTreeSet::new();
    for child in &request.children {
        if child.parent_ref != request.parent_ref {
            issues.push(SnapshotIssue::ParentMismatch);
        }
        let child_overlays = [
            &child.memory_overlay,
            &child.device_overlay,
            &child.disk_overlay,
            &child.endpoint_overlay,
        ];
        if child_overlays.iter().any(|identity| identity.0.is_empty()) {
            issues.push(SnapshotIssue::PartialOverlaySet);
        }
        for identity in child_overlays {
            if !overlays.insert(identity.clone()) {
                issues.push(SnapshotIssue::OverlayCollision);
            }
        }
    }
    issues.sort();
    issues.dedup();
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn validate_components(descriptor: &SnapshotDescriptor, issues: &mut Vec<SnapshotIssue>) {
    if descriptor.components.len() > MAX_SNAPSHOT_COMPONENTS {
        issues.push(SnapshotIssue::TooManyComponents);
    }
    let required = match descriptor.class {
        SnapshotClass::Logical => LOGICAL_COMPONENTS,
        SnapshotClass::Opaque => OPAQUE_COMPONENTS,
    };
    let mut observed = BTreeSet::new();
    for component in &descriptor.components {
        if !observed.insert(component.kind) {
            issues.push(SnapshotIssue::DuplicateComponent(component.kind));
        }
        if !required.contains(&component.kind) {
            issues.push(SnapshotIssue::UnexpectedComponent(component.kind));
        }
        if component.identity.is_empty() {
            issues.push(SnapshotIssue::EmptyIdentity);
        }
        let expected_owner = match (descriptor.class, component.kind) {
            (SnapshotClass::Opaque, SnapshotComponentKind::MachineDescriptor)
            | (SnapshotClass::Opaque, SnapshotComponentKind::CpuState)
            | (SnapshotClass::Opaque, SnapshotComponentKind::Memory)
            | (SnapshotClass::Opaque, SnapshotComponentKind::DeviceState)
            | (SnapshotClass::Opaque, SnapshotComponentKind::DiskState)
            | (SnapshotClass::Opaque, SnapshotComponentKind::BackendState) => ComponentOwner::ChaosControl,
            _ => ComponentOwner::Molten,
        };
        if component.owner != expected_owner {
            issues.push(SnapshotIssue::WrongOwner(component.kind));
        }
        match (component.kind.root_kind(), &component.root) {
            (Some(expected), Some(root)) if root.kind() != expected => {
                issues.push(SnapshotIssue::WrongRootKind(component.kind));
            }
            (Some(_), None) => issues.push(SnapshotIssue::MissingRoot(component.kind)),
            (None, Some(_)) => issues.push(SnapshotIssue::UnexpectedRoot(component.kind)),
            _ => {}
        }
    }
    for kind in required {
        if !observed.contains(kind) {
            issues.push(SnapshotIssue::MissingComponent(*kind));
        }
    }
}

fn validate_cohort(descriptor: &SnapshotDescriptor, destination: &SnapshotCohort, issues: &mut Vec<SnapshotIssue>) {
    if descriptor.cohort.facts.len() > MAX_COHORT_FACTS || destination.facts.len() > MAX_COHORT_FACTS {
        issues.push(SnapshotIssue::TooManyCohortFacts);
    }
    let required = match descriptor.class {
        SnapshotClass::Logical => LOGICAL_COHORT_FACTS,
        SnapshotClass::Opaque => OPAQUE_COHORT_FACTS,
    };
    let source = cohort_map(&descriptor.cohort.facts, required, issues);
    let target = cohort_map(&destination.facts, required, issues);
    for kind in required {
        match (source.get(kind), target.get(kind)) {
            (Some(left), Some(right)) if left != right => {
                issues.push(SnapshotIssue::CohortMismatch(*kind));
            }
            (Some(_), Some(_)) => {}
            _ => issues.push(SnapshotIssue::MissingCohortFact(*kind)),
        }
    }
}

fn cohort_map<'a>(
    facts: &'a [CohortFact],
    required: &[CohortFactKind],
    issues: &mut Vec<SnapshotIssue>,
) -> BTreeMap<CohortFactKind, &'a str> {
    let mut values = BTreeMap::new();
    for fact in facts {
        if fact.identity.is_empty() {
            issues.push(SnapshotIssue::EmptyIdentity);
        }
        if !required.contains(&fact.kind) {
            issues.push(SnapshotIssue::UnexpectedCohortFact(fact.kind));
        }
        if values.insert(fact.kind, fact.identity.as_str()).is_some() {
            issues.push(SnapshotIssue::DuplicateCohortFact(fact.kind));
        }
    }
    values
}

fn verdict(issues: &[SnapshotIssue]) -> CompatibilityVerdict {
    if issues.is_empty() {
        return CompatibilityVerdict::Compatible;
    }
    if issues.iter().any(|issue| matches!(issue, SnapshotIssue::LiveHandleCaptured)) {
        return CompatibilityVerdict::Unsafe;
    }
    if issues
        .iter()
        .any(|issue| matches!(issue, SnapshotIssue::MissingComponent(_) | SnapshotIssue::MissingCohortFact(_)))
    {
        return CompatibilityVerdict::Incomplete;
    }
    CompatibilityVerdict::Incompatible
}
