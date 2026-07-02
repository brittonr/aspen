
fn validate_member(member: &Member, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_text(&member.name, "member name", MAX_MEMBER_NAME_BYTES, diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&member.object_ref), "member object", diagnostics)?;
    if let Some(mime) = &member.mime_hint {
        validate_text(mime, "MIME hint", MAX_MIME_BYTES, diagnostics)?;
    }
    Ok(())
}

fn collect_visibility_diagnostics(visibility: &Visibility, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    if !matches!(visibility.profile.as_str(), PUBLIC_PROFILE | DIAGNOSTIC_PROFILE | INTERNAL_PROFILE) {
        push_diagnostic(diagnostics, "unsupported gateway visibility profile")?;
    }
    collect_ref_diagnostics(&visibility.visibility_policy_refs, "visibility policy", diagnostics)?;
    collect_ref_diagnostics(&visibility.retention_refs, "retention", diagnostics)?;
    collect_ref_diagnostics(&visibility.reveal_refs, "reveal", diagnostics)?;
    collect_ref_diagnostics(&visibility.redaction_refs, "redaction", diagnostics)?;
    collect_ref_diagnostics(&visibility.hidden_refs, "hidden", diagnostics)?;
    if visibility.visibility_policy_refs.is_empty() {
        push_diagnostic(diagnostics, "gateway visibility policy refs are required")?;
    }
    Ok(())
}

fn collect_ref_diagnostics(refs: &[String], label: &str, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_count(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        if let Err(error) = validate_content_ref(reference) {
            push_diagnostic(diagnostics, format!("invalid {label} ref: {error}"))?;
        }
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, maximum: usize, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    if value.trim().is_empty() {
        return push_diagnostic(diagnostics, format!("{label} must not be empty"));
    }
    if value.len() > maximum {
        return push_diagnostic(diagnostics, format!("{label} length {} exceeds bound {maximum}", value.len()));
    }
    Ok(())
}

fn validate_count(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn push_diagnostic(diagnostics: &mut impl DiagnosticSink, diagnostic: impl Into<String>) -> Result<()> {
    diagnostics.push_bounded(diagnostic.into())
}

fn refs_value(refs: &[String]) -> Result<IoValue> {
    validate_count(refs.len(), MAX_REFS, "gateway ref")?;
    Ok(sequence(refs.iter().map(string).collect()))
}

fn strings_value(values: &[String]) -> Result<IoValue> {
    validate_count(values.len(), MAX_DIAGNOSTICS, "gateway string")?;
    Ok(sequence(values.iter().map(string).collect()))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}
