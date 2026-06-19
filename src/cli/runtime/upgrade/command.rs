type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    PlanNameMove {
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        registry: Option<FilePath>,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long = "source-gate-receipt")]
        source_gate_receipts: Vec<FilePath>,
        #[arg(long)]
        out: FilePath,
    },
    Create {
        plan: FilePath,
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    SetName {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    RunTask {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Rollback {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Status {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        plan_ref: String,
    },
    CleanupCheck {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        registry: Option<FilePath>,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}
