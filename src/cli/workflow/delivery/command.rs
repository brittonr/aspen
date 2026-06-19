type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Scope {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: String,
        #[arg(long = "retention-ref")]
        retention_refs: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    OperationId {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Check {
        #[arg(long)]
        root: FilePath,
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        semantic_result_ref: Option<String>,
        #[arg(long, default_value = "deny")]
        gap_policy: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        root: FilePath,
    },
    Show {
        artifact: FilePath,
    },
}
