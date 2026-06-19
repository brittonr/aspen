type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Show {
        report: FilePath,
    },
    Validate {
        report: FilePath,
        #[arg(long)]
        failure_out: Option<FilePath>,
    },
}
