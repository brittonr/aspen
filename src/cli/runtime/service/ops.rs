type Command = super::ServiceCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Run { suite, out } => {
            let suite_value = super::io::read_preserves_file(&suite)?;
            let run = molten::service_runtime::run_service_runtime_suite_value(&suite_value)?;
            super::io::write_service_runtime_run(&out, &suite_value, &run)?;
            println!(
                "service runtime run report={} suite={} lifecycle={} readiness={} out={}",
                run.report_ref,
                run.suite_ref,
                run.lifecycle_receipts.len(),
                run.readiness_assertions.len(),
                out.display()
            );
            Ok(())
        }
        Command::RunTwoService { out } => {
            let suite_value = molten::service_runtime::two_service_suite_value()?;
            let run = molten::service_runtime::run_service_runtime_suite_value(&suite_value)?;
            super::io::write_service_runtime_run(&out, &suite_value, &run)?;
            println!(
                "service runtime two-service report={} suite={} lifecycle={} readiness={} out={}",
                run.report_ref,
                run.suite_ref,
                run.lifecycle_receipts.len(),
                run.readiness_assertions.len(),
                out.display()
            );
            Ok(())
        }
        Command::Supervise { suite, out } => {
            let suite_value = super::io::read_preserves_file(&suite)?;
            let run = molten::service_supervision::run_service_supervision_suite_value(&suite_value)?;
            super::io::write_service_supervision_run(&out, &suite_value, &run)?;
            println!(
                "service supervision run report={} suite={} monitors={} cleanup={} out={}",
                run.report_ref,
                run.suite_ref,
                run.monitor_notifications.len(),
                run.cleanup_receipts.len(),
                out.display()
            );
            Ok(())
        }
        Command::RunSupervisionFixture { out } => {
            let suite_value = molten::service_supervision::supervision_fixture_suite_value()?;
            let run = molten::service_supervision::run_service_supervision_suite_value(&suite_value)?;
            super::io::write_service_supervision_run(&out, &suite_value, &run)?;
            println!(
                "service supervision fixture report={} suite={} monitors={} cleanup={} out={}",
                run.report_ref,
                run.suite_ref,
                run.monitor_notifications.len(),
                run.cleanup_receipts.len(),
                out.display()
            );
            Ok(())
        }
        Command::Show { report } => {
            let value = super::io::read_preserves_file(&report)?;
            println!("{}", molten::service_runtime::service_runtime_summary(&value)?);
            Ok(())
        }
        Command::ShowSupervision { report } => {
            let value = super::io::read_preserves_file(&report)?;
            println!("{}", molten::service_supervision::service_supervision_summary(&value)?);
            Ok(())
        }
        Command::GateSupervision { report, receipt_out } => {
            let value = super::io::read_preserves_file(&report)?;
            let gate = molten::service_supervision::gate_service_supervision_report(&value)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "service supervision gate receipt", &gate.value)?;
            println!(
                "service supervision gate {} report={} suite={} restart={} monitors={} cleanup={} diagnostics={}",
                gate.decision,
                gate.report_ref,
                gate.suite_ref,
                gate.restart_decision.as_deref().unwrap_or("none"),
                gate.monitor_count,
                gate.cleanup_count,
                gate.diagnostics.len()
            );
            Ok(())
        }
        Command::Replay { report } => {
            let value = super::io::read_preserves_file(&report)?;
            let replay = molten::service_runtime::replay_service_runtime_report(&value)?;
            println!(
                "service runtime replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
        Command::ReplaySupervision { report } => {
            let value = super::io::read_preserves_file(&report)?;
            let replay = molten::service_supervision::replay_service_supervision_report(&value)?;
            println!(
                "service supervision replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
    }
}
