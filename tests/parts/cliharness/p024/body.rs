
fn expect_running(root: &std::path::Path, health: &std::path::Path, receipt: &std::path::Path) -> CliResult<()> {
    let output = molten_cmd()
        .args(["node", "status", "--state-root"])
        .arg(root)
        .args(["--health-out"])
        .arg(health)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node status");
    assert!(stdout(&output).contains("node status running"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(health)?), "node-health-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-receipt");
    Ok(())
}

fn expect_stop_loop(
    source_trust: Option<&std::ffi::OsStr>,
    root: &std::path::Path,
    shutdown: &std::path::Path,
    receipt: &std::path::Path,
) -> CliResult<()> {
    let output = source_gated_node_command(source_trust, root)?
        .args(["node", "run-loop", "--state-root"])
        .arg(root)
        .args(["--max-requests", "4", "--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node socket shutdown loop");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(shutdown)?), "node-shutdown-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-loop-receipt");
    Ok(())
}

fn start_state(
    source_trust: Option<&std::ffi::OsStr>,
    root: &std::path::Path,
    node_id: &str,
    init_label: &str,
    run_label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "init", "--state-root"])
            .arg(root)
            .args(["--node-id", node_id])
            .output()?,
        init_label,
    );
    let run = source_gated_node_command(source_trust, root)?
        .args(["test", "node", "run", "--state-root"])
        .arg(root)
        .output()?;
    assert_success(&run, run_label);
    Ok(())
}
