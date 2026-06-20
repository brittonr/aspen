type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;
type Command = super::ReproCommand;

struct FetchInput {
    ticket: String,
    store: FilePath,
    out: Option<FilePath>,
    ledger: Option<FilePath>,
    expected_bundle_ref: Option<String>,
    peer: String,
    receipt_out: Option<FilePath>,
    failure_out: Option<FilePath>,
}

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Export {
            report,
            out,
            profile,
            failure_out,
        } => export(report, out, profile, failure_out),
        Command::Verify {
            bundle,
            failure_out,
            receipt_out,
        } => verify(bundle, failure_out, receipt_out),
        Command::Unpack {
            bundle,
            out,
            reveal_receipts,
            failure_out,
        } => unpack(bundle, out, reveal_receipts, failure_out),
        Command::Publish {
            bundle,
            store,
            node,
            receipt_out,
            failure_out,
        } => publish(bundle, store, node, receipt_out, failure_out),
        Command::Fetch {
            ticket,
            store,
            out,
            ledger,
            expected_bundle_ref,
            peer,
            receipt_out,
            failure_out,
        } => fetch(FetchInput {
            ticket,
            store,
            out,
            ledger,
            expected_bundle_ref,
            peer,
            receipt_out,
            failure_out,
        }),
    }
}

fn export(report: FilePath, out: FilePath, profile: String, failure_out: Option<FilePath>) -> Outcome<()> {
    let artifact_value = super::io::read_preserves_file_with_failure(&report, failure_out.as_ref(), "export")?;
    let export_profile = match molten::harness::ReproExportProfile::parse(&profile) {
        Ok(profile) => profile,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
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
        super::bundle::export_failure(&artifact_value, &out, &command, failure_out.as_ref())
    } else {
        super::bundle::export_report(&artifact_value, &out, &command, export_profile, failure_out.as_ref())
    }
}

fn verify(bundle: FilePath, failure_out: Option<FilePath>, receipt_out: Option<FilePath>) -> Outcome<()> {
    let bundle_value = super::io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "verify")?;
    let receipt = match molten::harness::repro_verify_receipt_value(&bundle_value) {
        Ok(receipt) => receipt,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out.as_ref(), "verify", &error, &bundle_value)?;
            return Err(error);
        }
    };
    if let Err(error) = super::io::emit_verify_receipt(receipt_out.as_ref(), &receipt) {
        super::io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &bundle_value)?;
        return Err(error);
    }
    Ok(())
}

fn unpack(
    bundle: FilePath,
    out: FilePath,
    reveal_receipts: Vec<FilePath>,
    failure_out: Option<FilePath>,
) -> Outcome<()> {
    let bundle_value = super::io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "unpack")?;
    let reveal_receipt_values = reveal_receipts
        .iter()
        .map(|path| super::io::read_preserves_file(path))
        .collect::<Outcome<Vec<_>>>()?;
    super::bundle::unpack_report(&bundle_value, &out, &reveal_receipt_values, failure_out.as_ref())
}

fn publish(
    bundle: FilePath,
    store: FilePath,
    node: String,
    receipt_out: Option<FilePath>,
    failure_out: Option<FilePath>,
) -> Outcome<()> {
    let bundle_value = super::io::read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "publish")?;
    let published = match molten::iroh_exchange::publish_bundle(&store, &bundle_value, &node) {
        Ok(published) => published,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out.as_ref(), "publish", &error, &bundle_value)?;
            return Err(error);
        }
    };
    super::io::emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &published.receipt_value)?;
    println!("repro publish ok ticket={} bundle={}", published.ticket, published.bundle_ref);
    Ok(())
}

fn fetch(input: FetchInput) -> Outcome<()> {
    let fetched = match molten::iroh_exchange::fetch_bundle(&molten::iroh_exchange::FetchBundleInput {
        root: &input.store,
        ticket: &input.ticket,
        expected_bundle_ref: input.expected_bundle_ref.as_deref(),
        peer: &input.peer,
        out: input.out.as_deref(),
        ledger_root: input.ledger.as_deref(),
    }) {
        Ok(fetched) => fetched,
        Err(error) => {
            super::io::write_optional_failure(input.failure_out.as_ref(), "fetch", &error, None)?;
            return Err(error);
        }
    };
    super::io::emit_named_receipt(input.receipt_out.as_ref(), "iroh repro exchange receipt", &fetched.receipt_value)?;
    println!("repro fetch ok ticket={} bundle={}", fetched.ticket, fetched.bundle_ref);
    Ok(())
}
