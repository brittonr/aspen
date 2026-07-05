pub fn requirements_from_sources(sources: &[SpecSource]) -> Result<Vec<RequirementInput>> {
    if sources.len() > MAX_REQUIREMENTS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability source count {} exceeds bound {MAX_REQUIREMENTS}",
            sources.len()
        )));
    }
    let mut requirements = OrderedMap::new();
    for source in sources {
        validate_text("source", &source.source)?;
        validate_kind(&source.default_kind)?;
        for id in extract_requirement_ids(&source.markdown)? {
            let requirement = RequirementInput {
                id: id.clone(),
                source: source.source.clone(),
                kind: source.default_kind.clone(),
                changed: source.changed,
            };
            requirements.entry(id).or_insert(requirement);
        }
    }
    Ok(requirements.into_values().collect())
}

pub fn build_traceability_manifest(input: &TraceabilityInput) -> Result<TraceabilityManifest> {
    let requirement_map = requirement_map(&input.requirements)?;
    let mut coverage_map = coverage_map(&input.coverage)?;
    let mut entries = Vec::with_capacity(requirement_map.len());
    for requirement in requirement_map.values() {
        let coverage = coverage_map.remove(&requirement.id);
        entries.push(entry_for_requirement(requirement, coverage.as_ref(), input.require_receipt_backed)?);
    }
    for coverage in coverage_map.into_values() {
        entries.push(stale_coverage_entry(&coverage)?);
    }
    entries.sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
    let summary = summarize_entries(&entries)?;
    let decision = traceability_decision(&summary).to_string();
    let value = manifest_value(&decision, &entries, &summary, input.require_receipt_backed)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TraceabilityManifest {
        decision,
        entries,
        summary,
        manifest_ref,
        value,
    })
}

pub fn traceability_gate_value(manifest: &TraceabilityManifest) -> Result<IoValue> {
    crate::preserves_rail::validate_content_ref(&manifest.manifest_ref)?;
    validate_decision(&manifest.decision)?;
    Ok(record("requirement-traceability-gate-v1", vec![
        string(TRACEABILITY_GATE_SCHEMA),
        record("decision", vec![string(&manifest.decision)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("summary", vec![summary_value(&manifest.summary)?]),
        record("checks", vec![sequence(vec![
            check_value("positive-coverage-recorded", status(manifest.summary.missing_positive.is_empty())),
            check_value("negative-coverage-recorded", status(manifest.summary.missing_negative.is_empty())),
            check_value("stale-references-denied", status(manifest.summary.stale_reference.is_empty())),
            check_value("raw-coverage-claims-labeled", "pass"),
            check_value("documentation-exemptions-explicit", "pass"),
        ])]),
    ]))
}

pub fn render_summary(summary: &TraceabilitySummary) -> Result<String> {
    let groups = [
        ("covered", summary.covered.as_slice()),
        ("exempt", summary.exempt.as_slice()),
        ("missing-positive", summary.missing_positive.as_slice()),
        ("missing-negative", summary.missing_negative.as_slice()),
        ("stale-reference", summary.stale_reference.as_slice()),
        ("unsupported", summary.unsupported.as_slice()),
        ("compatibility-only", summary.compatibility_only.as_slice()),
    ];
    ensure_count_at_most(groups.len(), MAX_SUMMARY_LINES, "traceability summary groups")?;
    let mut lines = Vec::with_capacity(groups.len());
    for (label, ids) in groups {
        lines.push_limited(render_group_line(label, ids), MAX_SUMMARY_LINES, "traceability summary lines")?;
    }
    Ok(lines.join("\n"))
}

pub fn compatibility_evidence(
    target: String,
    command: String,
    artifact_ref: String,
    target_exists: bool,
) -> VerificationEvidence {
    VerificationEvidence {
        artifact_refs: vec![artifact_ref.clone()],
        target,
        command,
        artifact_ref,
        target_exists,
        artifact_present: true,
        source: "compatibility".to_string(),
        receipt_ref: None,
        expected_decision: "compatibility".to_string(),
    }
}

pub fn build_verification_run_receipt(input: &VerificationRunInput) -> Result<VerificationRunReceipt> {
    validate_verification_run_input(input)?;
    let mut diagnostics = verification_run_diagnostics(input)?;
    diagnostics.sort();
    let decision = if diagnostics.is_empty() {
        expected_decision(&input.coverage_kind)?
    } else {
        "deny"
    }
    .to_string();
    let value = verification_run_receipt_value(input, &decision, &diagnostics)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(VerificationRunReceipt {
        decision,
        requirement_id: input.requirement_id.clone(),
        coverage_kind: input.coverage_kind.clone(),
        target: input.target.clone(),
        argv: input.argv.clone(),
        profile_ref: input.profile_ref.clone(),
        toolchain_refs: input.toolchain_refs.clone(),
        exit_status: input.exit_status,
        stdout_ref: input.stdout_ref.clone(),
        stderr_ref: input.stderr_ref.clone(),
        artifact_refs: input.artifact_refs.clone(),
        diagnostics,
        receipt_ref,
        value,
    })
}

