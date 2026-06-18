#[derive(Debug, clap::Args)]
pub(crate) struct Install {
    pub(crate) dag: std::path::PathBuf,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) artifact_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Show {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Run {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) storage: std::path::PathBuf,
    #[arg(long)]
    pub(crate) cache: std::path::PathBuf,
    #[arg(long)]
    pub(crate) chunks: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) ledger: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) output_request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Plan {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) output_request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Profile {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) cache: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) output_request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FusionPreview {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) output_request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}
