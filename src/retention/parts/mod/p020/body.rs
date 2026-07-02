
fn redacted_bundle_value(
    value: &IoValue,
    path: &str,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<IoValue> {
    let mut frames = Vec::new();
    push_bounded(
        &mut frames,
        RedactionFrame::Visit {
            value: value.clone(),
            path: path.to_string(),
        },
        MAX_RETENTION_REFS,
        "retention bundle redaction stack",
    )?;
    let mut results = RedactionResults::new();
    while let Some(frame) = frames.pop() {
        match frame {
            RedactionFrame::Visit {
                value: current,
                path: current_path,
            } => {
                if let Some(marker) = sensitive_marker(&current, &current_path, bundle_ref, marker_refs)? {
                    results.push(marker)?;
                    continue;
                }
                match current.value_class() {
                    ValueClass::Atomic(_) | ValueClass::Embedded => results.push(current)?,
                    ValueClass::Compound(CompoundClass::Record) => {
                        let label = crate::preserves_rail::value_to_iovalue(&current.label());
                        let children = redaction_children(&current)?;
                        let child_count = children.len();
                        push_bounded(
                            &mut frames,
                            RedactionFrame::BuildRecord { label, child_count },
                            MAX_RETENTION_REFS,
                            "retention bundle redaction stack",
                        )?;
                        push_visit_frames(&mut frames, children, &current_path)?;
                    }
                    ValueClass::Compound(CompoundClass::Sequence) => {
                        let children = redaction_children(&current)?;
                        let child_count = children.len();
                        push_bounded(
                            &mut frames,
                            RedactionFrame::BuildSequence { child_count },
                            MAX_RETENTION_REFS,
                            "retention bundle redaction stack",
                        )?;
                        push_visit_frames(&mut frames, children, &current_path)?;
                    }
                    ValueClass::Compound(CompoundClass::Set) | ValueClass::Compound(CompoundClass::Dictionary) => {
                        results.push(current)?;
                    }
                }
            }
            RedactionFrame::BuildRecord { label, child_count } => results.build_record(label, child_count)?,
            RedactionFrame::BuildSequence { child_count } => results.build_sequence(child_count)?,
        }
    }
    results.finish()
}

fn sensitive_marker(
    value: &IoValue,
    path: &str,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<Option<IoValue>> {
    if let Some(label) = record_label_string(value)
        && is_sensitive_bundle_token(&label)
    {
        return marker_result(bundle_ref, path, &label, marker_refs).map(Some);
    }
    if let Some(text) = value.as_string()
        && is_sensitive_bundle_token(&text)
    {
        return marker_result(bundle_ref, path, &text, marker_refs).map(Some);
    }
    Ok(None)
}

fn marker_result(bundle_ref: &str, path: &str, token: &str, marker_refs: &mut impl VecSink<String>) -> Result<IoValue> {
    let marker_ref = bundle_marker_ref(bundle_ref, path, token)?;
    push_bounded(marker_refs, marker_ref.clone(), MAX_RETENTION_REFS, "retention bundle profile markers")?;
    Ok(crate::preserves_rail::record("retention-bundle-redaction-marker", vec![
        crate::preserves_rail::string(&marker_ref),
    ]))
}

fn redaction_children(value: &IoValue) -> Result<Vec<(usize, IoValue)>> {
    let mut children = Vec::new();
    for (index, child) in value.iter().enumerate() {
        push_bounded(
            &mut children,
            (index, crate::preserves_rail::value_to_iovalue(&child)),
            MAX_RETENTION_REFS,
            "retention bundle redaction children",
        )?;
    }
    Ok(children)
}

fn push_visit_frames(
    frames: &mut impl VecSink<RedactionFrame>,
    children: Vec<(usize, IoValue)>,
    current_path: &str,
) -> Result<()> {
    for (index, child) in children.into_iter().rev() {
        push_bounded(
            frames,
            RedactionFrame::Visit {
                value: child,
                path: format!("{current_path}/{index}"),
            },
            MAX_RETENTION_REFS,
            "retention bundle redaction stack",
        )?;
    }
    Ok(())
}

fn bundle_marker_ref(bundle_ref: &str, path: &str, token: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("retention-bundle-sensitive-marker", vec![
        crate::preserves_rail::string(bundle_ref),
        crate::preserves_rail::string(path),
        crate::preserves_rail::string(token),
    ]))
}

fn is_sensitive_bundle_token(value: &str) -> bool {
    matches!(
        value,
        "secret"
            | "confidential"
            | "credential"
            | "private"
            | "encrypted-ref"
            | "secret-ref-v1"
            | "encrypted-ref-v1"
            | CLASS_PRIVATE_SECRET_REF
    )
}

fn record_label_string(value: &IoValue) -> Option<String> {
    if !value.is_record() {
        return None;
    }
    value.label().as_symbol().map(std::borrow::Cow::into_owned)
}

fn candidate_bundle_profile_value(input: &CandidateBundleProfileValueInput<'_>) -> Result<IoValue> {
    validate_candidate_bundle_profile_value_input(input)?;
    Ok(crate::preserves_rail::record("retention-candidate-bundle-profile-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(input.profile.as_str())]),
        crate::preserves_rail::record("loss-classification", vec![crate::preserves_rail::string(
            input.profile.loss_classification(),
        )]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("markers", vec![strings_sequence(input.marker_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("profile-is-not-authority", "pass"),
            ("read-only-profile", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}

pub fn parse_candidate_bundle_profile(value: &IoValue) -> Result<CandidateBundleProfile> {
    let fields = value
        .collect_simple_record("retention-candidate-bundle-profile-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-candidate-bundle-profile-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_PROFILE_SCHEMA,
        "retention candidate bundle profile schema",
    )?;
    let profile = record_string(&fields[1], "profile")?;
    let parsed_profile = CandidateBundleExportProfile::parse(&profile)?;
    let loss_classification = record_string(&fields[2], "loss-classification")?;
    if loss_classification != parsed_profile.loss_classification() {
        return Err(MoltenError::invalid_harness("retention bundle profile loss classification mismatch"));
    }
    let decision = record_string(&fields[3], "decision")?;
    validate_decision(&decision)?;
    let bundle_ref = record_ref(&fields[4], "bundle")?;
    let marker_refs = record_ref_sequence(&fields[5], "markers")?;
    let diagnostics = record_string_sequence(&fields[6], "diagnostics")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "profile-is-not-authority", "retention candidate bundle profile")?;
    require_check(&checks, "read-only-profile", "retention candidate bundle profile")?;
    require_check(&checks, "normal-admission-still-required", "retention candidate bundle profile")?;
    require_check(&checks, "plan-apply-execute-still-required", "retention candidate bundle profile")?;
    require_check(&checks, "remote-clearance-import-still-required", "retention candidate bundle profile")?;
    Ok(CandidateBundleProfile {
        profile_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        profile,
        loss_classification,
        bundle_ref,
        marker_refs,
        diagnostics,
        value: value.clone(),
    })
}

fn validate_candidate_bundle_profile_value_input(input: &CandidateBundleProfileValueInput<'_>) -> Result<()> {
    validate_decision(input.decision)?;
    require_ref(input.bundle_ref, "retention bundle profile bundle ref")?;
    validate_refs(input.marker_refs, "retention bundle profile marker ref")?;
    validate_diagnostics(input.diagnostics, "retention bundle profile diagnostics")
}

fn candidate_bundle_value(input: &CandidateBundleValueInput<'_>) -> Result<IoValue> {
    validate_candidate_bundle_value_input(input)?;
    Ok(crate::preserves_rail::record("retention-candidate-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_CANDIDATE_BUNDLE_SCHEMA),
        crate::preserves_rail::record("explain", vec![crate::preserves_rail::string(&input.explain.explain_ref)]),
        crate::preserves_rail::record("object", vec![
            crate::preserves_rail::string(&input.explain.object_ref),
            optional_string_value(input.explain.object_kind.as_deref()),
        ]),
        crate::preserves_rail::record("filters", vec![
            crate::preserves_rail::record("class", vec![optional_string_value(
                input.explain.retention_class.as_deref(),
            )]),
            crate::preserves_rail::record("action", vec![optional_string_value(input.explain.action.as_deref())]),
            crate::preserves_rail::record("subsystem", vec![optional_string_value(input.explain.subsystem.as_deref())]),
        ]),
        crate::preserves_rail::record("gc-plans", vec![strings_sequence(&input.explain.gc_plan_refs)]),
        crate::preserves_rail::record("gc-applies", vec![strings_sequence(&input.explain.gc_apply_refs)]),
        crate::preserves_rail::record("gc-executes", vec![strings_sequence(&input.explain.gc_execution_refs)]),
        crate::preserves_rail::record("gc-audits", vec![strings_sequence(&input.explain.gc_audit_refs)]),
        crate::preserves_rail::record("retention-receipts", vec![strings_sequence(
            &input.explain.retention_receipt_refs,
        )]),
        crate::preserves_rail::record("tombstones", vec![strings_sequence(&input.explain.tombstone_refs)]),
        crate::preserves_rail::record("artifacts", vec![strings_sequence(input.artifact_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("bundle-is-not-authority", "pass"),
            ("read-only-export", "pass"),
            ("normal-admission-still-required", "pass"),
            ("plan-apply-execute-still-required", "pass"),
            ("remote-clearance-import-still-required", "pass"),
        ]),
    ]))
}
