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

const SNAPSHOT_DESCRIPTOR_ARITY: usize = 9;
const COHORT_FACT_ARITY: usize = 2;
const SNAPSHOT_COMPONENT_ARITY: usize = 4;
const TYPED_ROOT_ARITY: usize = 2;
const SYNCHRONIZATION_ARITY: usize = 3;

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

pub fn parse_canonical_snapshot_descriptor(bytes: &[u8]) -> Result<(SnapshotDescriptor, CanonicalSnapshotArtifact)> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(
        &decoded.value,
        SNAPSHOT_DESCRIPTOR_RECORD,
        SNAPSHOT_DESCRIPTOR_ARITY,
    )?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "snapshot descriptor schema")?;
    if schema != SNAPSHOT_DESCRIPTOR_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported snapshot descriptor schema"));
    }
    let class = SnapshotClass::parse(&required_named_string(&fields[1], "class")?).map_err(snapshot_parse_issue)?;
    let commit_ref = molten_core::world_commit::WorldCommitRef::new(required_named_ref(&fields[2], "commit-ref")?)
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot commit ref: {issue:?}")))?;
    let profile_ref =
        molten_core::world_commit::SnapshotProfileRef::new(required_named_ref(&fields[3], "profile-ref")?)
            .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot profile ref: {issue:?}")))?;
    let cohort_ref =
        molten_core::world_commit::SnapshotCohortRef::new(required_named_ref(&fields[4], "cohort-ref")?)
            .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot cohort ref: {issue:?}")))?;
    let cohort_values = required_named_sequence(&fields[5], "cohort-facts", MAX_COHORT_FACTS)?;
    let facts = cohort_values.iter().map(parse_cohort_fact).collect::<Result<Vec<_>>>()?;
    let component_values = required_named_sequence(&fields[6], "components", MAX_SNAPSHOT_COMPONENTS)?;
    let components = component_values.iter().map(parse_component).collect::<Result<Vec<_>>>()?;
    let contains_live_handle = parse_named_boolean(&fields[7], "contains-live-handle")?;
    let synchronization = parse_synchronization(&fields[8])?;
    let descriptor = SnapshotDescriptor {
        class,
        commit_ref,
        profile_ref,
        cohort: SnapshotCohort { cohort_ref, facts },
        components,
        contains_live_handle,
        synchronization,
    };
    let canonical = canonical_snapshot_descriptor(&descriptor)?;
    if canonical.bytes != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness(
            "snapshot descriptor is canonical Preserves but not normalized snapshot order",
        ));
    }
    Ok((descriptor, canonical))
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

fn parse_cohort_fact(value: &preserves::Value<IOValue>) -> Result<CohortFact> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "cohort-fact", COHORT_FACT_ARITY)?;
    let kind_text = crate::preserves_rail::required_string_field(&fields[0], "snapshot cohort fact kind")?;
    let kind = CohortFactKind::parse(&kind_text).map_err(snapshot_parse_issue)?;
    let identity = crate::preserves_rail::required_content_ref_string(&fields[1], "snapshot cohort fact identity")?;
    Ok(CohortFact { kind, identity })
}

fn parse_component(value: &preserves::Value<IOValue>) -> Result<SnapshotComponent> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, "snapshot-component", SNAPSHOT_COMPONENT_ARITY)?;
    let kind_text = crate::preserves_rail::required_string_field(&fields[0], "snapshot component kind")?;
    let kind = SnapshotComponentKind::parse(&kind_text).map_err(snapshot_parse_issue)?;
    let identity = crate::preserves_rail::required_content_ref_string(&fields[1], "snapshot component identity")?;
    let root = parse_optional_root(&fields[2])?;
    let owner_text = crate::preserves_rail::required_string_field(&fields[3], "snapshot component owner")?;
    let owner = ComponentOwner::parse(&owner_text).map_err(snapshot_parse_issue)?;
    Ok(SnapshotComponent {
        kind,
        identity,
        root,
        owner,
    })
}

