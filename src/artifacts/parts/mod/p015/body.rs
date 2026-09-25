
fn release_channel_admission_diagnostics(input: &ReleaseChannelAdmissionInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.release_evidence_refs.is_empty()
        || input.policy_refs.is_empty()
        || input.provenance_refs.is_empty()
        || input.source_gate_refs.is_empty()
        || input.authority_refs.is_empty()
        || input.resource_refs.is_empty()
    {
        push_bounded(
            &mut diagnostics,
            "release channel names are non-authority; bind release, policy, provenance, source-gate, authority, and resource evidence".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release channel admission diagnostics",
        )?;
    }
    push_missing_ref_diagnostic(&mut diagnostics, &input.release_evidence_refs, "release evidence")?;
    push_missing_ref_diagnostic(&mut diagnostics, &input.policy_refs, "policy evidence")?;
    push_missing_ref_diagnostic(&mut diagnostics, &input.provenance_refs, "provenance evidence")?;
    push_missing_ref_diagnostic(&mut diagnostics, &input.source_gate_refs, "source-gate evidence")?;
    push_missing_ref_diagnostic(&mut diagnostics, &input.authority_refs, "authority evidence")?;
    push_missing_ref_diagnostic(&mut diagnostics, &input.resource_refs, "resource evidence")?;
    Ok(diagnostics)
}

fn push_missing_ref_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, refs: &[String], label: &str) -> Result<()> {
    if refs.is_empty() {
        push_bounded(
            diagnostics,
            format!("release channel admission missing {label}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release channel admission diagnostics",
        )?;
    }
    Ok(())
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    crate::preserves_rail::optional_ref_value(value)
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &RailValue) -> Result<Option<String>> {
    crate::preserves_rail::optional_content_ref_string(value, "optional ref")
}

fn parse_optional_string_value(value: &RailValue) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn record_string(value: &RailValue, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &RailValue, label: &str) -> Result<String> {
    crate::preserves_rail::record_content_ref_string(value, label, label)
}

fn record_optional_ref(value: &RailValue, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_optional_string(value: &RailValue, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_string_value(&record[0])
}

fn record_ref_sequence(value: &RailValue, label: &str) -> Result<Vec<String>> {
    crate::preserves_rail::record_content_ref_strings(
        value,
        label,
        label,
        crate::bounded::u64_from_usize(MAX_ARTIFACT_REF_LIST, "artifact ref list bound")?,
    )
}

fn record_strings(value: &RailValue, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_ARTIFACT_DIAGNOSTICS, label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut strings, required_string(item, label)?, MAX_ARTIFACT_DIAGNOSTICS, label)?;
    }
    Ok(strings)
}

fn parse_ref_sequence_value(value: &RailValue, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_ARTIFACT_REF_LIST, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_ARTIFACT_REF_LIST, label)?;
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

fn parse_checks(value: &RailValue) -> Result<Vec<String>> {
    let parsed = crate::preserves_rail::parse_checks_record(
        value,
        crate::bounded::u64_from_usize(MAX_ARTIFACT_CHECKS, "artifact check bound")?,
        "artifact registry",
    )?;
    let mut names = Vec::with_capacity(parsed.len());
    for check in parsed {
        if check.status != "pass" && check.status != "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "artifact registry check {} has status {}",
                check.name, check.status
            )));
        }
        push_bounded(&mut names, check.name, MAX_ARTIFACT_CHECKS, "artifact checks")?;
    }
    Ok(names)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &RailValue, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, preserves::Record<RailValue>>> {
    crate::preserves_rail::simple_record_fields(value, label, crate::bounded::u64_from_usize(arity, "artifact record arity")?)
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a RailValue, field: &str) -> Result<std::borrow::Cow<'a, Vec<RailValue>>> {
    crate::preserves_rail::required_sequence_field(value, field)
}

fn required_string(value: &RailValue, field: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(value, field)
}

fn required_ref(value: &RailValue, field: &str) -> Result<String> {
    crate::preserves_rail::required_content_ref_string(value, field)
}

