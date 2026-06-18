#[derive(Debug, clap::Args)]
pub(crate) struct Bundle {
    #[arg(long)]
    pub(crate) state_root: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) ticket: std::path::PathBuf,
    #[arg(long)]
    pub(crate) peer_admission: std::path::PathBuf,
    #[arg(long)]
    pub(crate) authority_grant: std::path::PathBuf,
    #[arg(long)]
    pub(crate) send_receipt: std::path::PathBuf,
    #[arg(long = "receive-receipt")]
    pub(crate) receive_receipts: Vec<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) listener_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) service_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Export {
    #[arg(long)]
    pub(crate) ticket: std::path::PathBuf,
    #[arg(long)]
    pub(crate) peer_admission: std::path::PathBuf,
    #[arg(long)]
    pub(crate) authority_grant: std::path::PathBuf,
    #[arg(long = "receipt")]
    pub(crate) receipt_values: Vec<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Verify {
    pub(crate) bundle: std::path::PathBuf,
    #[arg(long)]
    pub(crate) expected_node: Option<String>,
    #[arg(long)]
    pub(crate) expected_topic: Option<String>,
    #[arg(long)]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long)]
    pub(crate) expected_peer: Option<String>,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long)]
    pub(crate) target_scope: Option<String>,
    #[arg(long)]
    pub(crate) resource_scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_sequence: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_epoch: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Gate {
    pub(crate) bundle: std::path::PathBuf,
    #[arg(long)]
    pub(crate) verify_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) require_verify_receipt: bool,
    #[arg(long)]
    pub(crate) expected_node: Option<String>,
    #[arg(long)]
    pub(crate) expected_topic: Option<String>,
    #[arg(long)]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long)]
    pub(crate) expected_peer: Option<String>,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long)]
    pub(crate) target_scope: Option<String>,
    #[arg(long)]
    pub(crate) resource_scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_sequence: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_epoch: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Apply {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) bundle: std::path::PathBuf,
    #[arg(long)]
    pub(crate) gate_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) require_gate_receipt: bool,
    #[arg(long)]
    pub(crate) request: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) send: bool,
    #[arg(long)]
    pub(crate) from_peer: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) sequence: u64,
    #[arg(long = "operation-id")]
    pub(crate) operation_id: Option<String>,
    #[arg(long)]
    pub(crate) expected_node: Option<String>,
    #[arg(long)]
    pub(crate) expected_topic: Option<String>,
    #[arg(long)]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long)]
    pub(crate) expected_peer: Option<String>,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long)]
    pub(crate) target_scope: Option<String>,
    #[arg(long)]
    pub(crate) resource_scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_sequence: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_epoch: u64,
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
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
    pub(crate) max_attempts: u64,
    #[arg(long, default_value_t = 10_000)]
    pub(crate) join_timeout_ms: u64,
    #[arg(long)]
    pub(crate) send_receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Reconcile {
    pub(crate) apply_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) send_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) ingress_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) queue_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) control_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) expected_envelope: Option<String>,
    #[arg(long)]
    pub(crate) expected_operation: Option<String>,
    #[arg(long)]
    pub(crate) expected_request: Option<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AckExport {
    pub(crate) apply_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) send_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) ingress_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) queue_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) control_receipt: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) reconcile_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) out: std::path::PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AckImport {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) ack: std::path::PathBuf,
    #[arg(long)]
    pub(crate) expected_bundle: Option<String>,
    #[arg(long)]
    pub(crate) expected_envelope: Option<String>,
    #[arg(long)]
    pub(crate) expected_operation: Option<String>,
    #[arg(long)]
    pub(crate) expected_request: Option<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ProtocolGate {
    pub(crate) bundle: std::path::PathBuf,
    #[arg(long)]
    pub(crate) gate_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) apply_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) reconcile_receipt: std::path::PathBuf,
    #[arg(long)]
    pub(crate) ack: std::path::PathBuf,
    #[arg(long)]
    pub(crate) expected_envelope: Option<String>,
    #[arg(long)]
    pub(crate) expected_operation: Option<String>,
    #[arg(long)]
    pub(crate) expected_request: Option<String>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Import {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    pub(crate) bundle: std::path::PathBuf,
    #[arg(long)]
    pub(crate) expected_node: Option<String>,
    #[arg(long)]
    pub(crate) expected_topic: Option<String>,
    #[arg(long)]
    pub(crate) expected_endpoint: Option<String>,
    #[arg(long)]
    pub(crate) expected_peer: Option<String>,
    #[arg(long = "operation")]
    pub(crate) operations: Vec<String>,
    #[arg(long)]
    pub(crate) target_scope: Option<String>,
    #[arg(long)]
    pub(crate) resource_scope: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_sequence: u64,
    #[arg(long, default_value_t = 1)]
    pub(crate) as_of_epoch: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}
