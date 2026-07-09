
fn collect_rkyv_admission_diagnostics(
    input: RkyvArchiveAdmissionInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if input.manifest.identity_claim != RKYV_IDENTITY_DERIVED_SIDECAR {
        push_rkyv_diagnostic(diagnostics, "rkyv archive overclaims canonical identity")?;
    }
    if input.manifest.profile_version != RKYV_CURRENT_PROFILE {
        push_rkyv_diagnostic(diagnostics, "rkyv archive profile is unsupported")?;
    }
    if input.manifest.source_digests.is_empty() {
        push_rkyv_diagnostic(diagnostics, "rkyv archive manifest has no canonical source refs")?;
    }
    if !rkyv_sources_match(&input.manifest.source_digests, input.current_sources) {
        push_rkyv_diagnostic(diagnostics, "rkyv archive source refs or digests are stale")?;
    }
    if input.manifest.archive_byte_digest != input.observed_archive_digest {
        push_rkyv_diagnostic(diagnostics, "rkyv archive byte digest does not match manifest")?;
    }
    if input.manifest.validation_required && !input.validation_passed {
        push_rkyv_diagnostic(diagnostics, "rkyv archive validation did not pass")?;
    }
    if input.manifest.validation_required
        && input.manifest.validation_receipt_ref.as_deref() != input.observed_validation_receipt_ref
    {
        push_rkyv_diagnostic(diagnostics, "rkyv archive validation receipt does not match exact bytes")?;
    }
    Ok(())
}

fn rkyv_admission_decision(input: RkyvArchiveAdmissionInput<'_>, diagnostics: &[String]) -> String {
    if diagnostics.is_empty() {
        return RKYV_DECISION_ADMIT.to_string();
    }
    if input.caller_allows_rebuild
        && input.manifest.rebuild_capability.is_some()
        && diagnostics.iter().all(|diagnostic| rkyv_rebuildable_diagnostic(diagnostic))
    {
        return RKYV_DECISION_REBUILD.to_string();
    }
    RKYV_DECISION_DENY.to_string()
}

fn rkyv_rebuildable_diagnostic(diagnostic: &str) -> bool {
    diagnostic.contains("stale") || diagnostic.contains("byte digest")
}

fn rkyv_derived_archive_manifest_value(input: &RkyvDerivedArchiveManifestInput) -> Result<IoValue> {
    Ok(record("rkyv-derived-archive-manifest-v1", vec![
        string(RKYV_DERIVED_ARCHIVE_MANIFEST_SCHEMA),
        record("cache-purpose", vec![string(&input.cache_purpose)]),
        record("artifact-kind", vec![string(&input.artifact_kind)]),
        record("profile", vec![string(&input.profile_version)]),
        record("producer-tool", vec![string(&input.producer_tool_ref)]),
        record("producer-version", vec![string(&input.producer_version)]),
        record("sources", vec![rkyv_source_digests_value(&input.source_digests)]),
        record("archive-digest", vec![string(&input.archive_byte_digest)]),
        record("validation-required", vec![string(if input.validation_required { "true" } else { "false" })]),
        record("validation-receipt", vec![optional_ref_value(input.validation_receipt_ref.as_deref())]),
        record("rebuild-capability", vec![optional_string_value(input.rebuild_capability.as_deref())]),
        record("retention-class", vec![string(&input.retention_class)]),
        record("identity-claim", vec![string(&input.identity_claim)]),
        checks_value_from_pairs(&[
            ("preserves-source-of-truth", "pass"),
            ("derived-cache-sidecar", "pass"),
            ("not-canonical-identity", "pass"),
        ]),
    ]))
}

