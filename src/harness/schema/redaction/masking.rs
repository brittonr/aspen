use preserves::ValueImpl;

pub struct ProfiledTransformOutput {
    pub(super) report_value: preserves::IOValue,
    pub(super) manifest_ref: String,
    pub(super) manifest_value: preserves::IOValue,
    pub(super) receipt_ref: String,
    pub(super) receipt_value: preserves::IOValue,
    pub(super) encrypted_refs: Vec<String>,
}

pub(super) struct RedactionTransformState {
    profile: super::model::ReproExportProfile,
    policy_ref: String,
    marker_refs: Vec<String>,
    marker_entries: Vec<RedactionManifestEntry>,
    encrypted_refs: Vec<String>,
}

pub(super) struct RedactionManifestEntry {
    pub(super) path: String,
    pub(super) reason: String,
    pub(super) commitment_ref: String,
    pub(super) marker_ref: Option<String>,
    pub(super) encrypted_ref: Option<String>,
}

pub(super) fn redacted_report_for_profile(
    report_value: &preserves::IOValue,
    report: &super::model::Report,
    profile: super::model::ReproExportProfile,
    policy_ref: &str,
) -> crate::error::Result<ProfiledTransformOutput> {
    let mut state = RedactionTransformState {
        profile,
        policy_ref: policy_ref.to_string(),
        marker_refs: Vec::new(),
        marker_entries: Vec::new(),
        encrypted_refs: Vec::new(),
    };
    let redacted_report_value = rebind_report_suite_ref(&transform_sensitive_value(report_value, "/", &mut state)?)?;
    state.marker_refs.sort();
    state.marker_refs.dedup();
    state.encrypted_refs.sort();
    state.encrypted_refs.dedup();
    let redacted_report = super::reading::parse_report(&redacted_report_value)?;
    super::audience::validate_profiled_output(&redacted_report_value, profile)?;
    let manifest_value = super::ledgering::redaction_transform_manifest_value(
        report,
        &redacted_report,
        profile,
        &state.marker_entries,
        &state.encrypted_refs,
    );
    let manifest_ref = crate::preserves_rail::canonical_hash(&manifest_value)?;
    let receipt_value =
        super::ledgering::redaction_transform_receipt_value(&super::ledgering::RedactionTransformReceiptInput {
            source_report_ref: &report.report_ref,
            source_suite_ref: &report.suite_ref,
            policy_ref,
            profile,
            manifest_ref: &manifest_ref,
            output_bundle_ref: &redacted_report.report_ref,
            marker_refs: &state.marker_refs,
            encrypted_refs: &state.encrypted_refs,
        })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ProfiledTransformOutput {
        report_value: redacted_report_value,
        manifest_ref,
        manifest_value,
        receipt_ref,
        receipt_value,
        encrypted_refs: state.encrypted_refs,
    })
}

fn rebind_report_suite_ref(report_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    let report = super::fields::simple_record(report_value, "harness-report-v1", 17)
        .or_else(|_| super::fields::simple_record(report_value, "harness-report-v1", 16))
        .or_else(|_| super::fields::simple_record(report_value, "harness-report-v1", 15))
        .or_else(|_| super::fields::simple_record(report_value, "harness-report-v1", 14))
        .or_else(|_| super::fields::simple_record(report_value, "harness-report-v1", 13))?;
    let suite_value = crate::preserves_rail::value_to_iovalue(&report[8]);
    let suite_ref = crate::preserves_rail::canonical_hash(&suite_value)?;
    let field_count = report.fields_iter().count();
    let mut fields = Vec::with_capacity(field_count);
    for (index, field) in report.fields_iter().enumerate() {
        if index == 5 {
            fields.push(crate::preserves_rail::string(&suite_ref));
        } else {
            fields.push(crate::preserves_rail::value_to_iovalue(field));
        }
    }
    Ok(crate::preserves_rail::record("harness-report-v1", fields))
}

