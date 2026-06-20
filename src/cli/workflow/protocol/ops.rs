type Command = super::ProtocolCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Install { manifest, out } => run_install(manifest, out),
        Command::RunRequestResponse { out } => run_request_response(out),
        Command::GateLifecycle { dir, receipt_out } => run_gate_lifecycle(dir, receipt_out),
        Command::Show { receipt } => run_show(receipt),
    }
}

fn run_install(manifest: std::path::PathBuf, out: std::path::PathBuf) -> Outcome<()> {
    let manifest_value = super::io::read_preserves_file(&manifest)?;
    let install = molten::protocol_session::install_protocol_manifest_value(&manifest_value)?;
    super::io::write_install(&out, &manifest_value, &install)?;
    println!(
        "protocol install decision={} receipt={} protocol={} endpoints={} out={}",
        install.decision,
        install.receipt_ref,
        install.manifest.protocol_id,
        install.endpoints.len(),
        out.display()
    );
    Ok(())
}

fn run_request_response(out: std::path::PathBuf) -> Outcome<()> {
    let lifecycle = molten::protocol_session::request_response_lifecycle()?;
    super::io::write_lifecycle(&out, &lifecycle)?;
    println!(
        "protocol request-response receipt={} operations={} out={}",
        lifecycle.install.receipt_ref,
        lifecycle.operations.len(),
        out.display()
    );
    Ok(())
}

fn run_gate_lifecycle(dir: std::path::PathBuf, receipt_out: Option<std::path::PathBuf>) -> Outcome<()> {
    let gate = molten::protocol_session::gate_protocol_session_lifecycle(super::io::read_lifecycle_gate_input(&dir)?)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "protocol session gate receipt", &gate.value)?;
    println!(
        "protocol session gate {} install={} protocol={} sessions={} operations={} diagnostics={}",
        gate.decision,
        gate.install_ref,
        gate.protocol_ref,
        gate.session_ids.len(),
        gate.operation_count,
        gate.diagnostics.len()
    );
    Ok(())
}

fn run_show(receipt: std::path::PathBuf) -> Outcome<()> {
    let value = super::io::read_preserves_file(&receipt)?;
    println!("{}", molten::protocol_session::protocol_summary(&value)?);
    Ok(())
}
