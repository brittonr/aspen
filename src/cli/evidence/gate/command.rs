type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Check {
        artifact: FilePath,
        #[arg(long)]
        failure_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}
