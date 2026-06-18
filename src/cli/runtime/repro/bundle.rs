pub(crate) fn export_report(
    report_value: &preserves::IOValue,
    out: &std::path::Path,
    command: &[String],
    profile: molten::harness::ReproExportProfile,
    failure_out: Option<&std::path::PathBuf>,
) -> molten::error::Result<()> {
    let bundle_value = match molten::harness::repro_bundle_value_with_export_profile(report_value, command, profile) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let bundle = match molten::harness::parse_repro_bundle(&bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let exported_report_value = bundle.report_value.as_ref().unwrap_or(report_value);
    let suite_value = match molten::harness::report_suite_value(exported_report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "export", &error, exported_report_value)?;
            return Err(error);
        }
    };
    let export = (|| -> molten::error::Result<()> {
        std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
        super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(exported_report_value)?)?;
        super::io::write_file(&out.join("suite.preserves"), &molten::preserves_rail::to_text(&suite_value)?)?;
        super::io::write_file(&out.join("summary.txt"), &molten::harness::report_summary(exported_report_value)?)?;
        super::io::write_file(&out.join("commands.txt"), REPORT_COMMANDS)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            super::io::write_file(
                &out.join("gate-receipt.preserves"),
                &molten::preserves_rail::to_text(gate_receipt_value)?,
            )?;
        }
        if let Some(value) = bundle.export_profile_value.as_ref() {
            super::io::write_file(&out.join("export-profile.preserves"), &molten::preserves_rail::to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            super::io::write_file(
                &out.join("redaction-transform-manifest.preserves"),
                &molten::preserves_rail::to_text(value)?,
            )?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            super::io::write_file(
                &out.join("redaction-transform-receipt.preserves"),
                &molten::preserves_rail::to_text(value)?,
            )?;
        }
        if let Some(value) = bundle.private_bundle_profile_value.as_ref() {
            super::io::write_file(
                &out.join("private-bundle-profile.preserves"),
                &molten::preserves_rail::to_text(value)?,
            )?;
        }
        super::io::write_file(&out.join("refs.preserves"), &molten::preserves_rail::to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        super::io::write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
        return Err(error);
    }
    println!("repro bundle written to {}", out.display());
    Ok(())
}

