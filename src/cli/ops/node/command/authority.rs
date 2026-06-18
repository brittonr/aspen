#[derive(Debug, clap::Args)]
pub(crate) struct Request {
    #[arg(long)]
    pub(crate) operation: String,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
    #[arg(long)]
    pub(crate) target: Option<String>,
    #[arg(long)]
    pub(crate) payload: Option<String>,
    #[arg(long = "authority")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "resource")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Provenance {
    #[arg(long)]
    pub(crate) artifact_ref: String,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GrantFixture {
    #[arg(long)]
    pub(crate) state_root: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) peer: String,
    #[arg(long)]
    pub(crate) node: String,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long, default_value = "*")]
    pub(crate) target_scope: String,
    #[arg(long, default_value = "*")]
    pub(crate) resource_scope: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) epoch: u64,
    #[arg(long)]
    pub(crate) expires_at: Option<u64>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "revocation")]
    pub(crate) revocation_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GrantImport {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) grant: std::path::PathBuf,
    #[arg(long)]
    pub(crate) peer: Option<String>,
    #[arg(long)]
    pub(crate) node: Option<String>,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long)]
    pub(crate) target_scope: Option<String>,
    #[arg(long)]
    pub(crate) resource_scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_epoch: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct PolicyFixture {
    #[arg(long)]
    pub(crate) state_root: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = 0)]
    pub(crate) max_restarts: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) restart_window_ticks: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) heartbeat_timeout_ticks: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) shutdown_drain_ticks: u64,
    #[arg(long)]
    pub(crate) allow_stale_lock_recovery: bool,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct TicketExport {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub(crate) struct TicketImport {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) ticket: std::path::PathBuf,
    #[arg(long)]
    pub(crate) peer_admission: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) expected_node: Option<String>,
    #[arg(long)]
    pub(crate) expected_topic: Option<String>,
    #[arg(long)]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long)]
    pub(crate) expected_peer: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_sequence: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct PeerAdmit {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long)]
    pub(crate) peer: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long)]
    pub(crate) expires_at: Option<u64>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
    pub(crate) ticket: std::path::PathBuf,
}
