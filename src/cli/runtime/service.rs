use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::service_runtime;
use molten::service_supervision;

#[derive(Debug, Subcommand)]
pub(crate) enum ServiceCommand {
    Run {
        suite: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunTwoService {
        #[arg(long)]
        out: PathBuf,
    },
    Supervise {
        suite: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunSupervisionFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
    },
    ShowSupervision {
        report: PathBuf,
    },
    GateSupervision {
        report: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Replay {
        report: PathBuf,
    },
    ReplaySupervision {
        report: PathBuf,
    },
}

pub(crate) fn run_service_command(command: ServiceCommand) -> Result<()> {
    match command {
        ServiceCommand::Run { suite, out } => {
            let suite_value = read_preserves_file(&suite)?;
            let run = service_runtime::run_service_runtime_suite_value(&suite_value)?;
            write_service_runtime_run(&out, &suite_value, &run)?;
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
        ServiceCommand::RunTwoService { out } => {
            let suite_value = service_runtime::two_service_suite_value()?;
            let run = service_runtime::run_service_runtime_suite_value(&suite_value)?;
            write_service_runtime_run(&out, &suite_value, &run)?;
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
        ServiceCommand::Supervise { suite, out } => {
            let suite_value = read_preserves_file(&suite)?;
            let run = service_supervision::run_service_supervision_suite_value(&suite_value)?;
            write_service_supervision_run(&out, &suite_value, &run)?;
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
        ServiceCommand::RunSupervisionFixture { out } => {
            let suite_value = service_supervision::supervision_fixture_suite_value()?;
            let run = service_supervision::run_service_supervision_suite_value(&suite_value)?;
            write_service_supervision_run(&out, &suite_value, &run)?;
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
        ServiceCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", service_runtime::service_runtime_summary(&value)?);
            Ok(())
        }
        ServiceCommand::ShowSupervision { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", service_supervision::service_supervision_summary(&value)?);
            Ok(())
        }
        ServiceCommand::GateSupervision { report, receipt_out } => {
            let value = read_preserves_file(&report)?;
            let gate = service_supervision::gate_service_supervision_report(&value)?;
            emit_named_receipt(receipt_out.as_ref(), "service supervision gate receipt", &gate.value)?;
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
        ServiceCommand::Replay { report } => {
            let value = read_preserves_file(&report)?;
            let replay = service_runtime::replay_service_runtime_report(&value)?;
            println!(
                "service runtime replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
        ServiceCommand::ReplaySupervision { report } => {
            let value = read_preserves_file(&report)?;
            let replay = service_supervision::replay_service_supervision_report(&value)?;
            println!(
                "service supervision replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
    }
}

fn write_service_runtime_run(
    out: &Path,
    suite_value: &preserves::IOValue,
    run: &service_runtime::ServiceRuntimeRun,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &service_runtime::service_runtime_summary(&run.value)?)?;
    write_indexed_values(out, "lifecycle", &run.lifecycle_receipts)?;
    write_indexed_values(out, "status", &run.statuses)?;
    write_indexed_values(out, "readiness", &run.readiness_assertions)?;
    write_indexed_values(out, "replay-identity", &run.replay_identities)?;
    write_indexed_values(out, "turn-context", &run.turn_contexts)
}

fn write_service_supervision_run(
    out: &Path,
    suite_value: &preserves::IOValue,
    run: &service_supervision::ServiceSupervisionRun,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &service_supervision::service_supervision_summary(&run.value)?)?;
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

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
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

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
