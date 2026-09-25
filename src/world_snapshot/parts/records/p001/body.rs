
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
