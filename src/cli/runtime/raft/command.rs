type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    RunFixture {
        #[arg(long)]
        out: FilePath,
    },
    MembershipPreflight {
        #[arg(long)]
        out: FilePath,
        #[arg(long)]
        peer: String,
    },
    Show {
        artifact: FilePath,
    },
}
