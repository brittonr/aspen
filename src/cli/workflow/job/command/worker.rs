#[derive(Debug, clap::Args)]
pub(crate) struct Request {
    #[arg(long)]
    pub(crate) admission_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) execution_request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) sync_ref: Option<String>,
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long = "authority-ref")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "resource-ref")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "peer-bootstrap-ref")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
    #[arg(long = "node-identity-ref")]
    pub(crate) node_identity_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RunLocal {
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) storage: std::path::PathBuf,
    #[arg(long)]
    pub(crate) cache: std::path::PathBuf,
    #[arg(long)]
    pub(crate) chunks: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) admission_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) execution_request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) transport_root: std::path::PathBuf,
    #[arg(long, default_value = "peer:source")]
    pub(crate) from_peer: String,
    #[arg(long, default_value = "source-worker")]
    pub(crate) from_actor: String,
    #[arg(long, default_value = "molten.job.worker")]
    pub(crate) topic: String,
    #[arg(long)]
    pub(crate) ledger: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ScheduleLocal {
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) storage: std::path::PathBuf,
    #[arg(long)]
    pub(crate) cache: std::path::PathBuf,
    #[arg(long)]
    pub(crate) chunks: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) admission_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) execution_request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) transport_root: std::path::PathBuf,
    #[arg(long, default_value = "queue:job-worker")]
    pub(crate) queue_key: String,
    #[arg(long)]
    pub(crate) lease_key: Option<String>,
    #[arg(long, default_value = "scheduler")]
    pub(crate) scheduler_session: String,
    #[arg(long, default_value = "worker")]
    pub(crate) worker_session: String,
    #[arg(long)]
    pub(crate) lease_token: Option<u64>,
    #[arg(long, default_value = "peer:source")]
    pub(crate) from_peer: String,
    #[arg(long, default_value = "source-worker")]
    pub(crate) from_actor: String,
    #[arg(long, default_value = "molten.job.worker")]
    pub(crate) topic: String,
    #[arg(long = "coordination-authority-ref")]
    pub(crate) coordination_authority_refs: Vec<String>,
    #[arg(long = "coordination-resource-ref")]
    pub(crate) coordination_resource_refs: Vec<String>,
    #[arg(long = "coordination-policy-ref")]
    pub(crate) coordination_policy_refs: Vec<String>,
    #[arg(long)]
    pub(crate) ledger: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}