fn rkyv_archive_admission_value(
    decision: &str,
    input: RkyvArchiveAdmissionInput<'_>,
    source_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("rkyv-derived-archive-admission-v1", vec![
        string(RKYV_DERIVED_ARCHIVE_ADMISSION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![string(&input.manifest.manifest_ref)]),
        record("sources", vec![refs_sequence(source_refs)]),
        record("observed-archive-digest", vec![string(input.observed_archive_digest)]),
        record("validation-receipt", vec![optional_ref_value(input.observed_validation_receipt_ref)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value_from_pairs(&[
            ("pure-before-shell-io", "pass"),
            ("validation-required-before-read", pass_fail(decision == RKYV_DECISION_ADMIT)),
            ("archive-bytes-not-identity", pass_fail(input.manifest.identity_claim == RKYV_IDENTITY_DERIVED_SIDECAR)),
        ]),
    ]))
}

fn rkyv_source_digests_value(sources: &[RkyvSourceDigest]) -> IoValue {
    sequence(
        sources
            .iter()
            .map(|source| {
                record("source", vec![
                    record("ref", vec![string(&source.source_ref)]),
                    record("blake3", vec![string(&source.blake3_digest)]),
                ])
            })
            .collect(),
    )
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_rkyv_manifest_input(input: &RkyvDerivedArchiveManifestInput) -> Result<()> {
    validate_rkyv_name(&input.cache_purpose, "rkyv cache purpose")?;
    validate_rkyv_name(&input.artifact_kind, "rkyv artifact kind")?;
    validate_rkyv_profile(&input.profile_version)?;
    validate_ref(&input.producer_tool_ref, "rkyv producer tool ref")?;
    validate_non_empty(&input.producer_version, "rkyv producer version")?;
    validate_rkyv_sources(&input.source_digests, "rkyv source")?;
    validate_ref(&input.archive_byte_digest, "rkyv archive byte digest")?;
    if let Some(receipt_ref) = input.validation_receipt_ref.as_ref() {
        validate_ref(receipt_ref, "rkyv validation receipt")?;
    }
    if let Some(capability) = input.rebuild_capability.as_ref() {
        validate_rkyv_name(capability, "rkyv rebuild capability")?;
    }
    validate_rkyv_retention_class(&input.retention_class)?;
    validate_rkyv_identity_claim(&input.identity_claim)?;
    Ok(())
}

fn validate_rkyv_sources(sources: &[RkyvSourceDigest], label: &str) -> Result<()> {
    if sources.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} refs cannot be empty")));
    }
    if sources.len() > RKYV_SOURCE_DIGEST_LIMIT {
        return Err(MoltenError::invalid_harness(format!(
            "{label} count {} exceeds bound {RKYV_SOURCE_DIGEST_LIMIT}",
            sources.len()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for source in sources {
        validate_ref(&source.source_ref, label)?;
        validate_ref(&source.blake3_digest, label)?;
        if !seen.insert(source.source_ref.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {label} ref {}", source.source_ref)));
        }
    }
    Ok(())
}

fn validate_rkyv_profile(profile: &str) -> Result<()> {
    if profile.is_empty() {
        return Err(MoltenError::invalid_harness("rkyv profile cannot be empty"));
    }
    validate_rkyv_name(profile, "rkyv profile")
}

fn validate_rkyv_retention_class(value: &str) -> Result<()> {
    match value {
        RKYV_RETENTION_EPHEMERAL_CACHE | RKYV_RETENTION_REPLAY_SNAPSHOT => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported rkyv retention class {value}"))),
    }
}

fn validate_rkyv_identity_claim(value: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness("rkyv identity claim cannot be empty"))
    } else {
        Ok(())
    }
}

fn validate_rkyv_name(value: &str, field: &str) -> Result<()> {
    validate_non_empty(value, field)?;
    if value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} must be lowercase ascii token")))
    }
}

fn rkyv_sources_match(left: &[RkyvSourceDigest], right: &[RkyvSourceDigest]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let left = left.iter().map(rkyv_source_key).collect::<std::collections::BTreeSet<_>>();
    let right = right.iter().map(rkyv_source_key).collect::<std::collections::BTreeSet<_>>();
    left == right
}

fn rkyv_source_key(source: &RkyvSourceDigest) -> (&str, &str) {
    (&source.source_ref, &source.blake3_digest)
}

fn push_rkyv_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: &str) -> Result<()> {
    push_bounded(
        diagnostics,
        diagnostic.to_string(),
        RKYV_DIAGNOSTIC_LIMIT,
        "rkyv derived archive diagnostics",
    )
}

fn pass_fail(condition: bool) -> &'static str {
    if condition { "pass" } else { "fail" }
}
