#[derive(Debug, clap::Args)]
pub(crate) struct RouterFixture {
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FrameFixture {
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct DiagnosticsFixture {
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct MetricsFixture {
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct PortMappingFixture {
    #[arg(long)]
    pub(crate) attempt: bool,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ExternalBridgeFixture {
    #[arg(long)]
    pub(crate) enable: bool,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}
