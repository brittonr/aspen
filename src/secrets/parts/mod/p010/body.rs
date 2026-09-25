
pub fn private_bundle_profile_value(input: &PrivateBundleProfileInput) -> Result<IoValue> {
    validate_ref(&input.profile_ref, "private bundle profile ref")?;
    validate_refs(&input.encrypted_refs, "private bundle encrypted ref")?;
    validate_refs(&input.reveal_receipt_refs, "private bundle reveal receipt")?;
    validate_ref(&input.transform_receipt_ref, "private bundle transform receipt")?;
    let checks = if input.is_gate_preserving {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("gate-preserving-redaction", "pass"),
        ]
    } else {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("diagnostic-only", "pass"),
        ]
    };
    Ok(record("private-bundle-profile-v1", vec![
        string(PRIVATE_BUNDLE_PROFILE_SCHEMA),
        record("profile", vec![string(&input.profile_ref)]),
        record("encrypted-refs", vec![refs_sequence(&input.encrypted_refs)]),
        record("reveal-receipts", vec![refs_sequence(&input.reveal_receipt_refs)]),
        record("transform-receipt", vec![string(&input.transform_receipt_ref)]),
        record("gate-preserving", vec![bool_value(input.is_gate_preserving)]),
        checks_value(&checks),
    ]))
}

pub fn parse_private_bundle_profile(value: &IoValue) -> Result<PrivateBundleProfile> {
    let fields = simple_record(value, "private-bundle-profile-v1", 7)?;
    require_schema(&fields[0], PRIVATE_BUNDLE_PROFILE_SCHEMA, "private bundle profile")?;
    let profile_ref = record_ref(&fields[1], "profile", "private bundle profile ref")?;
    let encrypted_refs = record_refs(&fields[2], "encrypted-refs", "private bundle encrypted refs")?;
    let reveal_receipt_refs = record_refs(&fields[3], "reveal-receipts", "private bundle reveal receipts")?;
    let transform_receipt_ref = record_ref(&fields[4], "transform-receipt", "private bundle transform receipt")?;
    let is_gate_preserving = record_bool(&fields[5], "gate-preserving", "private bundle gate preserving")?;
    if is_gate_preserving {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "gate-preserving-redaction",
        ])?;
    } else {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "diagnostic-only",
        ])?;
    }
    Ok(PrivateBundleProfile {
        profile_ref,
        encrypted_refs,
        reveal_receipt_refs,
        transform_receipt_ref,
        is_gate_preserving,
        value: value.clone(),
    })
}

pub fn contains_secret_marker(value: &IoValue) -> Result<bool> {
    let text = to_text(value)?;
    Ok(SENSITIVE_RECORD_LABELS.iter().any(|label| text.contains(&format!("<{label}"))))
}

pub fn redacted_value(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<IoValue> {
    Ok(redacted_view(value, redaction_profile_ref)?.value)
}

pub fn redacted_view(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<RedactedValue> {
    if !contains_secret_marker(value)? {
        return Ok(RedactedValue {
            value: value.clone(),
            marker: None,
            transform_receipt: None,
        });
    }
    let source_ref = canonical_hash(value)?;
    let policy_refs = vec![DEFAULT_REDACTION_POLICY.to_string()];
    let profile_ref = redaction_profile_ref.unwrap_or(DEFAULT_REDACTION_PROFILE).to_string();
    validate_ref(&profile_ref, "redaction profile ref")?;
    let reason = first_redaction_reason(value)?;
    let path_ref = fixture_ref(&format!("redaction-path:{source_ref}"));
    let marker_receipt_ref = redaction_seed_ref(&source_ref, &profile_ref, &policy_refs)?;
    let marker_value = redaction_marker_value(&RedactionMarkerInput {
        reason,
        commitment_ref: source_ref.clone(),
        schema_ref: fixture_ref("redacted-source-schema"),
        path_ref,
        policy_refs: policy_refs.clone(),
        receipt_ref: marker_receipt_ref,
    })?;
    let marker = parse_redaction_marker(&marker_value)?;
    let output_ref = canonical_hash(&marker.value)?;
    let transform_value = redaction_transform_receipt_value(&RedactionTransformInput {
        source_ref,
        output_ref,
        policy_refs,
        profile_ref,
        marker_refs: vec![marker.marker_ref.clone()],
        is_gate_preserving: true,
        diagnostics: Vec::new(),
    })?;
    let transform_receipt = parse_redaction_transform_receipt(&transform_value)?;
    Ok(RedactedValue {
        value: marker.value.clone(),
        marker: Some(marker),
        transform_receipt: Some(transform_receipt),
    })
}

pub fn redacted_text(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<String> {
    to_text(&redacted_value(value, redaction_profile_ref)?)
}

pub fn secrets_summary(value: &IoValue) -> Result<String> {
    let kind = crate::ledger::artifact_kind(value);
    if let Some(line) = summary_core(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_receipts(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_profiles(kind, value)? {
        return Ok(line);
    }
    Err(MoltenError::invalid_harness("not a secrets artifact"))
}
