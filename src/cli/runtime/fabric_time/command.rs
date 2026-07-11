#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum FabricTimeProfileArg {
    Live,
    DeterministicSimulation,
    Both,
}

impl From<FabricTimeProfileArg> for molten::fabric_time::FabricTimeFixtureSelection {
    fn from(value: FabricTimeProfileArg) -> Self {
        match value {
            FabricTimeProfileArg::Live => Self::Live,
            FabricTimeProfileArg::DeterministicSimulation => Self::DeterministicSimulation,
            FabricTimeProfileArg::Both => Self::Both,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum FabricTimeCommand {
    /// Exercise live and/or deterministic time adapters and emit canonical evidence.
    RunFixture {
        #[arg(long, value_enum, default_value = "both")]
        profile: FabricTimeProfileArg,
        #[arg(long, default_value = "artifacts/fabric-time-fixture")]
        out: std::path::PathBuf,
    },
    /// Read and validate a canonical fabric-time run report.
    Show { report: std::path::PathBuf },
}
