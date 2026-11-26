pub(super) struct RedactionTransformReceiptInput<'a> {
    pub(super) source_report_ref: &'a str,
    pub(super) source_suite_ref: &'a str,
    pub(super) policy_ref: &'a str,
    pub(super) profile: super::model::ReproExportProfile,
    pub(super) manifest_ref: &'a str,
    pub(super) output_bundle_ref: &'a str,
    pub(super) marker_refs: &'a [String],
    pub(super) encrypted_refs: &'a [String],
}

pub(super) struct RedactionTransformReceiptEvidence {
    pub(super) receipt_ref: String,
    pub(super) source_report_ref: String,
    pub(super) source_suite_ref: String,
    pub(super) policy_ref: String,
    pub(super) profile: super::model::ReproExportProfile,
    pub(super) manifest_ref: String,
    pub(super) output_bundle_ref: String,
    pub(super) loss_classification: String,
    pub(super) marker_refs: Vec<String>,
    pub(super) encrypted_refs: Vec<String>,
    pub(super) value: preserves::IOValue,
}

pub(super) fn redaction_transform_manifest_value(
    source_report: &super::model::Report,
    output_report: &super::model::Report,
    profile: super::model::ReproExportProfile,
    entries: &[super::masking::RedactionManifestEntry],
    encrypted_refs: &[String],
) -> preserves::IOValue {
    crate::preserves_rail::record("redaction-transform-manifest-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA),
        crate::preserves_rail::record("source-report", vec![crate::preserves_rail::string(&source_report.report_ref)]),
        crate::preserves_rail::record("source-suite", vec![crate::preserves_rail::string(&source_report.suite_ref)]),
        crate::preserves_rail::record("output-report", vec![crate::preserves_rail::string(&output_report.report_ref)]),
        crate::preserves_rail::record("output-suite", vec![crate::preserves_rail::string(&output_report.suite_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(profile.as_str())]),
        crate::preserves_rail::record("markers", vec![crate::preserves_rail::sequence(
            entries
                .iter()
                .map(|entry| {
                    crate::preserves_rail::record("redaction", vec![
                        crate::preserves_rail::string(&entry.path),
                        crate::preserves_rail::string(&entry.reason),
                        crate::preserves_rail::string(&entry.commitment_ref),
                        optional_ref_value(entry.marker_ref.as_deref()),
                        optional_ref_value(entry.encrypted_ref.as_deref()),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("encrypted-refs", vec![refs_sequence(encrypted_refs)]),
        checks_value_for_names(&[
            "source-report-bound",
            "output-report-bound",
            "deterministic-traversal-order",
            "marker-coverage-manifest",
            "encrypted-ref-inventory",
        ]),
    ])
}

pub(super) fn redaction_transform_receipt_value(
    input: &RedactionTransformReceiptInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("redaction-transform-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("source-report", vec![crate::preserves_rail::string(input.source_report_ref)]),
        crate::preserves_rail::record("source-suite", vec![crate::preserves_rail::string(input.source_suite_ref)]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::string(input.policy_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(input.profile.as_str())]),
        crate::preserves_rail::record("transform-manifest", vec![crate::preserves_rail::string(input.manifest_ref)]),
        crate::preserves_rail::record("output-bundle", vec![crate::preserves_rail::string(input.output_bundle_ref)]),
        crate::preserves_rail::record("loss-classification", vec![crate::preserves_rail::string(
            input.profile.loss_classification(),
        )]),
        crate::preserves_rail::record("markers", vec![refs_sequence(input.marker_refs)]),
        crate::preserves_rail::record("encrypted-refs", vec![refs_sequence(input.encrypted_refs)]),
        checks_value_for_names(&redaction_transform_check_names(input.profile)),
    ]))
}

pub(super) fn parse_redaction_transform_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<RedactionTransformReceiptEvidence> {
    let receipt = super::fields::simple_record(value, "redaction-transform-receipt-v1", 12)?;
    let schema = super::fields::required_string(&receipt[0], "redaction transform schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported redaction transform schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_RECEIPT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "redaction transform decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported redaction transform decision {decision}"
        )));
    }
    let source_report_ref =
        super::fields::required_record_hash(&receipt[2], "source-report", "redaction source report")?;
    let source_suite_ref = super::fields::required_record_hash(&receipt[3], "source-suite", "redaction source suite")?;
    let policy_ref = super::fields::required_record_hash(&receipt[4], "policy", "redaction policy")?;
    let profile_name = super::fields::required_record_string(&receipt[5], "profile", "redaction profile")?;
    let profile = super::model::ReproExportProfile::parse(&profile_name)?;
    let manifest_ref =
        super::fields::required_record_hash(&receipt[6], "transform-manifest", "redaction transform manifest")?;
    let output_bundle_ref =
        super::fields::required_record_hash(&receipt[7], "output-bundle", "redaction output bundle")?;
    let loss_classification =
        super::fields::required_record_string(&receipt[8], "loss-classification", "redaction loss classification")?;
    if loss_classification != profile.loss_classification() {
        return Err(crate::error::Failure::invalid_harness("redaction transform loss classification is not canonical"));
    }
    let marker_refs = super::fields::required_record_hash_sequence(&receipt[9], "markers", "redaction marker refs")?;
    let encrypted_refs =
        super::fields::required_record_hash_sequence(&receipt[10], "encrypted-refs", "redaction encrypted refs")?;
    let checks = super::disclosure::parse_redaction_gate_checks(&receipt[11])?;
    for check in redaction_transform_check_names(profile) {
        super::disclosure::require_redaction_check(&checks, check)?;
    }
    Ok(RedactionTransformReceiptEvidence {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        source_report_ref,
        source_suite_ref,
        policy_ref,
        profile,
        manifest_ref,
        output_bundle_ref,
        loss_classification,
        marker_refs,
        encrypted_refs,
        value: value.clone(),
    })
}

fn refs_sequence(refs: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> preserves::IOValue {
    match reference {
        Some(reference) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

pub(super) fn checks_value_for_names(names: &[&str]) -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        names
            .iter()
            .map(|name| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(*name),
                    crate::preserves_rail::string("pass"),
                ])
            })
            .collect(),
    )])
}

fn redaction_transform_check_names(profile: super::model::ReproExportProfile) -> Vec<&'static str> {
    let mut checks = vec![
        "source-report-ref-bound",
        "source-suite-ref-bound",
        "policy-ref-bound",
        "profile-ref-bound",
        "transform-manifest-bound",
        "output-bundle-ref-bound",
        "marker-coverage",
        "deterministic-traversal-order",
        "forbidden-cleartext-absent",
    ];
    match profile {
        super::model::ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        super::model::ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        super::model::ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    checks
}
