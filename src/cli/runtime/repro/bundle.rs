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
        let mut payloads = vec![
            materialization_payload("report.preserves", molten::preserves_rail::to_text(exported_report_value)?),
            materialization_payload("suite.preserves", molten::preserves_rail::to_text(&suite_value)?),
            materialization_payload("summary.txt", molten::harness::report_summary(exported_report_value)?),
            materialization_payload("commands.txt", REPORT_COMMANDS),
            materialization_payload("refs.preserves", molten::preserves_rail::to_text(&bundle_value)?),
        ];
        push_optional_payload(&mut payloads, "gate-receipt.preserves", bundle.receipt_value.as_ref())?;
        push_optional_payload(&mut payloads, "export-profile.preserves", bundle.export_profile_value.as_ref())?;
        push_optional_payload(
            &mut payloads,
            "redaction-transform-manifest.preserves",
            bundle.redaction_transform_manifest_value.as_ref(),
        )?;
        push_optional_payload(
            &mut payloads,
            "redaction-transform-receipt.preserves",
            bundle.redaction_transform_receipt_value.as_ref(),
        )?;
        push_optional_payload(
            &mut payloads,
            "private-bundle-profile.preserves",
            bundle.private_bundle_profile_value.as_ref(),
        )?;
        materialize_repro_payloads(out, "repro-report-export-v1", &payloads)
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
        let payloads = vec![
            materialization_payload("failure.preserves", molten::preserves_rail::to_text(failure_value)?),
            materialization_payload("summary.txt", molten::harness::failure_summary(failure_value)?),
            materialization_payload("commands.txt", FAILURE_COMMANDS),
            materialization_payload("refs.preserves", molten::preserves_rail::to_text(&bundle_value)?),
        ];
        materialize_repro_payloads(out, "repro-failure-export-v1", &payloads)
    })();
    if let Err(error) = export {
        super::io::write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
        return Err(error);
    }
    println!("failure repro bundle written to {}", out.display());
    Ok(())
}

fn materialization_payload(
    logical_path: &str,
    contents: impl AsRef<[u8]>,
) -> molten::materialization::MaterializationPayload {
    molten::materialization::MaterializationPayload::new(logical_path, contents.as_ref().to_vec())
}

fn push_optional_payload(
    payloads: &mut Vec<molten::materialization::MaterializationPayload>,
    logical_path: &str,
    value: Option<&preserves::IOValue>,
) -> molten::error::Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    payloads.push(materialization_payload(logical_path, molten::preserves_rail::to_text(value)?));
    Ok(())
}

pub(super) fn materialize_repro_payloads(
    out: &std::path::Path,
    profile: &str,
    payloads: &[molten::materialization::MaterializationPayload],
) -> molten::error::Result<()> {
    // r[impl molten.filesystem_materialization.root]
    // r[impl molten.filesystem_materialization.commit]
    let policy = molten::materialization::MaterializationPolicy::bounded(
        profile,
        molten::materialization::ReplacementPolicy::ReplaceRegularFiles,
    )?;
    let receipt = molten::materialization::materialize_path(out, &policy, payloads)?;
    let receipt_policy = molten::materialization::MaterializationPolicy::bounded(
        "repro-materialization-receipt-v1",
        molten::materialization::ReplacementPolicy::ReplaceRegularFiles,
    )?;
    let receipt_payload = [materialization_payload(
        "materialization-receipt.preserves",
        molten::preserves_rail::to_text(&receipt.value)?,
    )];
    molten::materialization::materialize_path(out, &receipt_policy, &receipt_payload)?;
    Ok(())
}

const REPORT_COMMANDS: &str = "molten test repro verify refs.preserves\nmolten test report validate report.preserves\nmolten test replay report.preserves\nmolten test report show report.preserves\nmolten test gate check refs.preserves\nmolten test repro unpack refs.preserves --out unpacked\n";
const FAILURE_COMMANDS: &str = "molten test report show failure.preserves\nmolten test gate check refs.preserves\n";
