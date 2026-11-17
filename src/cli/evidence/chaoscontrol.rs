//! Capability-rooted offline ChaosControl import and retained-policy status.

type Outcome<T> = molten::error::Result<T>;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    /// Validate retained instrumentation data and store its canonical receipt.
    Import {
        #[arg(long)]
        source_root: std::path::PathBuf,
        #[arg(long)]
        expected: String,
        #[arg(long)]
        bundle: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    /// Revalidate a receipt against its retained policy and show its summary.
    Status {
        #[arg(long)]
        ledger: std::path::PathBuf,
        #[arg(long)]
        receipt_ref: String,
    },
}

pub(crate) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Import {
            source_root,
            expected,
            bundle,
            ledger,
        } => import(&source_root, &expected, &bundle, &ledger),
        Command::Status { ledger, receipt_ref } => status(&ledger, &receipt_ref),
    }
}

// r[impl molten.testing.chaoscontrol_smr_evidence]
// r[impl molten.testing.chaoscontrol_smr_boundary]
fn import(source_root: &std::path::Path, expected: &str, bundle: &str, ledger: &std::path::Path) -> Outcome<()> {
    let source = cap_std::fs::Dir::open_ambient_dir(source_root, cap_std::ambient_authority())
        .map_err(molten::error::Failure::from)?;
    let expected_bytes = read_member(
        &source,
        expected,
        molten::fabric_consistency::chaoscontrol::conformance::admission::MAX_EXPECTED_BYTES,
    )?;
    let expected = molten::fabric_consistency::chaoscontrol::conformance::admission::expected_json(&expected_bytes)
        .map_err(denied)?;
    let bundle_bytes = read_member(
        &source,
        bundle,
        molten::fabric_consistency::chaoscontrol::conformance::admission::MAX_BUNDLE_BYTES,
    )?;
    let imported =
        molten::fabric_consistency::chaoscontrol::conformance::admission::import_json(&expected, &bundle_bytes)
            .map_err(denied)?;
    // Invalid evidence cannot create or mutate a destination ledger.
    let root = molten::ledger::open_capability_ledger_root(ledger)?;
    let stored = molten::ledger::import_artifact_with_root(&root, &imported.value)?;
    if stored.artifact_ref != imported.receipt_ref {
        return Err(molten::error::Failure::invalid_harness("ChaosControl receipt identity changed during storage"));
    }
    println!(
        "chaoscontrol status={} evidence-class={} receipt={} guest-execution=false snapshot-replay=false",
        imported.status, imported.evidence_class, imported.receipt_ref
    );
    Ok(())
}

fn status(ledger: &std::path::Path, receipt_ref: &str) -> Outcome<()> {
    let root = molten::ledger::CapabilityLedgerRoot::open_existing(ledger)?;
    let value = molten::ledger::read_artifact_bounded_with_root(
        &root,
        receipt_ref,
        molten::fabric_consistency::chaoscontrol::conformance::admission::MAX_RECEIPT_BYTES,
    )?;
    let fields = value
        .collect_simple_record(
            molten::fabric_consistency::chaoscontrol::conformance::admission::RECEIPT_RECORD_LABEL,
            Some(3),
        )
        .ok_or_else(|| molten::error::Failure::invalid_harness("not a ChaosControl conformance receipt"))?;
    let expected = fields[1]
        .collect_simple_record(
            molten::fabric_consistency::chaoscontrol::conformance::admission::EXPECTED_RECORD_LABEL,
            Some(1),
        )
        .ok_or_else(|| molten::error::Failure::invalid_harness("missing retained ChaosControl policy"))?;
    let expected = expected[0]
        .as_string()
        .ok_or_else(|| molten::error::Failure::invalid_harness("retained ChaosControl policy is not typed JSON"))?;
    let expected = molten::fabric_consistency::chaoscontrol::conformance::admission::expected_json(expected.as_bytes())
        .map_err(denied)?;
    let bundle = fields[2]
        .collect_simple_record(
            molten::fabric_consistency::chaoscontrol::conformance::admission::BUNDLE_RECORD_LABEL,
            Some(1),
        )
        .ok_or_else(|| molten::error::Failure::invalid_harness("missing retained ChaosControl bundle"))?;
    let bundle = bundle[0]
        .as_string()
        .ok_or_else(|| molten::error::Failure::invalid_harness("retained ChaosControl bundle is not typed JSON"))?;
    let imported =
        molten::fabric_consistency::chaoscontrol::conformance::admission::import_json(&expected, bundle.as_bytes())
            .map_err(denied)?;
    if imported.receipt_ref != receipt_ref {
        return Err(molten::error::Failure::invalid_harness(
            "ChaosControl receipt summary differs from retained inputs",
        ));
    }
    let summary: &preserves::IOValue = (&fields[0]).into();
    println!("{}", molten::preserves_rail::to_text(summary)?);
    Ok(())
}

fn denied(
    issue: molten::fabric_consistency::chaoscontrol::conformance::admission::ImportIssue,
) -> molten::error::Failure {
    molten::error::Failure::invalid_harness(format!("chaoscontrol-evidence-denied:{issue}"))
}

fn read_member(root: &cap_std::fs::Dir, member: &str, maximum_bytes: u64) -> Outcome<Vec<u8>> {
    use std::io::Read;

    use cap_fs_ext::OpenOptionsFollowExt;
    #[cfg(unix)]
    use cap_std::fs::OpenOptionsExt;

    let path = molten::node_state::RelativePath::parse(member)?;
    if path.as_path().file_name() != Some(std::ffi::OsStr::new(member)) {
        return Err(molten::error::Failure::invalid_harness("ChaosControl input must be one relative file leaf"));
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    #[cfg(unix)]
    options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
    let file = root.open_with(path.as_path(), &options).map_err(molten::error::Failure::from)?;
    let metadata = file.metadata().map_err(molten::error::Failure::from)?;
    if !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(molten::error::Failure::invalid_harness("ChaosControl input is not a bounded regular file"));
    }
    let limit = maximum_bytes
        .checked_add(1)
        .ok_or_else(|| molten::error::Failure::invalid_harness("ChaosControl read bound overflow"))?;
    let capacity = usize::try_from(metadata.len())
        .map_err(|_| molten::error::Failure::invalid_harness("ChaosControl input size exceeds host size"))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(limit).read_to_end(&mut bytes).map_err(molten::error::Failure::from)?;
    let actual = u64::try_from(bytes.len())
        .map_err(|_| molten::error::Failure::invalid_harness("ChaosControl input size exceeds bound representation"))?;
    if actual > maximum_bytes {
        return Err(molten::error::Failure::invalid_harness("ChaosControl input exceeds read bound"));
    }
    Ok(bytes)
}
