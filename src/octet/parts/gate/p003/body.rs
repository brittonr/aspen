
fn read_import_files(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> InputFiles {
    InputFiles {
        command: read_required_file(
            artifacts_dir,
            COMMAND_NAME,
            "ledger-command-artifact-present",
            checks,
            diagnostics,
        ),
        status_file: read_required_file(
            artifacts_dir,
            STATUS_NAME,
            "ledger-status-artifact-present",
            checks,
            diagnostics,
        ),
        summary: read_required_file(
            artifacts_dir,
            SUMMARY_NAME,
            "ledger-summary-artifact-present",
            checks,
            diagnostics,
        ),
        object_corpus: read_required_file(
            artifacts_dir,
            OBJECT_CORPUS_RECEIPT_NAME,
            "ledger-object-corpus-artifact-present",
            checks,
            diagnostics,
        ),
    }
}

fn raw_values(files: &InputFiles) -> Vec<IoValue> {
    let mut values = Vec::with_capacity(MAX_OCTET_ARTIFACT_VALUES);
    if let Some(command) = files.command.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-command-artifact-v1",
            crate::preserves_rail::OCTET_COMMAND_ARTIFACT_SCHEMA,
            COMMAND_NAME,
            command,
        ));
    }
    if let Some(status_file) = files.status_file.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-status-artifact-v1",
            crate::preserves_rail::OCTET_STATUS_ARTIFACT_SCHEMA,
            STATUS_NAME,
            status_file,
        ));
    }
    if let Some(summary) = files.summary.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-summary-artifact-v1",
            crate::preserves_rail::OCTET_SUMMARY_ARTIFACT_SCHEMA,
            SUMMARY_NAME,
            summary,
        ));
    }
    if let Some(object_corpus) = files.object_corpus.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-object-corpus-artifact-v1",
            crate::preserves_rail::OCTET_OBJECT_CORPUS_ARTIFACT_SCHEMA,
            OBJECT_CORPUS_RECEIPT_NAME,
            object_corpus,
        ));
    }
    values
}

fn add_structured_value(
    values: &mut impl crate::bounded::VecSink<IoValue>,
    files: &InputFiles,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let status = parse_status(files.status_file.as_ref(), checks, diagnostics);
    if let Some((status, status_file, summary)) = status
        .as_ref()
        .zip(files.status_file.as_ref())
        .zip(files.summary.as_ref())
        .map(|((status, status_file), summary)| (status, status_file, summary))
    {
        let (structured, unkeyed) = octet_structured_findings_value(status_file, summary, status);
        if unkeyed == 0 {
            values.push_item(structured);
        } else {
            push_diagnostic(diagnostics, format!("structured findings omitted stable keys for {unkeyed} findings"));
        }
    }
}

fn add_fingerprint_value(
    values: &mut impl crate::bounded::VecSink<IoValue>,
    files: &InputFiles,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if let Some((object_corpus_receipt, object_corpus)) =
        validate_object_corpus(files.object_corpus.as_ref(), checks, diagnostics)
            .as_ref()
            .zip(files.object_corpus.as_ref())
    {
        values.push_item(octet_fingerprint_evidence_value(object_corpus, object_corpus_receipt)?);
    }
    Ok(())
}

