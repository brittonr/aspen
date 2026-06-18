#[path = "repro/bundle.rs"]
mod bundle;
#[path = "repro/command.rs"]
mod command;
#[path = "repro/io.rs"]
mod io;

pub(crate) type ReproCommand = command::Top;

pub(crate) fn run_repro_command(command: ReproCommand) -> molten::error::Result<()> {
    match command {
        ReproCommand::Export {
            report,
            out,
            profile,
            failure_out,
        } => {
            let artifact_value = io::read_preserves_file_with_failure(&report, failure_out.as_ref(), "export")?;
            let export_profile = match molten::harness::ReproExportProfile::parse(&profile) {
                Ok(profile) => profile,
                Err(error) => {
                    io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
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
            if molten::harness::parse_failure(&artifact_value).is_ok() {
                bundle::export_failure(&artifact_value, &out, &command, failure_out.as_ref())
            } else {
                bundle::export_report(&artifact_value, &out, &command, export_profile, failure_out.as_ref())
            }
        }
        ReproCommand::Verify {
            bundle,
            failure_out,
            receipt_out,
        } => {
            let bundle_value = io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "verify")?;
            let receipt = match molten::harness::repro_verify_receipt_value(&bundle_value) {
                Ok(receipt) => receipt,
                Err(error) => {
                    io::write_optional_artifact_failure(failure_out.as_ref(), "verify", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            if let Err(error) = io::emit_verify_receipt(receipt_out.as_ref(), &receipt) {
                io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &bundle_value)?;
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
            let bundle_value = io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "unpack")?;
            let reveal_receipt_values = reveal_receipts
                .iter()
                .map(|path| io::read_preserves_file(path))
                .collect::<molten::error::Result<Vec<_>>>()?;
            bundle::unpack_report(&bundle_value, &out, &reveal_receipt_values, failure_out.as_ref())
        }
        ReproCommand::Publish {
            bundle,
            store,
            node,
            receipt_out,
            failure_out,
        } => {
            let bundle_value = io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "publish")?;
            let published = match molten::iroh_exchange::publish_bundle(&store, &bundle_value, &node) {
                Ok(published) => published,
                Err(error) => {
                    io::write_optional_artifact_failure(failure_out.as_ref(), "publish", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            io::emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &published.receipt_value)?;
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
            let fetched = match molten::iroh_exchange::fetch_bundle(&molten::iroh_exchange::FetchBundleInput {
                root: &store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                out: out.as_deref(),
                ledger_root: ledger.as_deref(),
            }) {
                Ok(fetched) => fetched,
                Err(error) => {
                    io::write_optional_failure(failure_out.as_ref(), "fetch", &error, None)?;
                    return Err(error);
                }
            };
            io::emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &fetched.receipt_value)?;
            println!("repro fetch ok ticket={} bundle={}", fetched.ticket, fetched.bundle_ref);
            Ok(())
        }
    }
}
