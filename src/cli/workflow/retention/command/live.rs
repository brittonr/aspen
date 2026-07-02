type PathBuf = std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub(crate) struct RequestSend {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) requester_node_root: Option<PathBuf>,
    #[arg(long)]
    pub(crate) peer_ticket: PathBuf,
    #[arg(long)]
    pub(crate) requester_node_id: String,
    #[arg(long)]
    pub(crate) peer_node_id: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
    pub(crate) max_attempts: u64,
    #[arg(long, default_value_t = 10_000)]
    pub(crate) join_timeout_ms: u64,
    #[arg(long)]
    pub(crate) requester_ref: String,
    #[arg(long)]
    pub(crate) peer_ref: String,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long)]
    pub(crate) action: String,
    #[arg(long)]
    pub(crate) remote_ref: String,
    #[arg(long)]
    pub(crate) policy_ref: String,
    #[arg(long)]
    pub(crate) authority_ref: String,
    #[arg(long = "retention-evidence-ref")]
    pub(crate) retention_evidence_refs: Vec<String>,
    #[arg(long = "peer-bootstrap-ref")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
    #[arg(long = "authority")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "resource")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "transport-evidence-ref")]
    pub(crate) transport_evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) request_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) control_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) transport_receipt_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ResponseSend {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) peer_node_root: Option<PathBuf>,
    #[arg(long)]
    pub(crate) requester_ticket: PathBuf,
    pub(crate) request: PathBuf,
    #[arg(long)]
    pub(crate) peer_node_id: String,
    #[arg(long)]
    pub(crate) requester_node_id: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
    pub(crate) max_attempts: u64,
    #[arg(long, default_value_t = 10_000)]
    pub(crate) join_timeout_ms: u64,
    #[arg(long = "response-evidence-ref")]
    pub(crate) response_evidence_refs: Vec<String>,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "stale")]
    pub(crate) is_stale: bool,
    #[arg(long = "revoked-ref")]
    pub(crate) revoked_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long = "peer-bootstrap-ref")]
    pub(crate) peer_bootstrap_refs: Vec<String>,
    #[arg(long = "authority")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "policy")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "resource")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long = "transport-evidence-ref")]
    pub(crate) transport_evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) response_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) control_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) transport_receipt_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ImportWorkflow {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) request: PathBuf,
    #[arg(long)]
    pub(crate) response: PathBuf,
    #[arg(long)]
    pub(crate) request_control: PathBuf,
    #[arg(long)]
    pub(crate) request_send_receipt: PathBuf,
    #[arg(long)]
    pub(crate) request_receive_receipt: PathBuf,
    #[arg(long)]
    pub(crate) request_ingress_ref: String,
    #[arg(long)]
    pub(crate) response_control: PathBuf,
    #[arg(long)]
    pub(crate) response_send_receipt: PathBuf,
    #[arg(long)]
    pub(crate) response_receive_receipt: PathBuf,
    #[arg(long)]
    pub(crate) response_ingress_ref: String,
    #[arg(long)]
    pub(crate) expected_peer_ref: Option<String>,
    #[arg(long)]
    pub(crate) expected_remote_ref: Option<String>,
    #[arg(long)]
    pub(crate) import_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Loopback {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) requester_node_root: PathBuf,
    #[arg(long)]
    pub(crate) peer_node_root: PathBuf,
    #[arg(long)]
    pub(crate) requester_node_id: String,
    #[arg(long)]
    pub(crate) peer_node_id: String,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) request_sequence: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) response_sequence: u64,
    #[arg(long)]
    pub(crate) requester_ref: String,
    #[arg(long)]
    pub(crate) peer_ref: String,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long)]
    pub(crate) action: String,
    #[arg(long)]
    pub(crate) remote_ref: String,
    #[arg(long)]
    pub(crate) policy_ref: String,
    #[arg(long)]
    pub(crate) authority_ref: String,
    #[arg(long = "retention-evidence-ref")]
    pub(crate) retention_evidence_refs: Vec<String>,
    #[arg(long = "response-evidence-ref")]
    pub(crate) response_evidence_refs: Vec<String>,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "stale")]
    pub(crate) is_stale: bool,
    #[arg(long = "revoked-ref")]
    pub(crate) revoked_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long = "request-peer-bootstrap-ref")]
    pub(crate) request_peer_bootstrap_refs: Vec<String>,
    #[arg(long = "request-authority-ref")]
    pub(crate) request_authority_refs: Vec<String>,
    #[arg(long = "request-policy-ref")]
    pub(crate) request_policy_refs: Vec<String>,
    #[arg(long = "request-resource-ref")]
    pub(crate) request_resource_refs: Vec<String>,
    #[arg(long = "request-transport-evidence-ref")]
    pub(crate) request_transport_evidence_refs: Vec<String>,
    #[arg(long = "response-peer-bootstrap-ref")]
    pub(crate) response_peer_bootstrap_refs: Vec<String>,
    #[arg(long = "response-authority-ref")]
    pub(crate) response_authority_refs: Vec<String>,
    #[arg(long = "response-policy-ref")]
    pub(crate) response_policy_refs: Vec<String>,
    #[arg(long = "response-resource-ref")]
    pub(crate) response_resource_refs: Vec<String>,
    #[arg(long = "response-transport-evidence-ref")]
    pub(crate) response_transport_evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) request_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) response_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) import_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}
