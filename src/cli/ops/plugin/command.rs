type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Install {
        manifest: FilePath,
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    RunFixture {
        #[arg(long)]
        state_root: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    Show {
        artifact: FilePath,
    },
}
