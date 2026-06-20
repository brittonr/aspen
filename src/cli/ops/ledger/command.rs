type FilePath = std::path::PathBuf;
type RetentionEvidenceArgs = crate::RetentionEvidenceArgs;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Import {
        artifact: FilePath,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Export {
        artifact_ref: String,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        out: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    List {
        #[arg(long)]
        ledger: FilePath,
    },
    Pin {
        artifact_ref: String,
        #[arg(long)]
        ledger: FilePath,
    },
    Gc {
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Chain {
    Publish {
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        iroh_store: FilePath,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        epoch: String,
        #[arg(long)]
        anchor: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Fetch {
        ticket: String,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        iroh_store: FilePath,
        #[arg(long)]
        expected_bundle_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}
