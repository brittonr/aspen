type Command = super::GateCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Check {
            artifact,
            failure_out,
            receipt_out,
        } => run_check(artifact, failure_out, receipt_out),
    }
}

fn run_check(
    artifact: std::path::PathBuf,
    failure_out: Option<std::path::PathBuf>,
    receipt_out: Option<std::path::PathBuf>,
) -> Outcome<()> {
    let artifact_value = super::io::read_preserves_file_with_failure(&artifact, failure_out.as_ref(), "validate")?;
    let check = match molten::harness::gate_check_value(&artifact_value) {
        Ok(check) => check,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out.as_ref(), "validate", &error, &artifact_value)?;
            return Err(error);
        }
    };
    let receipt = molten::harness::gate_receipt_value(&check);
    if let Err(error) = super::io::emit_gate_receipt(receipt_out.as_ref(), &receipt) {
        super::io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
        return Err(error);
    }
    Ok(())
}
