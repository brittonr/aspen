type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Install {
        manifest: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    RunRequestResponse {
        #[arg(long)]
        out: FilePath,
    },
    GateLifecycle {
        dir: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Show {
        receipt: FilePath,
    },
}