fn parse_optional_root(value: &preserves::Value<IOValue>) -> Result<Option<WorldRootRef>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("snapshot root must be <none> or <some ROOT>"))?;
    let root_value = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let root_fields = crate::preserves_rail::simple_record_fields(&root_value, "typed-root", TYPED_ROOT_ARITY)?;
    let kind_text = crate::preserves_rail::required_string_field(&root_fields[0], "snapshot root kind")?;
    let kind = molten_core::world_commit::RootKind::parse(&kind_text)
        .map_err(|_| MoltenError::invalid_harness("unsupported snapshot root kind"))?;
    let reference = crate::preserves_rail::required_content_ref_string(&root_fields[1], "snapshot root ref")?;
    molten_core::world_commit::WorldRootRef::parse(kind, reference)
        .map(Some)
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot root ref: {issue:?}")))
}

fn parse_synchronization(value: &preserves::Value<IOValue>) -> Result<Option<SnapshotSynchronization>> {
    let inner = named_field_value(value, "synchronization")?;
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = inner
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("snapshot synchronization must be <none> or <some FACT>"))?;
    let synchronization_value = crate::preserves_rail::value_to_iovalue(&some[0]);
    let fields =
        crate::preserves_rail::simple_record_fields(&synchronization_value, "synchronization", SYNCHRONIZATION_ARITY)?;
    let logical_commit_ref = molten_core::world_commit::WorldCommitRef::new(
        crate::preserves_rail::required_content_ref_string(&fields[0], "synchronized logical commit")?,
    )
    .map_err(|issue| MoltenError::invalid_harness(format!("invalid synchronized commit: {issue:?}")))?;
    let opaque_ref = crate::preserves_rail::required_content_ref_string(&fields[1], "synchronized opaque root")?;
    let opaque_snapshot_ref = molten_core::world_commit::WorldRootRef::parse(
        molten_core::world_commit::RootKind::OpaqueMachineSnapshot,
        opaque_ref,
    )
    .map_err(|issue| MoltenError::invalid_harness(format!("invalid synchronized opaque root: {issue:?}")))?;
    let observation_ref =
        crate::preserves_rail::required_content_ref_string(&fields[2], "snapshot synchronization observation")?;
    Ok(Some(SnapshotSynchronization {
        logical_commit_ref,
        opaque_snapshot_ref,
        observation_ref,
    }))
}

fn required_named_string(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(&named_field_value(value, label)?, label)
}

fn required_named_ref(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_content_ref_string(&named_field_value(value, label)?, label)
}

fn required_named_sequence(
    value: &preserves::Value<IOValue>,
    label: &str,
    maximum: usize,
) -> Result<Vec<preserves::Value<IOValue>>> {
    let inner = named_field_value(value, label)?;
    let values = crate::preserves_rail::required_sequence_field(&inner, label)?;
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "snapshot {label} count {} exceeds maximum {maximum}",
            values.len()
        )));
    }
    Ok(values.into_owned())
}

fn parse_named_boolean(value: &preserves::Value<IOValue>, label: &str) -> Result<bool> {
    let inner = named_field_value(value, label)?;
    if inner.collect_simple_record("true", Some(0)).is_some() {
        return Ok(true);
    }
    if inner.collect_simple_record("false", Some(0)).is_some() {
        return Ok(false);
    }
    Err(MoltenError::invalid_harness(format!("snapshot {label} must be <true> or <false>")))
}

fn named_field_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(fields[0].clone())
}

fn snapshot_parse_issue(issue: SnapshotIssue) -> MoltenError {
    MoltenError::invalid_harness(format!("snapshot descriptor parse denied: {issue:?}"))
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
        SnapshotIssue::UnsupportedComponentKind => "unsupported-component-kind",
        SnapshotIssue::UnsupportedCohortFact => "unsupported-cohort-fact",
        SnapshotIssue::UnsupportedOwner => "unsupported-owner",
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
