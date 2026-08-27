use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const SNAPSHOT_DESCRIPTOR_RECORD: &str = "molten-world-snapshot-descriptor-v1";
pub const SNAPSHOT_INVENTORY_RECORD: &str = "molten-world-snapshot-inventory-v1";
pub const SNAPSHOT_COMPATIBILITY_RECORD: &str = "molten-world-snapshot-compatibility-v1";
pub const SNAPSHOT_RESTORE_PLAN_RECORD: &str = "molten-world-snapshot-restore-plan-v1";
pub const SNAPSHOT_CLONE_PLAN_RECORD: &str = "molten-world-snapshot-clone-plan-v1";
pub const SNAPSHOT_RECEIPT_RECORD: &str = "molten-world-snapshot-receipt-v1";

#[derive(Debug, Clone)]
pub struct CanonicalSnapshotArtifact {
    pub artifact_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_snapshot_descriptor(descriptor: &SnapshotDescriptor) -> Result<CanonicalSnapshotArtifact> {
    let report = validate_snapshot(descriptor, &descriptor.cohort);
    if report.verdict != CompatibilityVerdict::Compatible {
        return Err(MoltenError::invalid_harness(format!("snapshot descriptor denied: {:?}", report.issues)));
    }
    let mut facts = descriptor.cohort.facts.clone();
    facts.sort_by_key(|fact| fact.kind);
    let mut components = descriptor.components.clone();
    components.sort_by_key(|component| component.kind);
    let value = crate::preserves_rail::record(SNAPSHOT_DESCRIPTOR_RECORD, vec![
        crate::preserves_rail::string(SNAPSHOT_DESCRIPTOR_SCHEMA),
        field("class", string(descriptor.class.as_str())),
        field("commit-ref", string(descriptor.commit_ref.as_str())),
        field("profile-ref", string(descriptor.profile_ref.as_str())),
        field("cohort-ref", string(descriptor.cohort.cohort_ref.as_str())),
        field(
            "cohort-facts",
            sequence(
                facts
                    .iter()
                    .map(|fact| record("cohort-fact", vec![string(fact.kind.as_str()), string(&fact.identity)]))
                    .collect(),
            ),
        ),
        field("components", sequence(components.iter().map(component_value).collect())),
        field("contains-live-handle", boolean(descriptor.contains_live_handle)),
        field(
            "synchronization",
            descriptor.synchronization.as_ref().map_or_else(
                || record("none", Vec::new()),
                |fact| {
                    record("some", vec![record("synchronization", vec![
                        string(fact.logical_commit_ref.as_str()),
                        string(fact.opaque_snapshot_ref.as_str()),
                        string(&fact.observation_ref),
                    ])])
                },
            ),
        ),
    ]);
    canonical(SnapshotIdentityKind::Descriptor, value)
}

pub fn canonical_snapshot_inventory(inventory: &SnapshotInventory) -> Result<CanonicalSnapshotArtifact> {
    if inventory.required.len() > MAX_SNAPSHOT_COMPONENTS || inventory.observed.len() > MAX_SNAPSHOT_COMPONENTS {
        return Err(MoltenError::invalid_harness("snapshot inventory exceeds component bound"));
    }
    let mut required = inventory.required.clone();
    required.sort();
    required.dedup();
    let mut observed = inventory.observed.clone();
    observed.sort_by_key(|component| component.kind);
    let value = record(SNAPSHOT_INVENTORY_RECORD, vec![
        string(SNAPSHOT_INVENTORY_SCHEMA),
        field("class", string(inventory.class.as_str())),
        field("required", sequence(required.iter().map(|kind| string(kind.as_str())).collect())),
        field("observed", sequence(observed.iter().map(component_value).collect())),
    ]);
    canonical(SnapshotIdentityKind::Inventory, value)
}

pub fn canonical_snapshot_compatibility(report: &CompatibilityReport) -> Result<CanonicalSnapshotArtifact> {
    if report.issues.len() > MAX_SNAPSHOT_COMPONENTS + MAX_COHORT_FACTS {
        return Err(MoltenError::invalid_harness("snapshot compatibility issue count exceeds bound"));
    }
    let mut issues = report.issues.clone();
    issues.sort();
    issues.dedup();
    let value = record(SNAPSHOT_COMPATIBILITY_RECORD, vec![
        string(SNAPSHOT_COMPATIBILITY_SCHEMA),
        field("verdict", string(report.verdict.as_str())),
        field("issues", sequence(issues.iter().map(issue_value).collect())),
        non_claims(),
    ]);
    canonical(SnapshotIdentityKind::Compatibility, value)
}

pub fn canonical_snapshot_restore_plan(plan: &SnapshotRestorePlan) -> Result<CanonicalSnapshotArtifact> {
    if plan.steps.is_empty() || plan.steps.len() > MAX_SNAPSHOT_COMPONENTS {
        return Err(MoltenError::invalid_harness("snapshot restore step count is invalid"));
    }
    let value = record(SNAPSHOT_RESTORE_PLAN_RECORD, vec![
        string(SNAPSHOT_RESTORE_PLAN_SCHEMA),
        field("commit-ref", string(plan.commit_ref.as_str())),
        field("class", string(plan.class.as_str())),
        field("steps", sequence(plan.steps.iter().map(|step| string(step.as_str())).collect())),
        field("activation-permitted", boolean(plan.activation_permitted)),
        non_claims(),
    ]);
    canonical(SnapshotIdentityKind::RestorePlan, value)
}

pub fn canonical_snapshot_clone_plan(plan: &ClonePlanRequest) -> Result<CanonicalSnapshotArtifact> {
    validate_clone_plan(plan)
        .map_err(|issues| MoltenError::invalid_harness(format!("snapshot clone plan denied: {issues:?}")))?;
    let mut children = plan.children.clone();
    children.sort_by(|left, right| {
        (&left.memory_overlay, &left.device_overlay, &left.disk_overlay, &left.endpoint_overlay).cmp(&(
            &right.memory_overlay,
            &right.device_overlay,
            &right.disk_overlay,
            &right.endpoint_overlay,
        ))
    });
    let value = record(SNAPSHOT_CLONE_PLAN_RECORD, vec![
        string(SNAPSHOT_CLONE_PLAN_SCHEMA),
        field("parent-ref", string(plan.parent_ref.as_str())),
        field(
            "children",
            sequence(
                children
                    .iter()
                    .map(|child| {
                        record("clone-child", vec![
                            string(child.parent_ref.as_str()),
                            string(&child.memory_overlay.0),
                            string(&child.device_overlay.0),
                            string(&child.disk_overlay.0),
                            string(&child.endpoint_overlay.0),
                        ])
                    })
                    .collect(),
            ),
        ),
        non_claims(),
    ]);
    canonical(SnapshotIdentityKind::ClonePlan, value)
}

pub fn canonical_snapshot_receipt(receipt: &SnapshotReceipt) -> Result<CanonicalSnapshotArtifact> {
    validate_snapshot_receipt(receipt)
        .map_err(|issues| MoltenError::invalid_harness(format!("snapshot receipt denied: {issues:?}")))?;
    let value = record(SNAPSHOT_RECEIPT_RECORD, vec![
        string(SNAPSHOT_RECEIPT_SCHEMA),
        field("decision", string(receipt.decision.as_str())),
        field("descriptor-ref", string(&receipt.descriptor_ref)),
        field("compatibility-ref", string(&receipt.compatibility_ref)),
        field("restore-plan-ref", optional_ref(receipt.restore_plan_ref.as_deref())),
        field("clone-plan-ref", optional_ref(receipt.clone_plan_ref.as_deref())),
        field("current-admission-ref", optional_ref(receipt.current_admission_ref.as_deref())),
        field("issues", sequence(receipt.issues.iter().map(string).collect())),
        non_claims(),
    ]);
    canonical(SnapshotIdentityKind::Receipt, value)
}

fn canonical(kind: SnapshotIdentityKind, value: IOValue) -> Result<CanonicalSnapshotArtifact> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let artifact_ref = identify_snapshot_artifact(kind, &bytes)
        .map_err(|issue| MoltenError::invalid_harness(format!("snapshot identity denied: {issue:?}")))?;
    Ok(CanonicalSnapshotArtifact {
        artifact_ref,
        value,
        bytes,
    })
}

