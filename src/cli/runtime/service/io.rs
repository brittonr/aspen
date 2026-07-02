type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn write_service_runtime_run(
    out: &std::path::Path,
    suite_value: &preserves::IOValue,
    run: &molten::service_runtime::Run,
) -> Outcome<()> {
    std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &molten::preserves_rail::to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &molten::service_runtime::summary(&run.value)?)?;
    write_indexed_values(out, "lifecycle", &run.lifecycle_receipts)?;
    write_indexed_values(out, "status", &run.statuses)?;
    write_indexed_values(out, "readiness", &run.readiness_assertions)?;
    write_indexed_values(out, "replay-identity", &run.replay_identities)?;
    write_indexed_values(out, "turn-context", &run.turn_contexts)
}

pub(super) fn write_service_supervision_run(
    out: &std::path::Path,
    suite_value: &preserves::IOValue,
    run: &molten::service_supervision::ServiceSupervisionRun,
) -> Outcome<()> {
    std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &molten::preserves_rail::to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &molten::service_supervision::service_supervision_summary(&run.value)?)?;
    write_indexed_values(out, "failure", &run.failure_markers)?;
    write_indexed_values(out, "status", &run.statuses)?;
    write_indexed_values(out, "lifecycle", &run.lifecycle_receipts)?;
    write_indexed_values(out, "monitor-notification", &run.monitor_notifications)?;
    write_indexed_values(out, "restart-decision", &run.restart_decisions)?;
    write_indexed_values(out, "scheduled-demand", &run.scheduled_demands)?;
    write_indexed_values(out, "cleanup", &run.cleanup_receipts)?;
    write_indexed_values(out, "retraction", &run.retractions)?;
    write_indexed_values(out, "retention", &run.retention_inputs)
}

fn write_indexed_values(out: &std::path::Path, prefix: &str, values: &[preserves::IOValue]) -> Outcome<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn emit_named_receipt(path: Option<&FilePath>, label: &str, receipt: &preserves::IOValue) -> Outcome<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &std::path::Path, contents: &str) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
