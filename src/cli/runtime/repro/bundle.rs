#[path = "bundle/unpack.rs"]
mod unpack;

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
        if let Some(receipt_value) = bundle.receipt_value.as_ref() {
            super::io::write_file(
                &out.join("gate-receipt.preserves"),
                &molten::preserves_rail::to_text(receipt_value)?,
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
    unpack::run(bundle_value, out, reveal_receipt_values, failure_out)
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