fn transform_sensitive_value(
    value: &preserves::IOValue,
    path: &str,
    state: &mut RedactionTransformState,
) -> crate::error::Result<preserves::IOValue> {
    super::scrub::RedactionTraversal::new(state).run(value, path)
}

pub(super) fn transform_sensitive_record(
    value: &preserves::IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> crate::error::Result<preserves::IOValue> {
    if label == "encrypted-ref" {
        return Err(crate::error::Failure::invalid_harness(
            "malformed encrypted-ref marker cannot be accepted into a repro export profile",
        ));
    }
    if label == "encrypted-ref-v1" {
        let encrypted = crate::secrets::parse_encrypted_ref(value)?;
        if state.profile != super::model::ReproExportProfile::EncryptedPrivate {
            return redaction_marker_for_value(value, label, path, state);
        }
        state.encrypted_refs.push(encrypted.encrypted_ref);
        return Ok(value.clone());
    }
    match state.profile {
        super::model::ReproExportProfile::DenySensitive => Err(crate::error::Failure::invalid_harness(format!(
            "redaction preflight found sensitive marker {label}; sealed pass repro bundles require explicit redaction before export"
        ))),
        super::model::ReproExportProfile::RedactedDiagnostic => redaction_marker_for_value(value, label, path, state),
        super::model::ReproExportProfile::EncryptedPrivate => encrypted_ref_for_value(value, label, path, state),
    }
}

fn redaction_marker_for_value(
    value: &preserves::IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> crate::error::Result<preserves::IOValue> {
    let commitment_ref = crate::preserves_rail::canonical_hash(value)?;
    let path_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(path))?;
    let receipt_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("redaction-marker-seed", vec![
            crate::preserves_rail::string(&commitment_ref),
            crate::preserves_rail::string(&path_ref),
            crate::preserves_rail::string(&state.policy_ref),
        ]))?;
    let marker_value = crate::secrets::redaction_marker_value(&crate::secrets::RedactionMarkerInput {
        reason: label.to_string(),
        commitment_ref: commitment_ref.clone(),
        schema_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label))?,
        path_ref,
        policy_refs: vec![state.policy_ref.clone()],
        receipt_ref,
    })?;
    let marker = crate::secrets::parse_redaction_marker(&marker_value)?;
    state.marker_refs.push(marker.marker_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: Some(marker.marker_ref),
        encrypted_ref: None,
    });
    Ok(marker_value)
}

fn encrypted_ref_for_value(
    value: &preserves::IOValue,
    label: &str,
    path: &str,
    state: &mut RedactionTransformState,
) -> crate::error::Result<preserves::IOValue> {
    let commitment_ref = crate::preserves_rail::canonical_hash(value)?;
    let ciphertext_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("encrypted-redaction-ciphertext", vec![
            crate::preserves_rail::string(&commitment_ref),
            crate::preserves_rail::string(path),
        ]))?;
    let encrypted_value = crate::secrets::encrypted_ref_value(&crate::secrets::EncryptedRefInput {
        ciphertext_ref,
        commitment_ref: commitment_ref.clone(),
        encryption_ref: crate::preserves_rail::canonical_hash(&super::audience::repro_export_profile_value(
            state.profile,
        ))?,
        schema_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label))?,
        policy_refs: vec![state.policy_ref.clone()],
        evidence_refs: vec![crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(
            path,
        ))?],
    })?;
    let encrypted = crate::secrets::parse_encrypted_ref(&encrypted_value)?;
    state.encrypted_refs.push(encrypted.encrypted_ref.clone());
    state.marker_entries.push(RedactionManifestEntry {
        path: path.to_string(),
        reason: label.to_string(),
        commitment_ref,
        marker_ref: None,
        encrypted_ref: Some(encrypted.encrypted_ref),
    });
    Ok(encrypted_value)
}

pub(super) fn record_label_string(value: &preserves::IOValue) -> Option<String> {
    if !value.is_record() {
        return None;
    }
    value.label().as_symbol().map(std::borrow::Cow::into_owned)
}
