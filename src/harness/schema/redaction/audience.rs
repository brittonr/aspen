use preserves::ValueImpl;

const MAX_REDACTION_MARKER_REFS: usize = 100_000;

const MAX_REDACTION_ENCRYPTED_REFS: usize = 100_000;

const _: () = assert!(MAX_REDACTION_MARKER_REFS <= 1_000_000);

const _: () = assert!(MAX_REDACTION_ENCRYPTED_REFS <= 1_000_000);

pub(super) fn validate_redaction_transform_manifest(
    value: &preserves::IOValue,
    source_report_ref: &str,
    source_suite_ref: &str,
    report: &super::model::Report,
    profile: super::model::ReproExportProfile,
) -> crate::error::Result<()> {
    let manifest = super::fields::simple_record(value, "redaction-transform-manifest-v1", 9)?;
    let schema = super::fields::required_string(&manifest[0], "redaction transform manifest schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported redaction transform manifest schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_TRANSFORM_MANIFEST_SCHEMA
        )));
    }
    let manifest_source_report =
        super::fields::required_record_hash(&manifest[1], "source-report", "manifest source report")?;
    let manifest_source_suite =
        super::fields::required_record_hash(&manifest[2], "source-suite", "manifest source suite")?;
    let manifest_output_report =
        super::fields::required_record_hash(&manifest[3], "output-report", "manifest output report")?;
    let manifest_output_suite =
        super::fields::required_record_hash(&manifest[4], "output-suite", "manifest output suite")?;
    let manifest_profile = super::fields::required_record_string(&manifest[5], "profile", "manifest profile")?;
    if manifest_source_report != source_report_ref
        || manifest_source_suite != source_suite_ref
        || manifest_output_report != report.report_ref
        || manifest_output_suite != report.suite_ref
        || manifest_profile != profile.as_str()
    {
        return Err(crate::error::Failure::invalid_harness("redaction transform manifest binding mismatch"));
    }
    super::manifest::validate_sequence_record(&manifest[6], "markers", "redaction transform manifest markers")?;
    let manifest_encrypted_refs =
        super::fields::required_record_hash_sequence(&manifest[7], "encrypted-refs", "manifest encrypted refs")?;
    if profile == super::model::ReproExportProfile::EncryptedPrivate && manifest_encrypted_refs.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "encrypted-private repro bundle transform manifest missing encrypted refs",
        ));
    }
    let checks = super::disclosure::parse_redaction_gate_checks(&manifest[8])?;
    super::disclosure::require_redaction_check(&checks, "source-report-bound")?;
    super::disclosure::require_redaction_check(&checks, "output-report-bound")?;
    super::disclosure::require_redaction_check(&checks, "deterministic-traversal-order")?;
    super::disclosure::require_redaction_check(&checks, "marker-coverage-manifest")?;
    super::disclosure::require_redaction_check(&checks, "encrypted-ref-inventory")?;
    Ok(())
}

pub(super) fn collect_redaction_marker_refs(value: &preserves::IOValue) -> crate::error::Result<Vec<String>> {
    let mut refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.collect_simple_record("redaction-marker-v1", None).is_some() {
            super::scrub::ensure_redaction_bound(refs.len() + 1, MAX_REDACTION_MARKER_REFS, "redaction marker refs")?;
            refs.push(crate::secrets::parse_redaction_marker(&current)?.marker_ref);
            continue;
        }
        match current.value_class() {
            preserves::ValueClass::Atomic(_) | preserves::ValueClass::Embedded => {}
            preserves::ValueClass::Compound(preserves::CompoundClass::Record)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Sequence)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&child));
                }
            }
            preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&key));
                    stack.push(crate::preserves_rail::value_to_iovalue(&value));
                }
            }
        }
    }
    refs.sort();
    refs.dedup();
    Ok(refs)
}

pub(super) struct ReproExportProfileEvidence {
    pub(super) profile: super::model::ReproExportProfile,
    pub(super) profile_ref: String,
    pub(super) loss_classification: String,
    pub(super) is_gate_preserving: bool,
    pub(super) requires_reveal: bool,
}

pub(super) fn repro_export_profile_value(profile: super::model::ReproExportProfile) -> preserves::IOValue {
    crate::preserves_rail::record("repro-export-profile-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA),
        crate::preserves_rail::record("name", vec![crate::preserves_rail::string(profile.as_str())]),
        crate::preserves_rail::record("loss-classification", vec![crate::preserves_rail::string(
            profile.loss_classification(),
        )]),
        crate::preserves_rail::record("gate-preserving", vec![crate::preserves_rail::bool_value(
            profile.is_gate_preserving(),
        )]),
        crate::preserves_rail::record("requires-reveal", vec![crate::preserves_rail::bool_value(
            profile.requires_reveal(),
        )]),
        super::ledgering::checks_value_for_names(&[
            "explicit-export-profile",
            "loss-classification-bound",
            "gate-preserving-bound",
            "reveal-requirement-bound",
        ]),
    ])
}

