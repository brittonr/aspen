#[derive(Debug, clap::Args)]
pub(crate) struct Submit {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Dispatch {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long)]
    pub(crate) request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct IngressBuild {
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
    #[arg(long)]
    pub(crate) from_peer: String,
    #[arg(long)]
    pub(crate) to_node: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long = "peer-bootstrap")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
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
pub(crate) struct IngressLiveBuild {
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
    #[arg(long)]
    pub(crate) from_peer: String,
    #[arg(long)]
    pub(crate) to_node: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long = "peer-bootstrap")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
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
pub(crate) struct IngressLiveLoopback {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) from_peer: String,
    #[arg(long)]
    pub(crate) to_node: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long = "peer-bootstrap")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
    #[arg(long = "authority")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "resource")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) publish_receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receive_receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct IngressLiveSend {
    #[arg(long)]
    pub(crate) state_root: Option<std::path::PathBuf>,
    pub(crate) request: std::path::PathBuf,
    pub(crate) ticket: std::path::PathBuf,
    #[arg(long)]
    pub(crate) from_peer: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long = "operation-id")]
    pub(crate) operation_id: Option<String>,
    #[arg(long = "expected-node")]
    pub(crate) expected_node: Option<String>,
    #[arg(long = "expected-topic")]
    pub(crate) expected_topic: Option<String>,
    #[arg(long = "expected-endpoint")]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long = "topology-profile-ref")]
    pub(crate) topology_profile_ref: Option<String>,
    #[arg(long = "topology-profile-alpn")]
    pub(crate) topology_profile_alpns: Vec<String>,
    #[arg(long = "topology-profile-role")]
    pub(crate) topology_profile_role: Option<String>,
    #[arg(long = "transport-profile-ref")]
    pub(crate) transport_profile_ref: Option<String>,
    #[arg(long = "transport-profile-publish-timeout-ms")]
    pub(crate) transport_profile_publish_timeout_ms: Option<u64>,
    #[arg(long = "transport-profile-relay", default_value = "auto")]
    pub(crate) transport_profile_relay: String,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
    pub(crate) max_attempts: u64,
    #[arg(long = "peer-bootstrap")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
    #[arg(long = "authority")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "resource")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "evidence")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long, default_value_t = 10_000)]
    pub(crate) join_timeout_ms: u64,
    #[arg(long)]
    pub(crate) transport_receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) retry_receipts_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) duplicate_receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct IngressPublish {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) envelope: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct IngressDeliver {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    pub(crate) envelope_ref: String,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Deny {
    pub(crate) request: std::path::PathBuf,
    #[arg(long)]
    pub(crate) startup: String,
    #[arg(long)]
    pub(crate) diagnostic: String,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}