fn component_value(component: &SnapshotComponent) -> IOValue {
    record("snapshot-component", vec![
        string(component.kind.as_str()),
        string(&component.identity),
        component
            .root
            .as_ref()
            .map_or_else(|| record("none", Vec::new()), |root| record("some", vec![root_value(root)])),
        string(component.owner.as_str()),
    ])
}

fn root_value(root: &WorldRootRef) -> IOValue {
    record("typed-root", vec![string(root.kind().as_str()), string(root.as_str())])
}

fn issue_value(issue: &SnapshotIssue) -> IOValue {
    let (code, detail) = match issue {
        SnapshotIssue::DuplicateComponent(kind) => ("duplicate-component", Some(kind.as_str())),
        SnapshotIssue::MissingComponent(kind) => ("missing-component", Some(kind.as_str())),
        SnapshotIssue::UnexpectedComponent(kind) => ("unexpected-component", Some(kind.as_str())),
        SnapshotIssue::DuplicateCohortFact(kind) => ("duplicate-cohort-fact", Some(kind.as_str())),
        SnapshotIssue::MissingCohortFact(kind) => ("missing-cohort-fact", Some(kind.as_str())),
        SnapshotIssue::UnexpectedCohortFact(kind) => ("unexpected-cohort-fact", Some(kind.as_str())),
        SnapshotIssue::WrongOwner(kind) => ("wrong-owner", Some(kind.as_str())),
        SnapshotIssue::MissingRoot(kind) => ("missing-root", Some(kind.as_str())),
        SnapshotIssue::WrongRootKind(kind) => ("wrong-root-kind", Some(kind.as_str())),
        SnapshotIssue::UnexpectedRoot(kind) => ("unexpected-root", Some(kind.as_str())),
        SnapshotIssue::CohortMismatch(kind) => ("cohort-mismatch", Some(kind.as_str())),
        issue => (simple_issue_code(issue), None),
    };
    record("snapshot-issue", vec![
        string(code),
        detail.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)])),
    ])
}