fn required_u64(value: &RailValue, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn validate_name_view_input(input: &ArtifactNameViewInput) -> Result<()> {
    validate_pointer_kind(&input.view_kind)?;
    validate_non_empty(&input.name, "artifact name view name")?;
    validate_dependency_label(&input.scope, "artifact name view scope")?;
    validate_name_view_target_kind(&input.target_kind)?;
    validate_ref(&input.target_ref, "artifact name view target ref")?;
    validate_ref(&input.issuer_ref, "artifact name view issuer ref")?;
    validate_refs(&input.policy_refs, "artifact name view policy ref")?;
    validate_refs(&input.evidence_refs, "artifact name view evidence ref")?;
    validate_refs(&input.capability_refs, "artifact name view capability ref")?;
    if let Some(tombstone_ref) = input.tombstone_ref.as_ref() {
        validate_ref(tombstone_ref, "artifact name view tombstone ref")?;
    }
    Ok(())
}

fn validate_name_view_update_authority(input: &ArtifactNameViewInput) -> Result<()> {
    validate_name_view_input(input)?;
    ensure_non_empty(input.capability_refs.len(), "artifact name view capability refs")?;
    ensure_non_empty(input.policy_refs.len(), "artifact name view policy refs")
}

fn validate_name_view_target_kind(kind: &str) -> Result<()> {
    match kind {
        "artifact-ref" | "artifact-set-ref" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!(
            "unsupported artifact name view target kind {kind}; expected artifact-ref or artifact-set-ref"
        ))),
    }
}

fn validate_name_resolution_input(input: &ArtifactNameResolutionInput) -> Result<()> {
    validate_pointer_kind(&input.view_kind)?;
    validate_non_empty(&input.name, "artifact name resolution name")?;
    if let Some(scope) = input.scope.as_ref() {
        validate_dependency_label(scope, "artifact name resolution scope")?;
    }
    validate_refs(&input.stale_view_refs, "artifact name resolution stale view ref")?;
    ensure_count_at_most(
        input.candidate_views.len(),
        MAX_ARTIFACT_RECORDS,
        "artifact name resolution candidates",
    )
}

fn validate_name_use_input(input: &ArtifactNameUseInput) -> Result<()> {
    validate_non_empty(&input.operation, "artifact name use operation")?;
    if let Some(name) = input.name.as_ref() {
        validate_non_empty(name, "artifact name use name")?;
    }
    if let Some(exact_artifact_ref) = input.exact_artifact_ref.as_ref() {
        validate_ref(exact_artifact_ref, "artifact name use exact artifact ref")?;
    }
    if let Some(resolution_receipt_ref) = input.resolution_receipt_ref.as_ref() {
        validate_ref(resolution_receipt_ref, "artifact name use resolution receipt ref")?;
    }
    validate_refs(&input.policy_refs, "artifact name use policy ref")?;
    validate_refs(&input.provenance_refs, "artifact name use provenance ref")?;
    validate_refs(&input.capability_refs, "artifact name use capability ref")
}

fn scoped_name_view_key(scope: &str, name: &str) -> Result<String> {
    validate_dependency_label(scope, "artifact name view scope")?;
    validate_non_empty(name, "artifact name view name")?;
    Ok(format!("{scope}:{name}"))
}

fn name_view_update_refs(
    input: &ArtifactNameViewInput,
    view: &ArtifactNameView,
    pointer: &ArtifactNamePointer,
) -> Result<Vec<String>> {
    let mut refs = vec![view.view_ref.clone(), pointer.pointer_ref.clone(), pointer.receipt_ref.clone(), input.target_ref.clone()];
    push_bounded(&mut refs, input.issuer_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    if let Some(previous_view_ref) = view.previous_view_ref.as_ref() {
        push_bounded(&mut refs, previous_view_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    }
    if let Some(tombstone_ref) = input.tombstone_ref.as_ref() {
        push_bounded(&mut refs, tombstone_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact name view refs")?;
    }
    Ok(sorted_unique(&refs))
}
