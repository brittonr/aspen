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
    if descriptor.synchronization.is_some() {
        issues.push(SnapshotIssue::UnexpectedSynchronization);
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
    if request.children.is_empty() {
        issues.push(SnapshotIssue::EmptyClonePlan);
    }
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
        if child_overlays
            .iter()
            .any(|identity| identity.0.len() > MAX_OVERLAY_IDENTITY_BYTES || identity.0.chars().any(char::is_control))
        {
            issues.push(SnapshotIssue::InvalidOverlayIdentity);
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
        } else if !valid_content_identity(&component.identity) {
            issues.push(SnapshotIssue::InvalidContentIdentity);
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
    if descriptor.cohort.cohort_ref != destination.cohort_ref {
        issues.push(SnapshotIssue::CohortIdentityMismatch);
    }
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
        } else if !valid_content_identity(&fact.identity) {
            issues.push(SnapshotIssue::InvalidContentIdentity);
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

pub fn inventory_for(descriptor: &SnapshotDescriptor) -> SnapshotInventory {
    SnapshotInventory {
        class: descriptor.class,
        required: match descriptor.class {
            SnapshotClass::Logical => LOGICAL_COMPONENTS.to_vec(),
            SnapshotClass::Opaque => OPAQUE_COMPONENTS.to_vec(),
        },
        observed: descriptor.components.clone(),
    }
}

pub fn ownership_for(descriptor: &SnapshotDescriptor) -> Vec<SnapshotOwnership> {
    let mut ownership = descriptor
        .components
        .iter()
        .map(|component| SnapshotOwnership {
            component: component.kind,
            owner: component.owner,
        })
        .collect::<Vec<_>>();
    ownership.sort_by_key(|fact| fact.component);
    ownership.dedup_by_key(|fact| fact.component);
    ownership
}

pub fn validate_snapshot_receipt(receipt: &SnapshotReceipt) -> Result<(), Vec<SnapshotIssue>> {
    let mut issues = Vec::new();
    let refs = [
        Some(receipt.descriptor_ref.as_str()),
        Some(receipt.compatibility_ref.as_str()),
        receipt.restore_plan_ref.as_deref(),
        receipt.clone_plan_ref.as_deref(),
        receipt.current_admission_ref.as_deref(),
    ];
    if refs.into_iter().flatten().any(|reference| !valid_content_identity(reference)) {
        issues.push(SnapshotIssue::InvalidContentIdentity);
    }
    if receipt.issues.len() > MAX_SNAPSHOT_RECEIPT_ISSUES
        || receipt
            .issues
            .iter()
            .any(|issue| issue.len() > MAX_SNAPSHOT_ISSUE_BYTES || issue.chars().any(char::is_control))
    {
        issues.push(SnapshotIssue::ReceiptBoundExceeded);
    }
    let expected_non_claims = SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        issues.push(SnapshotIssue::ReceiptNonClaimsIncomplete);
    }
    if receipt.decision == SnapshotReceiptDecision::Denied && receipt.issues.is_empty() {
        issues.push(SnapshotIssue::ReceiptBoundExceeded);
    }
    if receipt.decision != SnapshotReceiptDecision::Denied && !receipt.issues.is_empty() {
        issues.push(SnapshotIssue::ReceiptBoundExceeded);
    }
    issues.sort();
    issues.dedup();
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn valid_content_identity(value: &str) -> bool {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
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
