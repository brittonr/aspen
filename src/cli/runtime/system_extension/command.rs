#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum Profile {
    InProcessNative,
    NativeProcess,
    SandboxedComponent,
}

impl From<Profile> for molten::system_extension::ExecutionProfile {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::InProcessNative => Self::InProcessNative,
            Profile::NativeProcess => Self::NativeProcess,
            Profile::SandboxedComponent => Self::SandboxedComponent,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    /// Execute lifecycle and request callbacks and write canonical evidence.
    RunFixture {
        #[arg(long, value_enum, default_value = "sandboxed-component")]
        profile: Profile,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Show bounded, secret-free fields from a canonical status artifact.
    Show {
        #[arg(long)]
        status: std::path::PathBuf,
    },
}