pub(crate) fn unpack_report(
    bundle_value: &preserves::IOValue,
    out: &std::path::Path,
    reveal_receipt_values: &[preserves::IOValue],
    failure_out: Option<&std::path::PathBuf>,
) -> molten::error::Result<()> {
    let bundle = match molten::harness::parse_repro_bundle(bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    if bundle.loss_classification.as_deref() == Some("requires-reveal") {
        if let Err(error) = validate_reveal_receipts(&bundle.encrypted_refs, reveal_receipt_values) {
            super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    } else if !reveal_receipt_values.is_empty() {
        let error = molten::error::MoltenError::invalid_harness(
            "reveal receipts are only accepted for encrypted-private repro bundles",
        );
        super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    let verify_receipt = if bundle.loss_classification.as_deref().unwrap_or("gate-preserving") == "gate-preserving" {
        match molten::harness::repro_verify_receipt_value(bundle_value) {
            Ok(receipt) => Some(receipt),
            Err(error) => {
                super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
                return Err(error);
            }
        }
    } else {
        None
    };
    let report_value = match bundle.report_value.as_ref() {
        Some(report_value) => report_value,
        None => {
            let error = molten::error::MoltenError::invalid_harness("repro unpack requires an embedded report");
            super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let suite_value = match molten::harness::report_suite_value(report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let export = (|| -> molten::error::Result<()> {
        std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
        super::io::write_file(&out.join("refs.preserves"), &molten::preserves_rail::to_text(bundle_value)?)?;
        super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(report_value)?)?;
        super::io::write_file(&out.join("suite.preserves"), &molten::preserves_rail::to_text(&suite_value)?)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            super::io::write_file(
                &out.join("gate-receipt.preserves"),
                &molten::preserves_rail::to_text(gate_receipt_value)?,
            )?;
        }
        if let Some(verify_receipt) = verify_receipt.as_ref() {
            super::io::write_file(
                &out.join("verify-receipt.preserves"),
                &molten::preserves_rail::to_text(verify_receipt)?,
            )?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            super::io::write_file(
                &out.join("redaction-transform-receipt.preserves"),
                &molten::preserves_rail::to_text(value)?,
            )?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            super::io::write_file(
                &out.join("redaction-transform-manifest.preserves"),
                &molten::preserves_rail::to_text(value)?,
            )?;
        }
        for (index, receipt) in reveal_receipt_values.iter().enumerate() {
            super::io::write_file(
                &out.join(format!("reveal-receipt-{index}.preserves")),
                &molten::preserves_rail::to_text(receipt)?,
            )?;
        }
        super::io::write_file(&out.join("summary.txt"), &molten::harness::repro_bundle_summary(bundle_value)?)?;
        super::io::write_file(&out.join("commands.txt"), REPORT_COMMANDS)?;
        Ok(())
    })();
    if let Err(error) = export {
        super::io::write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    println!("repro bundle unpacked to {}", out.display());
    Ok(())
}

fn validate_reveal_receipts(
    encrypted_refs: &[String],
    receipt_values: &[preserves::IOValue],
) -> molten::error::Result<()> {
    if encrypted_refs.is_empty() {
        return Err(molten::error::MoltenError::invalid_harness(
            "encrypted-private repro bundle has no encrypted refs to reveal",
        ));
    }
    if receipt_values.is_empty() {
        return Err(molten::error::MoltenError::invalid_harness(
            "encrypted-private repro unpack requires at least one passing reveal receipt",
        ));
    }
    let expected_refs = encrypted_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let mut authorized_refs = std::collections::BTreeSet::new();
    for receipt_value in receipt_values {
        let receipt = molten::secrets::parse_reveal_receipt(receipt_value)?;
        if receipt.decision != "pass" {
            return Err(molten::error::MoltenError::invalid_harness(
                "unauthorized reveal receipt cannot unpack private repro material",
            ));
        }
        let encrypted_ref = receipt.encrypted_ref.as_ref().ok_or_else(|| {
            molten::error::MoltenError::invalid_harness("reveal receipt does not bind an encrypted repro reference")
        })?;
        if !expected_refs.contains(encrypted_ref) {
            return Err(molten::error::MoltenError::invalid_harness(
                "reveal receipt encrypted ref is not part of this repro bundle",
            ));
        }
        authorized_refs.insert(encrypted_ref.clone());
    }
    for encrypted_ref in encrypted_refs {
        if !authorized_refs.contains(encrypted_ref) {
            return Err(molten::error::MoltenError::invalid_harness(
                "reveal receipts do not authorize every encrypted repro reference",
            ));
        }
    }
    Ok(())
}

pub(crate) fn export_failure(
    failure_value: &preserves::IOValue,
    out: &std::path::Path,
    command: &[String],
    failure_out: Option<&std::path::PathBuf>,
) -> molten::error::Result<()> {
    let bundle_value = match molten::harness::failure_repro_bundle_value_with_command(failure_value, command) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
            return Err(error);
        }
    };
    let export = (|| -> molten::error::Result<()> {
        std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
        super::io::write_file(&out.join("failure.preserves"), &molten::preserves_rail::to_text(failure_value)?)?;
        super::io::write_file(&out.join("summary.txt"), &molten::harness::failure_summary(failure_value)?)?;
        super::io::write_file(&out.join("commands.txt"), FAILURE_COMMANDS)?;
        super::io::write_file(&out.join("refs.preserves"), &molten::preserves_rail::to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        super::io::write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
        return Err(error);
    }
    println!("failure repro bundle written to {}", out.display());
    Ok(())
}

const REPORT_COMMANDS: &str = "molten test repro verify refs.preserves\nmolten test report validate report.preserves\nmolten test replay report.preserves\nmolten test report show report.preserves\nmolten test gate check refs.preserves\nmolten test repro unpack refs.preserves --out unpacked\n";
const FAILURE_COMMANDS: &str = "molten test report show failure.preserves\nmolten test gate check refs.preserves\n";