pub(super) fn parse_repro_export_profile(
    value: &preserves::IOValue,
) -> crate::error::Result<ReproExportProfileEvidence> {
    let profile_value = super::fields::simple_record(value, "repro-export-profile-v1", 6)?;
    let schema = super::fields::required_string(&profile_value[0], "repro export profile schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported repro export profile schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_PROFILE_SCHEMA
        )));
    }
    let name = super::fields::required_record_string(&profile_value[1], "name", "repro export profile name")?;
    let profile = super::model::ReproExportProfile::parse(&name)?;
    let loss_classification = super::fields::required_record_string(
        &profile_value[2],
        "loss-classification",
        "repro export loss classification",
    )?;
    if loss_classification != profile.loss_classification() {
        return Err(crate::error::Failure::invalid_harness(
            "repro export profile loss classification is not canonical",
        ));
    }
    let is_gate_preserving =
        super::fields::required_record_bool(&profile_value[3], "gate-preserving", "repro export gate preserving flag")?;
    if is_gate_preserving != profile.is_gate_preserving() {
        return Err(crate::error::Failure::invalid_harness("repro export gate-preserving flag is not canonical"));
    }
    let is_requires_reveal =
        super::fields::required_record_bool(&profile_value[4], "requires-reveal", "repro export reveal flag")?;
    if is_requires_reveal != profile.requires_reveal() {
        return Err(crate::error::Failure::invalid_harness("repro export reveal flag is not canonical"));
    }
    let checks = super::disclosure::parse_redaction_gate_checks(&profile_value[5])?;
    super::disclosure::require_redaction_check(&checks, "explicit-export-profile")?;
    super::disclosure::require_redaction_check(&checks, "loss-classification-bound")?;
    super::disclosure::require_redaction_check(&checks, "gate-preserving-bound")?;
    super::disclosure::require_redaction_check(&checks, "reveal-requirement-bound")?;
    Ok(ReproExportProfileEvidence {
        profile,
        profile_ref: crate::preserves_rail::canonical_hash(value)?,
        loss_classification,
        is_gate_preserving,
        requires_reveal: is_requires_reveal,
    })
}

pub(super) fn profiled_repro_checks_value(profile: super::model::ReproExportProfile) -> preserves::IOValue {
    let mut checks = vec![
        "profile-schema",
        "redaction-transform-receipt",
        "transform-manifest-bound",
        "source-report-ref-binding",
        "output-report-ref-binding",
        "no-forbidden-cleartext",
    ];
    match profile {
        super::model::ReproExportProfile::DenySensitive => checks.push("gate-preserving"),
        super::model::ReproExportProfile::RedactedDiagnostic => checks.push("diagnostic-only"),
        super::model::ReproExportProfile::EncryptedPrivate => {
            checks.push("requires-reveal");
            checks.push("encrypted-ref-validation");
        }
    }
    crate::preserves_rail::record("seal-checks", vec![crate::preserves_rail::sequence(
        checks
            .as_slice()
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

pub(super) fn validate_profiled_output(
    value: &preserves::IOValue,
    profile: super::model::ReproExportProfile,
) -> crate::error::Result<Vec<String>> {
    let mut encrypted_refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol() {
                let label = label.as_ref();
                if matches!(label, "secret" | "confidential" | "credential" | "private" | "secret-ref-v1") {
                    return Err(crate::error::Failure::invalid_harness(format!(
                        "redaction transform missed sensitive marker {label}"
                    )));
                }
                if label == "encrypted-ref" {
                    return Err(crate::error::Failure::invalid_harness(
                        "malformed encrypted-ref marker in redacted repro bundle",
                    ));
                }
                if label == "encrypted-ref-v1" {
                    if profile != super::model::ReproExportProfile::EncryptedPrivate {
                        return Err(crate::error::Failure::invalid_harness(
                            "encrypted refs are allowed only in encrypted-private repro bundles",
                        ));
                    }
                    let encrypted = crate::secrets::parse_encrypted_ref(&current)?;
                    super::scrub::ensure_redaction_bound(
                        encrypted_refs.len() + 1,
                        MAX_REDACTION_ENCRYPTED_REFS,
                        "redaction encrypted refs",
                    )?;
                    encrypted_refs.push(encrypted.encrypted_ref);
                    continue;
                }
            }
            stack.push(crate::preserves_rail::value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            preserves::ValueClass::Atomic(_) | preserves::ValueClass::Embedded => {}
            preserves::ValueClass::Compound(preserves::CompoundClass::Record)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Sequence)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&child));
                }
            }
            preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&key));
                    stack.push(crate::preserves_rail::value_to_iovalue(&value));
                }
            }
        }
    }
    encrypted_refs.sort();
    encrypted_refs.dedup();
    Ok(encrypted_refs)
}
