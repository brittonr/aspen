type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Record {
        #[arg(long)]
        out: FilePath,
    },
    Verify {
        fixture: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Tamper {
        fixture: FilePath,
        #[arg(long, default_value = "effect-response")]
        kind: String,
        #[arg(long)]
        out: FilePath,
    },
    Rollup {
        #[arg(long = "receipt")]
        receipts: Vec<FilePath>,
        #[arg(long)]
        out: FilePath,
    },
    Index {
        #[arg(long = "receipt")]
        receipts: Vec<FilePath>,
        #[arg(long = "rollup")]
        rollups: Vec<FilePath>,
        #[arg(long)]
        out: FilePath,
    },
    Show {
        report: FilePath,
    },
}
