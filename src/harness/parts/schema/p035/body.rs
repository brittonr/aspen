
fn redaction_transform_manifest_value(
    source_report: &Report,
    output_report: &Report,
    profile: ReproExportProfile,
    entries: &[RedactionManifestEntry],
    encrypted_refs: &[String],
) -> IoValue {
    record("redaction-transform-manifest-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA),
        record("source-report", vec![string(&source_report.report_ref)]),
        record("source-suite", vec![string(&source_report.suite_ref)]),
        record("output-report", vec![string(&output_report.report_ref)]),
        record("output-suite", vec![string(&output_report.suite_ref)]),
        record("profile", vec![string(profile.as_str())]),
        record("markers", vec![sequence(
            entries
                .iter()
                .map(|entry| {
                    record("redaction", vec![
                        string(&entry.path),
                        string(&entry.reason),
                        string(&entry.commitment_ref),
                        optional_ref_value(entry.marker_ref.as_deref()),
                        optional_ref_value(entry.encrypted_ref.as_deref()),
                    ])
                })
                .collect(),
        )]),
        record("encrypted-refs", vec![refs_sequence(encrypted_refs)]),
        checks_value_for_names(&[
            "source-report-bound",
            "output-report-bound",
            "deterministic-traversal-order",
            "marker-coverage-manifest",
            "encrypted-ref-inventory",
        ]),
    ])
}

fn redaction_transform_receipt_value(input: &RedactionTransformReceiptInput<'_>) -> Result<IoValue> {
    Ok(record("redaction-transform-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("source-report", vec![string(input.source_report_ref)]),
        record("source-suite", vec![string(input.source_suite_ref)]),
        record("policy", vec![string(input.policy_ref)]),
        record("profile", vec![string(input.profile.as_str())]),
        record("transform-manifest", vec![string(input.manifest_ref)]),
        record("output-bundle", vec![string(input.output_bundle_ref)]),
        record("loss-classification", vec![string(input.profile.loss_classification())]),
        record("markers", vec![refs_sequence(input.marker_refs)]),
        record("encrypted-refs", vec![refs_sequence(input.encrypted_refs)]),
        checks_value_for_names(&redaction_transform_check_names(input.profile)),
    ]))
}
