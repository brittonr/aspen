type Command = super::ReportCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Show { report } => {
            let report_value = super::io::read_preserves_file(&report)?;
            println!("{}", report_show_summary(&report_value)?);
            Ok(())
        }
        Command::Validate { report, failure_out } => {
            let report_value = super::io::read_preserves_file_with_failure(&report, failure_out.as_ref(), "validate")?;
            let validation = match molten::harness::validate_report_value(&report_value) {
                Ok(validation) => validation,
                Err(error) => {
                    super::io::write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            let replay = match molten::harness::replay_report_value(&report_value) {
                Ok(replay) => replay,
                Err(error) => {
                    super::io::write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            println!(
                "report validate ok report={} suite={} observations={} final_state={} replay_actual={}",
                validation.report_ref,
                validation.suite_ref,
                validation.observations,
                validation.final_state_hash,
                replay.actual_report_ref
            );
            Ok(())
        }
    }
}

fn report_show_summary(report_value: &preserves::IOValue) -> Outcome<String> {
    let report_error = match molten::harness::report_summary(report_value) {
        Ok(summary) => return Ok(summary),
        Err(error) => error,
    };
    if let Ok(summary) = molten::harness::failure_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::harness::repro_bundle_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::harness::gate_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::harness::repro_verify_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::evidence::signed_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::secrets::fixture_report_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = molten::secrets::secrets_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = crate::cli_remote::remote_dataspace_gate_summary(report_value) {
        return Ok(summary);
    }
    Err(report_error)
}
