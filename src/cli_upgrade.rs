use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::upgrades;

#[derive(Debug, Subcommand)]
pub(crate) enum UpgradeCommand {
    PlanNameMove {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long = "source-gate-receipt")]
        source_gate_receipts: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Create {
        plan: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SetName {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunTask {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Rollback {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        plan_ref: String,
    },
    CleanupCheck {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_upgrade_command(command: UpgradeCommand) -> Result<()> {
    match command {
        UpgradeCommand::PlanNameMove {
            ledger,
            registry,
            session_id,
            name,
            from_ref,
            to_ref,
            source_gate_receipts,
            out,
        } => {
            if source_gate_receipts.is_empty() {
                return Err(MoltenError::invalid_harness(
                    "upgrade plan-name-move requires --source-gate-receipt for strict Octet source-gate validation",
                ));
            }
            let source_gate_receipt_values =
                source_gate_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let plan = upgrades::name_move_plan_value_with_registry(
                registry.as_deref(),
                &ledger,
                &upgrades::NameMovePlanInput {
                    session_id,
                    name: name.clone(),
                    from_ref,
                    to_ref,
                    initiator_ref: cli_upgrade_ref("initiator", &name)?,
                    capability_refs: vec![cli_upgrade_ref("capability", &name)?],
                    policy_refs: vec![cli_upgrade_ref("policy", &name)?],
                    evidence_refs: vec![cli_upgrade_ref("transcript", &name)?],
                    source_gate_receipt_values,
                },
            )?;
            let plan_ref = canonical_hash(&plan)?;
            write_file(&out, &to_text(&plan)?)?;
            println!("upgrade plan-name-move ok plan={} out={}", plan_ref, out.display());
            Ok(())
        }
        UpgradeCommand::Create {
            plan,
            store,
            receipt_out,
        } => {
            let plan_value = read_preserves_file(&plan)?;
            let created = upgrades::create_session(&store, &plan_value)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &created.receipt.value)?;
            println!(
                "upgrade create ok session={} plan={} tasks={} store={}",
                created.plan.session_id,
                created.plan.plan_ref,
                created.plan.tasks.len(),
                store.display()
            );
            Ok(())
        }
        UpgradeCommand::SetName {
            store,
            name,
            artifact_ref,
            receipt_out,
        } => {
            let receipt = upgrades::set_name_pointer(&store, &name, &artifact_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!("upgrade set-name ok name={} artifact={} store={}", name, artifact_ref, store.display());
            Ok(())
        }
        UpgradeCommand::RunTask {
            store,
            ledger,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let executed = upgrades::execute_task(&store, &ledger, &plan_ref, &task_id)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &executed.receipt.value)?;
            println!(
                "upgrade run-task ok plan={} task={} kind={} decision={}",
                executed.plan_ref, executed.task_id, executed.task_kind, executed.receipt.decision
            );
            Ok(())
        }
        UpgradeCommand::Rollback {
            store,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let receipt = upgrades::rollback_task(&store, &plan_ref, &task_id)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade rollback {} plan={} task={} receipt={}",
                receipt.decision, plan_ref, task_id, receipt.receipt_ref
            );
            Ok(())
        }
        UpgradeCommand::Status { store, plan_ref } => {
            let status = upgrades::status(&store, &plan_ref)?;
            println!(
                "upgrade status session={} plan={} remaining={}",
                status.session_id,
                status.plan_ref,
                status.remaining_task_ids.len()
            );
            for task in status.tasks {
                println!(
                    "{} {} {} {}",
                    task.task_id,
                    task.kind,
                    if task.done { "done" } else { "todo" },
                    task.receipt_ref.unwrap_or_else(|| "-".to_string())
                );
            }
            Ok(())
        }
        UpgradeCommand::CleanupCheck {
            store,
            ledger,
            registry,
            artifact_ref,
            receipt_out,
        } => {
            let receipt =
                upgrades::cleanup_admission_with_registry(&store, &ledger, registry.as_deref(), &artifact_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade cleanup-check {} artifact={} receipt={}",
                receipt.decision, artifact_ref, receipt.receipt_ref
            );
            Ok(())
        }
    }
}

fn cli_upgrade_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("upgrade-cli-ref", vec![string(kind), string(label)]))
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
