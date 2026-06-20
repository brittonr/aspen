type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    RunFixture {
        #[arg(long)]
        out: FilePath,
    },
    Show {
        artifact: FilePath,
    },
}
