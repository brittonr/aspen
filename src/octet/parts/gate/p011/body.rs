
fn octet_warning_baseline_value(input: &OctetWarningBaselineValueInput<'_>) -> IoValue {
    let critical_keys = critical_keys(&input.run.findings);
    record("octet-warning-baseline-v1", vec![
        string(OCTET_WARNING_BASELINE_SCHEMA),
        record("scope", vec![string("workspace")]),
        record("created-at", vec![string(input.created_at)]),
        record("expires-at", vec![string(input.expires_at)]),
        record("octet-config-hash", vec![string(&input.run.status.metadata.config_hash)]),
        record("octet-profile-hash", vec![string(&input.run.status.metadata.profile_hash)]),
        record("toolchain", vec![string(&input.run.status.metadata.toolchain)]),
        record("source-snapshot", vec![string(input.source_snapshot_ref)]),
        record("finding-keys", vec![sequence(input.run.findings.values().map(finding_entry_value).collect())]),
        record("critical-finding-keys", vec![sequence(critical_keys.iter().map(string).collect())]),
        record("allowed-profiles", vec![sequence(vec![string(QUARANTINE_PROFILE)])]),
        record("burn-down", vec![
            record("total", vec![u64_value(input.run.status.total_findings)]),
            record("target-next", vec![u64_value(input.target_next)]),
            record("deadline", vec![string(input.expires_at)]),
        ]),
        record("review-refs", vec![sequence(Vec::new())]),
        checks_value(input.checks),
    ])
}

fn octet_baseline_receipt_value(input: OctetBaselineReceiptInput<'_>) -> IoValue {
    record("octet-baseline-receipt-v1", vec![
        string(crate::preserves_rail::OCTET_BASELINE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("baseline", vec![string(input.baseline_ref)]),
        record("run-status", vec![string(input.status_ref)]),
        record("new-findings", vec![sequence(input.new_findings.iter().map(finding_entry_value).collect())]),
        record("removed-findings", vec![sequence(
            input.removed_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("unchanged-findings", vec![sequence(
            input.unchanged_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("critical-unreviewed", vec![sequence(
            input.critical_unreviewed.iter().map(finding_entry_value).collect(),
        )]),
        record("review-refs", vec![sequence(input.review_refs.iter().map(string).collect())]),
        record("expired", vec![bool_value(input.expired)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn parse_review_manifests(values: &[IoValue]) -> Result<Vec<ParsedReviewManifest>> {
    values.iter().map(parse_review_manifest).collect()
}
