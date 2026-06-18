use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::harness::ReproExportProfile;
use molten::harness::failure_repro_bundle_value_with_command;
use molten::harness::failure_summary;
use molten::harness::failure_value;
use molten::harness::parse_failure;
use molten::harness::parse_repro_bundle;
use molten::harness::report_suite_value;
use molten::harness::report_summary;
use molten::harness::repro_bundle_summary;
use molten::harness::repro_bundle_value_with_export_profile;
use molten::harness::repro_verify_receipt_value;
use molten::iroh_exchange::FetchBundleInput;
use molten::iroh_exchange::fetch_bundle;
use molten::iroh_exchange::publish_bundle;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::secrets;

#[path = "repro/command.rs"]
mod command;

pub(crate) type ReproCommand = command::Top;

pub(crate) fn run_repro_command(command: ReproCommand) -> Result<()> {
    match command {
        ReproCommand::Export {
            report,
            out,
            profile,
            failure_out,
        } => {
            let artifact_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "export")?;
            let export_profile = match ReproExportProfile::parse(&profile) {
                Ok(profile) => profile,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
                    return Err(error);
                }
            };
            let command = vec![
                "molten".to_string(),
                "test".to_string(),
                "repro".to_string(),
                "export".to_string(),
                report.display().to_string(),
                "--out".to_string(),
                out.display().to_string(),
                "--profile".to_string(),
                profile,
            ];
            if parse_failure(&artifact_value).is_ok() {
                export_failure_repro(&artifact_value, &out, &command, failure_out.as_ref())
            } else {
                export_report_repro(&artifact_value, &out, &command, export_profile, failure_out.as_ref())
            }
        }
        ReproCommand::Verify {
            bundle,
            failure_out,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "verify")?;
            let receipt = match repro_verify_receipt_value(&bundle_value) {
                Ok(receipt) => receipt,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "verify", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            if let Err(error) = emit_repro_verify_receipt(receipt_out.as_ref(), &receipt) {
                write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &bundle_value)?;
                return Err(error);
            }
            Ok(())
        }
        ReproCommand::Unpack {
            bundle,
            out,
            reveal_receipts,
            failure_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "unpack")?;
            let reveal_receipt_values =
                reveal_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            unpack_report_repro(&bundle_value, &out, &reveal_receipt_values, failure_out.as_ref())
        }
        ReproCommand::Publish {
            bundle,
            store,
            node,
            receipt_out,
            failure_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "publish")?;
            let published = match publish_bundle(&store, &bundle_value, &node) {
                Ok(published) => published,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "publish", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &published.receipt_value)?;
            println!("repro publish ok ticket={} bundle={}", published.ticket, published.bundle_ref);
            Ok(())
        }
        ReproCommand::Fetch {
            ticket,
            store,
            out,
            ledger,
            expected_bundle_ref,
            peer,
            receipt_out,
            failure_out,
        } => {
            let fetched = match fetch_bundle(&FetchBundleInput {
                root: &store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                out: out.as_deref(),
                ledger_root: ledger.as_deref(),
            }) {
                Ok(fetched) => fetched,
                Err(error) => {
                    write_optional_failure(failure_out.as_ref(), "fetch", &error, None)?;
                    return Err(error);
                }
            };
            emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &fetched.receipt_value)?;
            println!("repro fetch ok ticket={} bundle={}", fetched.ticket, fetched.bundle_ref);
            Ok(())
        }
    }
}