pub fn build_octet_review_manifest(input: &OctetReviewManifestInput) -> Result<OctetReviewManifestArtifact> {
    if input.finding_keys.is_empty() {
        return Err(MoltenError::invalid_harness("octet review manifest requires at least one finding key"));
    }
    let review_value = record("octet-review-manifest-v1", vec![
        string(OCTET_REVIEW_MANIFEST_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        record("expires-at", vec![string(&input.expires_at)]),
        record("finding-keys", vec![sequence(input.finding_keys.iter().map(string).collect())]),
        record("rationale", vec![string(&input.rationale)]),
        checks_value(&[
            Check {
                name: "exact-finding-keys",
                status: "pass",
            },
            Check {
                name: "temporary-review",
                status: "pass",
            },
        ]),
    ]);
    let review_ref = canonical_hash(&review_value)?;
    Ok(OctetReviewManifestArtifact {
        review_ref,
        review_value,
    })
}

pub fn check_octet_warning_baseline(input: &OctetBaselineCheckInput) -> Result<OctetBaselineEvaluation> {
    let baseline = parse_warning_baseline(&input.baseline_value)?;
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let run = load_current_octet_run(&input.artifacts_dir, &mut checks, &mut diagnostics)?;
    let reviews = parse_review_manifests(&input.review_values)?;
    let review_refs = reviews.iter().map(|review| review.review_ref.clone()).collect::<Vec<_>>();
    let facts = BaselineFacts {
        has_bound_review_refs: baseline.review_refs.is_empty()
            || baseline
                .review_refs
                .iter()
                .all(|baseline_ref| review_refs.iter().any(|review_ref| review_ref == baseline_ref)),
        is_profile_allowed: baseline.allowed_profiles.iter().any(|profile| profile == &input.profile),
        is_baseline_current: baseline.expires_at.as_str() >= input.as_of.as_str(),
        is_config_current: run.status.metadata.config_hash == baseline.config_hash,
        is_profile_hash_current: run.status.metadata.profile_hash == baseline.profile_hash,
        is_within_shrink_target: run.status.total_findings <= baseline.target_next,
        has_zero_unkeyed_findings: run.unkeyed_findings == 0,
    };
    let new_findings = finding_count_delta(&run.findings, &baseline.findings, DeltaKind::NewOrIncreased);
    let removed_findings = finding_count_delta(&run.findings, &baseline.findings, DeltaKind::Removed);
    let unchanged_findings = finding_intersection(&run.findings, &baseline.findings);
    let critical_unreviewed = run
        .findings
        .values()
        .filter(|finding| is_critical_lint(&finding.lint))
        .filter(|finding| !finding_is_reviewed(finding, &reviews, &input.profile, &input.as_of))
        .cloned()
        .collect::<Vec<_>>();

    push_baseline_checks(&mut checks, &facts, &new_findings, &critical_unreviewed);
    push_baseline_diagnostics(&mut diagnostics, DiagnosticInput {
        input,
        baseline: &baseline,
        run: &run,
        facts: &facts,
        new_findings: &new_findings,
        critical_unreviewed: &critical_unreviewed,
    });

    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = octet_baseline_receipt_value(OctetBaselineReceiptInput {
        decision: &decision,
        baseline_ref: &baseline.baseline_ref,
        status_ref: &run.status_ref,
        new_findings: &new_findings,
        removed_findings: &removed_findings,
        unchanged_findings: &unchanged_findings,
        critical_unreviewed: &critical_unreviewed,
        review_refs: &review_refs,
        expired: !facts.is_baseline_current,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetBaselineEvaluation {
        decision,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn push_baseline_checks(
    checks: &mut impl crate::bounded::VecSink<Check>,
    facts: &BaselineFacts,
    new_findings: &[FindingEntry],
    critical_unreviewed: &[FindingEntry],
) {
    push_check(checks, "baseline-profile-allowed", facts.is_profile_allowed);
    push_check(checks, "baseline-not-expired", facts.is_baseline_current);
    push_check(checks, "baseline-config-current", facts.is_config_current);
    push_check(checks, "baseline-profile-current", facts.is_profile_hash_current);
    push_check(checks, "baseline-no-new-findings", new_findings.is_empty());
    push_check(checks, "baseline-no-unkeyed-findings", facts.has_zero_unkeyed_findings);
    push_check(checks, "baseline-critical-reviewed", critical_unreviewed.is_empty());
    push_check(checks, "baseline-review-refs-bound", facts.has_bound_review_refs);
    push_check(checks, "baseline-shrink-target", facts.is_within_shrink_target);
}

fn push_baseline_diagnostics(diagnostics: &mut impl crate::bounded::VecSink<String>, context: DiagnosticInput<'_>) {
    if !context.facts.is_profile_allowed {
        push_diagnostic(diagnostics, format!("baseline does not allow profile `{}`", context.input.profile));
    }
    if !context.facts.is_baseline_current {
        push_diagnostic(
            diagnostics,
            format!("octet warning baseline expired at {} as_of {}", context.baseline.expires_at, context.input.as_of),
        );
    }
    if !context.facts.is_config_current {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline config hash mismatch: baseline={} current={}",
                context.baseline.config_hash, context.run.status.metadata.config_hash
            ),
        );
    }
    if !context.facts.is_profile_hash_current {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline profile hash mismatch: baseline={} current={}",
                context.baseline.profile_hash, context.run.status.metadata.profile_hash
            ),
        );
    }
    if !context.new_findings.is_empty() {
        push_diagnostic(diagnostics, format!("new or increased octet findings: {}", context.new_findings.len()));
    }
    if !context.facts.has_zero_unkeyed_findings {
        push_diagnostic(diagnostics, format!("unkeyed octet findings: {}", context.run.unkeyed_findings));
    }
    if !context.critical_unreviewed.is_empty() {
        push_diagnostic(
            diagnostics,
            format!("unreviewed critical baseline findings: {}", context.critical_unreviewed.len()),
        );
    }
    if !context.facts.has_bound_review_refs {
        push_diagnostic(diagnostics, "baseline references review manifests not supplied to check".to_string());
    }
    if !context.facts.is_within_shrink_target {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline burn-down target exceeded: current={} target={}",
                context.run.status.total_findings, context.baseline.target_next
            ),
        );
    }
}
