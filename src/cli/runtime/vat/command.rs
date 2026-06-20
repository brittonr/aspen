type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    RunFixture {
        #[arg(long)]
        out: FilePath,
    },
    SnapshotFixture {
        #[arg(long)]
        out: FilePath,
    },
    RestoreFixture {
        #[arg(long)]
        out: FilePath,
    },
    PromiseFixture {
        #[arg(long)]
        out: FilePath,
    },
    AmbientAuthorityFixture {
        #[arg(long)]
        out: FilePath,
    },
    RightsFixture {
        #[arg(long)]
        out: FilePath,
    },
    DistributedRefFixture {
        #[arg(long)]
        out: FilePath,
    },
    TimeTravelFixture {
        #[arg(long)]
        out: FilePath,
    },
    ReplayFixture {
        #[arg(long)]
        out: FilePath,
    },
    AuthorityGraphFixture {
        #[arg(long)]
        out: FilePath,
    },
    PortableStorageFixture {
        #[arg(long)]
        out: FilePath,
    },
    Show {
        report: FilePath,
    },
}
