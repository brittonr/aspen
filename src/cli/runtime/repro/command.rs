#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Export {
        report: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long, default_value = "deny-sensitive")]
        profile: String,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
    },
    Verify {
        bundle: std::path::PathBuf,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Unpack {
        bundle: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long = "reveal-receipt")]
        reveal_receipts: Vec<std::path::PathBuf>,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
    },
    Publish {
        bundle: std::path::PathBuf,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
    },
    Fetch {
        ticket: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        expected_bundle_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
    },
}
