#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum ProfileArg {
    Live,
    DeterministicSimulation,
    Both,
}

impl From<ProfileArg> for molten::fabric_time::Selection {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Live => Self::Live,
            ProfileArg::DeterministicSimulation => Self::DeterministicSimulation,
            ProfileArg::Both => Self::Both,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ClockAction {
    /// Exercise live and/or deterministic time adapters and emit canonical evidence.
    RunFixture {
        #[arg(long, value_enum, default_value = "both")]
        profile: ProfileArg,
        #[arg(long, default_value = "artifacts/fabric-time-fixture")]
        out: std::path::PathBuf,
    },
    /// Read and validate a canonical fabric-time run report.
    Show { report: std::path::PathBuf },
}