fn simple_issue_code(issue: &SnapshotIssue) -> &'static str {
    match issue {
        SnapshotIssue::UnsupportedProfile => "unsupported-profile",
        SnapshotIssue::TooManyComponents => "too-many-components",
        SnapshotIssue::TooManyCohortFacts => "too-many-cohort-facts",
        SnapshotIssue::EmptyIdentity => "empty-identity",
        SnapshotIssue::LiveHandleCaptured => "live-handle-captured",
        SnapshotIssue::UnexpectedSynchronization => "unexpected-synchronization",
        SnapshotIssue::InvalidContentIdentity => "invalid-content-identity",
        SnapshotIssue::InvalidOverlayIdentity => "invalid-overlay-identity",
        SnapshotIssue::ReceiptBoundExceeded => "receipt-bound-exceeded",
        SnapshotIssue::ReceiptNonClaimsIncomplete => "receipt-non-claims-incomplete",
        SnapshotIssue::CohortIdentityMismatch => "cohort-identity-mismatch",
        SnapshotIssue::OpaqueMergeDenied => "opaque-merge-denied",
        SnapshotIssue::CurrentAdmissionDenied => "current-admission-denied",
        SnapshotIssue::EmptyClonePlan => "empty-clone-plan",
        SnapshotIssue::ChildBoundExceeded => "child-bound-exceeded",
        SnapshotIssue::ParentMismatch => "parent-mismatch",
        SnapshotIssue::OverlayCollision => "overlay-collision",
        SnapshotIssue::PartialOverlaySet => "partial-overlay-set",
        SnapshotIssue::DuplicateComponent(_)
        | SnapshotIssue::MissingComponent(_)
        | SnapshotIssue::UnexpectedComponent(_)
        | SnapshotIssue::DuplicateCohortFact(_)
        | SnapshotIssue::MissingCohortFact(_)
        | SnapshotIssue::UnexpectedCohortFact(_)
        | SnapshotIssue::WrongOwner(_)
        | SnapshotIssue::MissingRoot(_)
        | SnapshotIssue::WrongRootKind(_)
        | SnapshotIssue::UnexpectedRoot(_)
        | SnapshotIssue::CohortMismatch(_) => "typed-issue",
    }
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn non_claims() -> IOValue {
    field("non-claims", sequence(SNAPSHOT_NON_CLAIMS.iter().map(string).collect()))
}

fn optional_ref(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |reference| record("some", vec![string(reference)]))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