fn export_report_repro(
    report_value: &preserves::IOValue,
    out: &Path,
    command: &[String],
    profile: ReproExportProfile,
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle_value = match repro_bundle_value_with_export_profile(report_value, command, profile) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let bundle = match parse_repro_bundle(&bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let exported_report_value = bundle.report_value.as_ref().unwrap_or(report_value);
    let suite_value = match report_suite_value(exported_report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, exported_report_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("report.preserves"), &to_text(exported_report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        write_file(&out.join("summary.txt"), &report_summary(exported_report_value)?)?;
        write_file(&out.join("commands.txt"), REPORT_REPRO_COMMANDS)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            write_file(&out.join("gate-receipt.preserves"), &to_text(gate_receipt_value)?)?;
        }
        if let Some(value) = bundle.export_profile_value.as_ref() {
            write_file(&out.join("export-profile.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            write_file(&out.join("redaction-transform-manifest.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            write_file(&out.join("redaction-transform-receipt.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.private_bundle_profile_value.as_ref() {
            write_file(&out.join("private-bundle-profile.preserves"), &to_text(value)?)?;
        }
        write_file(&out.join("refs.preserves"), &to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
        return Err(error);
    }
    println!("repro bundle written to {}", out.display());
    Ok(())
}

fn unpack_report_repro(
    bundle_value: &preserves::IOValue,
    out: &Path,
    reveal_receipt_values: &[preserves::IOValue],
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle = match parse_repro_bundle(bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    if bundle.loss_classification.as_deref() == Some("requires-reveal") {
        if let Err(error) = validate_repro_reveal_receipts(&bundle.encrypted_refs, reveal_receipt_values) {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    } else if !reveal_receipt_values.is_empty() {
        let error =
            MoltenError::invalid_harness("reveal receipts are only accepted for encrypted-private repro bundles");
        write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    let verify_receipt = if bundle.loss_classification.as_deref().unwrap_or("gate-preserving") == "gate-preserving" {
        match repro_verify_receipt_value(bundle_value) {
            Ok(receipt) => Some(receipt),
            Err(error) => {
                write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
                return Err(error);
            }
        }
    } else {
        None
    };
    let report_value = match bundle.report_value.as_ref() {
        Some(report_value) => report_value,
        None => {
            let error = MoltenError::invalid_harness("repro unpack requires an embedded report");
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let suite_value = match report_suite_value(report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("refs.preserves"), &to_text(bundle_value)?)?;
        write_file(&out.join("report.preserves"), &to_text(report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            write_file(&out.join("gate-receipt.preserves"), &to_text(gate_receipt_value)?)?;
        }
        if let Some(verify_receipt) = verify_receipt.as_ref() {
            write_file(&out.join("verify-receipt.preserves"), &to_text(verify_receipt)?)?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            write_file(&out.join("redaction-transform-receipt.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            write_file(&out.join("redaction-transform-manifest.preserves"), &to_text(value)?)?;
        }
        for (index, receipt) in reveal_receipt_values.iter().enumerate() {
            write_file(&out.join(format!("reveal-receipt-{index}.preserves")), &to_text(receipt)?)?;
        }
        write_file(&out.join("summary.txt"), &repro_bundle_summary(bundle_value)?)?;
        write_file(&out.join("commands.txt"), REPORT_REPRO_COMMANDS)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    println!("repro bundle unpacked to {}", out.display());
    Ok(())
}

fn validate_repro_reveal_receipts(encrypted_refs: &[String], receipt_values: &[preserves::IOValue]) -> Result<()> {
    if encrypted_refs.is_empty() {
        return Err(MoltenError::invalid_harness("encrypted-private repro bundle has no encrypted refs to reveal"));
    }
    if receipt_values.is_empty() {
        return Err(MoltenError::invalid_harness(
            "encrypted-private repro unpack requires at least one passing reveal receipt",
        ));
    }
    let expected_refs = encrypted_refs.iter().cloned().collect::<BTreeSet<_>>();
    let mut authorized_refs = BTreeSet::new();
    for receipt_value in receipt_values {
        let receipt = secrets::parse_reveal_receipt(receipt_value)?;
        if receipt.decision != "pass" {
            return Err(MoltenError::invalid_harness(
                "unauthorized reveal receipt cannot unpack private repro material",
            ));
        }
        let encrypted_ref = receipt
            .encrypted_ref
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("reveal receipt does not bind an encrypted repro reference"))?;
        if !expected_refs.contains(encrypted_ref) {
            return Err(MoltenError::invalid_harness("reveal receipt encrypted ref is not part of this repro bundle"));
        }
        authorized_refs.insert(encrypted_ref.clone());
    }
    for encrypted_ref in encrypted_refs {
        if !authorized_refs.contains(encrypted_ref) {
            return Err(MoltenError::invalid_harness(
                "reveal receipts do not authorize every encrypted repro reference",
            ));
        }
    }
    Ok(())
}

fn export_failure_repro(
    failure_value: &preserves::IOValue,
    out: &Path,
    command: &[String],
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle_value = match failure_repro_bundle_value_with_command(failure_value, command) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("failure.preserves"), &to_text(failure_value)?)?;
        write_file(&out.join("summary.txt"), &failure_summary(failure_value)?)?;
        write_file(&out.join("commands.txt"), FAILURE_REPRO_COMMANDS)?;
        write_file(&out.join("refs.preserves"), &to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
        return Err(error);
    }
    println!("failure repro bundle written to {}", out.display());
    Ok(())
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn read_preserves_file_with_failure(
    path: &Path,
    failure_out: Option<&PathBuf>,
    phase: &'static str,
) -> Result<preserves::IOValue> {
    let text = match fs::read_to_string(path).map_err(MoltenError::from) {
        Ok(text) => text,
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            return Err(error);
        }
    };
    match parse_text(&text) {
        Ok(value) => Ok(value),
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            Err(error)
        }
    }
}

fn write_optional_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn write_optional_artifact_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    artifact_value: &preserves::IOValue,
) -> Result<()> {
    let artifact_ref = canonical_hash(artifact_value)?;
    write_optional_failure(
        path,
        phase,
        error,
        Some(vec![
            record("artifact-ref", vec![string(&artifact_ref)]),
            record("artifact", vec![artifact_value.clone()]),
        ]),
    )
}

fn emit_repro_verify_receipt(path: Option<&PathBuf>, receipt: &preserves::IOValue) -> Result<()> {
    emit_named_receipt(path, "repro verify receipt", receipt)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn emit_failure(path: Option<&PathBuf>, failure: &preserves::IOValue) -> Result<()> {
    let failure_text = to_text(failure)?;
    let failure_ref = canonical_hash(failure)?;
    if let Some(path) = path {
        write_file(path, &failure_text)?;
        eprintln!("failure {failure_ref} written to {}", path.display());
    } else {
        println!("{failure_text}");
        eprintln!("failure {failure_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}

const REPORT_REPRO_COMMANDS: &str = "molten test repro verify refs.preserves\nmolten test report validate report.preserves\nmolten test replay report.preserves\nmolten test report show report.preserves\nmolten test gate check refs.preserves\nmolten test repro unpack refs.preserves --out unpacked\n";
const FAILURE_REPRO_COMMANDS: &str =
    "molten test report show failure.preserves\nmolten test gate check refs.preserves\n";
