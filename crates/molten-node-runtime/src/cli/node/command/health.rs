#[derive(Debug, clap::Args)]
pub(crate) struct Shutdown {
    #[arg(long)]
    pub(crate) startup: String,
    #[arg(long = "adapter")]
    pub(crate) adapters: Vec<String>,
    #[arg(long = "drained-job")]
    pub(crate) drained_jobs: Vec<String>,
    #[arg(long = "index")]
    pub(crate) index_receipt_refs: Vec<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Restart {
    pub(crate) startup_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) shutdown: Option<String>,
    #[arg(long = "index")]
    pub(crate) index_receipt_refs: Vec<String>,
    #[arg(long = "head")]
    pub(crate) head_refs: Vec<String>,
    #[arg(long = "open-job")]
    pub(crate) open_job_refs: Vec<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}
