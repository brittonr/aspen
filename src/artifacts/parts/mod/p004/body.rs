
fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

fn release_snapshot_dependency_index_digest(root: &Path, artifact_refs: &[String]) -> Result<String> {
    let mut edges = Vec::new();
    for artifact_ref in sorted_unique(artifact_refs) {
        let artifact = read_artifact(root, &artifact_ref)?;
        extend_bounded(
            &mut edges,
            dependency_edges_for_artifact(&artifact)?,
            MAX_ARTIFACT_RECORDS,
            "release snapshot dependency edges",
        )?;
    }
    dependency_index_digest(&edges)
}

fn release_snapshot_install_evidence_refs(input: &ReleaseSnapshotValueInput) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    extend_cloned_bounded(&mut refs, &input.doc_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.transcript_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.expected_receipt_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.source_gate_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.resource_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.compatibility_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.migration_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.upgrade_session_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.rollback_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.cutover_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.signature_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.stale_evidence_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.dependency_closure_digest.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.dependency_index_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.signature_subject_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    if let Some(artifact_set_ref) = input.artifact_set_ref.as_ref() {
        push_bounded(&mut refs, artifact_set_ref.clone(), MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    }
    if let Some(redaction_profile_ref) = input.redaction_profile_ref.as_ref() {
        push_bounded(
            &mut refs,
            redaction_profile_ref.clone(),
            MAX_ARTIFACT_REF_LIST,
            "release snapshot evidence refs",
        )?;
    }
    Ok(sorted_unique(&refs))
}

fn release_snapshot_verify_refs(snapshot_artifact_ref: &str, snapshot: &ReleaseSnapshot) -> Result<Vec<String>> {
    validate_ref(snapshot_artifact_ref, "release snapshot artifact ref")?;
    let mut refs = Vec::new();
    push_bounded(
        &mut refs,
        snapshot_artifact_ref.to_string(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    push_bounded(
        &mut refs,
        snapshot.snapshot_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    extend_cloned_bounded(&mut refs, &snapshot.artifact_refs, MAX_ARTIFACT_REF_LIST, "release snapshot verify refs")?;
    extend_cloned_bounded(
        &mut refs,
        &release_snapshot_install_evidence_refs(&ReleaseSnapshotValueInput {
            namespace_scope: snapshot.namespace_scope.clone(),
            snapshot_id: snapshot.snapshot_id.clone(),
            artifact_refs: snapshot.artifact_refs.clone(),
            artifact_set_ref: snapshot.artifact_set_ref.clone(),
            dependency_closure_digest: snapshot.dependency_closure_digest.clone(),
            dependency_index_ref: snapshot.dependency_index_ref.clone(),
            doc_refs: snapshot.doc_refs.clone(),
            transcript_refs: snapshot.transcript_refs.clone(),
            expected_receipt_refs: snapshot.expected_receipt_refs.clone(),
            policy_refs: snapshot.policy_refs.clone(),
            provenance_refs: snapshot.provenance_refs.clone(),
            source_gate_refs: snapshot.source_gate_refs.clone(),
            resource_refs: snapshot.resource_refs.clone(),
            compatibility_refs: snapshot.compatibility_refs.clone(),
            migration_refs: snapshot.migration_refs.clone(),
            upgrade_session_refs: snapshot.upgrade_session_refs.clone(),
            rollback_refs: snapshot.rollback_refs.clone(),
            cutover_refs: snapshot.cutover_refs.clone(),
            caveats: snapshot.caveats.clone(),
            non_claims: snapshot.non_claims.clone(),
            redaction_profile_ref: snapshot.redaction_profile_ref.clone(),
            signature_subject_ref: snapshot.signature_subject_ref.clone(),
            signature_refs: snapshot.signature_refs.clone(),
            stale_evidence_refs: snapshot.stale_evidence_refs.clone(),
        })?,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    extend_cloned_bounded(&mut refs, &snapshot.policy_refs, MAX_ARTIFACT_REF_LIST, "release snapshot verify refs")?;
    Ok(sorted_unique(&refs))
}

fn set_difference(left: &[String], right: &[String]) -> Result<Vec<String>> {
    let right_set = right.iter().collect::<std::collections::BTreeSet<_>>();
    let mut difference = Vec::new();
    for item in left {
        if !right_set.contains(item) {
            push_bounded(&mut difference, item.clone(), MAX_ARTIFACT_REF_LIST, "release snapshot set difference")?;
        }
    }
    Ok(difference)
}

fn release_snapshot_caveats_rendered(
    snapshot: &ReleaseSnapshot,
    required_caveats: &[String],
    diagnostics: &mut Vec<String>,
) -> Result<bool> {
    let mut rendered = true;
    if snapshot.caveats.is_empty() {
        rendered = false;
        push_bounded(
            diagnostics,
            "release snapshot must render at least one caveat".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
    }
    for required in required_caveats {
        if !snapshot.caveats.iter().any(|caveat| caveat == required) {
            rendered = false;
            push_bounded(
                diagnostics,
                format!("required caveat not rendered: {required}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "release snapshot diagnostics",
            )?;
        }
    }
    Ok(rendered)
}

fn release_snapshot_fresh_evidence(snapshot: &ReleaseSnapshot, diagnostics: &mut Vec<String>) -> Result<bool> {
    for stale_ref in &snapshot.stale_evidence_refs {
        push_bounded(
            diagnostics,
            format!("stale evidence ref {stale_ref} prevents release snapshot pass evidence"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
    }
    Ok(snapshot.stale_evidence_refs.is_empty())
}

fn release_snapshot_redaction_bound(snapshot: &ReleaseSnapshot, diagnostics: &mut Vec<String>) -> Result<bool> {
    if snapshot.redaction_profile_ref.is_some()
        && !snapshot.caveats.iter().any(|caveat| caveat.contains("redaction") || caveat.contains("redacted"))
    {
        push_bounded(
            diagnostics,
            "redaction profile is bound but no redaction caveat is rendered".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

fn release_snapshot_required_evidence_bound(
    snapshot: &ReleaseSnapshot,
    diagnostics: &mut Vec<String>,
) -> Result<bool> {
    let mut bound = true;
    bound &= require_non_empty_refs(&snapshot.doc_refs, "release snapshot docs", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.transcript_refs, "release snapshot transcripts", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.expected_receipt_refs, "release snapshot expected receipts", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.policy_refs, "release snapshot policy evidence", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.provenance_refs, "release snapshot provenance evidence", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.source_gate_refs, "release snapshot source-gate evidence", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.resource_refs, "release snapshot resource evidence", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.compatibility_refs, "release snapshot compatibility receipts", diagnostics)?;
    bound &= require_non_empty_refs(&snapshot.migration_refs, "release snapshot migration receipts", diagnostics)?;
    Ok(bound)
}

fn require_non_empty_refs(refs: &[String], label: &str, diagnostics: &mut Vec<String>) -> Result<bool> {
    if refs.is_empty() {
        push_bounded(
            diagnostics,
            format!("{label} must be bound"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
        Ok(false)
    } else {
        Ok(true)
    }
}

fn release_channel_update_refs(input: &ReleaseChannelUpdateInput, pointer: &ArtifactNamePointer) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_bounded(&mut refs, input.snapshot_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    push_bounded(&mut refs, pointer.pointer_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    push_bounded(&mut refs, pointer.receipt_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    if let Some(previous_ref) = pointer.previous_ref.as_ref() {
        push_bounded(&mut refs, previous_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    }
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    Ok(sorted_unique(&refs))
}

fn release_channel_admission_refs(input: &ReleaseChannelAdmissionInput) -> Result<Vec<String>> {
    let mut refs = vec![input.channel_pointer_ref.clone()];
    extend_cloned_bounded(
        &mut refs,
        &input.release_evidence_refs,
        MAX_ARTIFACT_REF_LIST,
        "release channel admission refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.source_gate_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.authority_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.resource_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    Ok(sorted_unique(&refs))
}

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

fn push_missing_ref_diagnostic(diagnostics: &mut Vec<String>, refs: &[String], label: &str) -> Result<()> {
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
    crate::preserves_rail::record_content_ref_strings(value, label, label, MAX_ARTIFACT_REF_LIST)
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
    let parsed = crate::preserves_rail::parse_checks_record(value, MAX_ARTIFACT_CHECKS, "artifact registry")?;
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
    crate::preserves_rail::simple_record_fields(value, label, arity)
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

fn active_resolution_candidates(input: &ArtifactNameResolutionInput) -> Result<Vec<ArtifactNameView>> {
    let stale = input.stale_view_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let mut candidates = Vec::new();
    for view in &input.candidate_views {
        if view.view_kind != input.view_kind || view.name != input.name || view.tombstone_ref.is_some() {
            continue;
        }
        if input.scope.as_ref().is_some_and(|scope| &view.scope != scope) {
            continue;
        }
        if stale.contains(&view.view_ref) {
            continue;
        }
        push_bounded(&mut candidates, view.clone(), MAX_ARTIFACT_RECORDS, "artifact name resolution candidates")?;
    }
    candidates.sort_by(|left, right| left.view_ref.cmp(&right.view_ref));
    Ok(candidates)
}

fn name_resolution_diagnostics(
    input: &ArtifactNameResolutionInput,
    candidates: &[ArtifactNameView],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for stale_ref in &input.stale_view_refs {
        if input.candidate_views.iter().any(|view| &view.view_ref == stale_ref) {
            push_bounded(
                &mut diagnostics,
                format!("deny stale name view {stale_ref}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
    }
    match candidates.len() {
        0 => push_bounded(
            &mut diagnostics,
            "deny name resolution has no active exact-ref candidate".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name resolution diagnostics",
        )?,
        1 => {
            let scope = candidates[0].scope.clone();
            push_bounded(
                &mut diagnostics,
                format!("resolved exact artifact ref in scope {scope}; name views are non-authority"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
        _ => {
            let refs = candidates.iter().map(|view| view.target_ref.clone()).collect::<Vec<_>>().join(",");
            push_bounded(
                &mut diagnostics,
                format!("deny ambiguous name resolution candidates: {refs}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
    }
    if input.normative_use {
        push_bounded(
            &mut diagnostics,
            "normative use must pin the resolved exact artifact ref".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name resolution diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn name_use_diagnostics(input: &ArtifactNameUseInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.exact_artifact_ref.is_none() {
        push_bounded(
            &mut diagnostics,
            "name-only use denies until exact artifact ref is pinned".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    if input.resolution_receipt_ref.is_none() && input.name.is_some() {
        push_bounded(
            &mut diagnostics,
            "name use must bind an admitted resolution receipt".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() || input.provenance_refs.is_empty() || input.capability_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "name views do not grant policy, provenance, or capability authority".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn name_use_refs(input: &ArtifactNameUseInput) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    if let Some(exact_artifact_ref) = input.exact_artifact_ref.as_ref() {
        push_bounded(&mut refs, exact_artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    }
    if let Some(resolution_receipt_ref) = input.resolution_receipt_ref.as_ref() {
        push_bounded(
            &mut refs,
            resolution_receipt_ref.clone(),
            MAX_ARTIFACT_REF_LIST,
            "artifact name use refs",
        )?;
    }
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    if refs.is_empty() {
        push_bounded(
            &mut refs,
            canonical_hash(&record("artifact-name-use-denial", vec![string(&input.operation)]))?,
            MAX_ARTIFACT_REF_LIST,
            "artifact name use refs",
        )?;
    }
    Ok(sorted_unique(&refs))
}

fn validate_release_snapshot_draft(input: &ReleaseSnapshotDraftInput) -> Result<()> {
    validate_non_empty(&input.namespace_scope, "release snapshot namespace")?;
    validate_non_empty(&input.snapshot_id, "release snapshot id")?;
    validate_release_snapshot_refs(
        &ReleaseSnapshotValueInput {
            namespace_scope: input.namespace_scope.clone(),
            snapshot_id: input.snapshot_id.clone(),
            artifact_refs: input.artifact_refs.clone(),
            artifact_set_ref: input.artifact_set_ref.clone(),
            dependency_closure_digest: testable_placeholder_ref("release-snapshot-closure")?,
            dependency_index_ref: testable_placeholder_ref("release-snapshot-index")?,
            doc_refs: input.doc_refs.clone(),
            transcript_refs: input.transcript_refs.clone(),
            expected_receipt_refs: input.expected_receipt_refs.clone(),
            policy_refs: input.policy_refs.clone(),
            provenance_refs: input.provenance_refs.clone(),
            source_gate_refs: input.source_gate_refs.clone(),
            resource_refs: input.resource_refs.clone(),
            compatibility_refs: input.compatibility_refs.clone(),
            migration_refs: input.migration_refs.clone(),
            upgrade_session_refs: input.upgrade_session_refs.clone(),
            rollback_refs: input.rollback_refs.clone(),
            cutover_refs: input.cutover_refs.clone(),
            caveats: input.caveats.clone(),
            non_claims: input.non_claims.clone(),
            redaction_profile_ref: input.redaction_profile_ref.clone(),
            signature_subject_ref: testable_placeholder_ref("release-snapshot-subject")?,
            signature_refs: input.signature_refs.clone(),
            stale_evidence_refs: input.stale_evidence_refs.clone(),
        },
        "release snapshot draft",
    )
}

fn validate_release_snapshot_value_input(input: &ReleaseSnapshotValueInput) -> Result<()> {
    validate_non_empty(&input.namespace_scope, "release snapshot namespace")?;
    validate_non_empty(&input.snapshot_id, "release snapshot id")?;
    validate_release_snapshot_refs(input, "release snapshot")
}

fn validate_release_snapshot_subject_input(input: &ReleaseSnapshotSubjectInput<'_>) -> Result<()> {
    validate_non_empty(input.namespace_scope, "release snapshot subject namespace")?;
    validate_non_empty(input.snapshot_id, "release snapshot subject id")?;
    validate_refs(input.artifact_refs, "release snapshot subject artifact ref")?;
    ensure_non_empty(input.artifact_refs.len(), "release snapshot subject artifacts")?;
    if let Some(artifact_set_ref) = input.artifact_set_ref {
        validate_ref(artifact_set_ref, "release snapshot subject artifact set ref")?;
    }
    validate_ref(input.dependency_closure_digest, "release snapshot subject closure digest")?;
    validate_ref(input.dependency_index_ref, "release snapshot subject dependency index ref")?;
    validate_refs(input.doc_refs, "release snapshot subject doc ref")?;
    validate_refs(input.transcript_refs, "release snapshot subject transcript ref")?;
    validate_refs(input.expected_receipt_refs, "release snapshot subject expected receipt ref")?;
    validate_refs(input.policy_refs, "release snapshot subject policy ref")?;
    validate_refs(input.provenance_refs, "release snapshot subject provenance ref")?;
    validate_refs(input.source_gate_refs, "release snapshot subject source gate ref")?;
    validate_refs(input.resource_refs, "release snapshot subject resource ref")?;
    validate_refs(input.compatibility_refs, "release snapshot subject compatibility ref")?;
    validate_refs(input.migration_refs, "release snapshot subject migration ref")?;
    validate_refs(input.upgrade_session_refs, "release snapshot subject upgrade ref")?;
    validate_refs(input.rollback_refs, "release snapshot subject rollback ref")?;
    validate_refs(input.cutover_refs, "release snapshot subject cutover ref")?;
    validate_strings(input.caveats, "release snapshot subject caveat")?;
    validate_strings(input.non_claims, "release snapshot subject non-claim")?;
    if let Some(redaction_profile_ref) = input.redaction_profile_ref {
        validate_ref(redaction_profile_ref, "release snapshot subject redaction profile ref")?;
    }
    validate_refs(input.stale_evidence_refs, "release snapshot subject stale evidence ref")
}

fn validate_release_snapshot_refs(input: &ReleaseSnapshotValueInput, label: &str) -> Result<()> {
    validate_refs(&input.artifact_refs, "release snapshot artifact ref")?;
    ensure_non_empty(input.artifact_refs.len(), "release snapshot artifacts")?;
    if let Some(artifact_set_ref) = input.artifact_set_ref.as_ref() {
        validate_ref(artifact_set_ref, "release snapshot artifact set ref")?;
    }
    validate_ref(&input.dependency_closure_digest, "release snapshot closure digest")?;
    validate_ref(&input.dependency_index_ref, "release snapshot dependency index ref")?;
    validate_refs(&input.doc_refs, "release snapshot doc ref")?;
    validate_refs(&input.transcript_refs, "release snapshot transcript ref")?;
    validate_refs(&input.expected_receipt_refs, "release snapshot expected receipt ref")?;
    validate_refs(&input.policy_refs, "release snapshot policy ref")?;
    validate_refs(&input.provenance_refs, "release snapshot provenance ref")?;
    validate_refs(&input.source_gate_refs, "release snapshot source gate ref")?;
    validate_refs(&input.resource_refs, "release snapshot resource ref")?;
    validate_refs(&input.compatibility_refs, "release snapshot compatibility ref")?;
    validate_refs(&input.migration_refs, "release snapshot migration ref")?;
    validate_refs(&input.upgrade_session_refs, "release snapshot upgrade session ref")?;
    validate_refs(&input.rollback_refs, "release snapshot rollback ref")?;
    validate_refs(&input.cutover_refs, "release snapshot cutover ref")?;
    validate_strings(&input.caveats, "release snapshot caveat")?;
    validate_strings(&input.non_claims, "release snapshot non-claim")?;
    if let Some(redaction_profile_ref) = input.redaction_profile_ref.as_ref() {
        validate_ref(redaction_profile_ref, "release snapshot redaction profile ref")?;
    }
    validate_ref(&input.signature_subject_ref, "release snapshot signature subject ref")?;
    validate_refs(&input.signature_refs, "release snapshot signature ref")?;
    ensure_non_empty(input.signature_refs.len(), "release snapshot signatures")?;
    validate_refs(&input.stale_evidence_refs, "release snapshot stale evidence ref")?;
    ensure_count_at_most(input.caveats.len(), MAX_ARTIFACT_DIAGNOSTICS, label)?;
    ensure_count_at_most(input.non_claims.len(), MAX_ARTIFACT_DIAGNOSTICS, label)
}

fn validate_release_channel_update_input(input: &ReleaseChannelUpdateInput) -> Result<()> {
    validate_non_empty(&input.channel, "release channel name")?;
    validate_ref(&input.snapshot_ref, "release channel snapshot ref")?;
    validate_refs(&input.policy_refs, "release channel policy ref")?;
    validate_refs(&input.capability_refs, "release channel capability ref")?;
    validate_refs(&input.evidence_refs, "release channel evidence ref")?;
    ensure_non_empty(input.policy_refs.len(), "release channel policy refs")?;
    ensure_non_empty(input.capability_refs.len(), "release channel capability refs")
}

fn validate_release_channel_admission_input(input: &ReleaseChannelAdmissionInput) -> Result<()> {
    validate_ref(&input.channel_pointer_ref, "release channel pointer ref")?;
    validate_refs(&input.release_evidence_refs, "release channel release evidence ref")?;
    validate_refs(&input.policy_refs, "release channel policy ref")?;
    validate_refs(&input.provenance_refs, "release channel provenance ref")?;
    validate_refs(&input.source_gate_refs, "release channel source gate ref")?;
    validate_refs(&input.authority_refs, "release channel authority ref")?;
    validate_refs(&input.resource_refs, "release channel resource ref")
}

fn validate_strings(values: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_ARTIFACT_REF_LIST, field)?;
    for value in values {
        validate_non_empty(value, field)?;
    }
    Ok(())
}

fn ensure_non_empty(count: usize, label: &str) -> Result<()> {
    if count == 0 {
        Err(MoltenError::invalid_harness(format!("{label} cannot be empty")))
    } else {
        Ok(())
    }
}

fn testable_placeholder_ref(label: &'static str) -> Result<String> {
    canonical_hash(&record("artifact-placeholder-ref", vec![string(label)]))
}

fn validate_install_input(input: &ArtifactInstallInput) -> Result<()> {
    validate_kind(&input.kind)?;
    validate_refs(&input.schema_refs, "artifact schema ref")?;
    validate_refs(&input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(&input.policy_refs, "artifact policy ref")?;
    validate_refs(&input.evidence_refs, "artifact evidence ref")?;
    validate_ref(&input.installer_ref, "artifact installer ref")?;
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("artifact install requires at least one capability ref"));
    }
    validate_refs(&input.capability_refs, "artifact capability ref")
}

fn validate_kind(kind: &str) -> Result<()> {
    validate_non_empty(kind, "artifact kind")?;
    if kind.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "artifact kind {kind} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_pointer_kind(kind: &str) -> Result<()> {
    if matches!(kind, "name" | "alias" | "tag" | "channel") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported artifact pointer kind {kind}; expected name, alias, tag, or channel"
        )))
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_ARTIFACT_REF_LIST, field)?;
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn extend_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: Vec<T>,
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    for item in incoming {
        values.push_item(item);
    }
    Ok(())
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("artifact registry redb index error: {error}"))
}
