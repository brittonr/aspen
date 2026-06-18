#[derive(Debug, clap::Args)]
pub(crate) struct Plan {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) source_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Loopback {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) source_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long = "provenance")]
    pub(crate) provenance_paths: Vec<std::path::PathBuf>,
    #[arg(long = "build-verification")]
    pub(crate) build_verification_paths: Vec<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) plan_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AdmitPlan {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) sync_ref: Option<String>,
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "resource-ref")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AdmitLoopback {
    pub(crate) job: String,
    #[arg(long)]
    pub(crate) target_registry: std::path::PathBuf,
    #[arg(long)]
    pub(crate) sync_ref: Option<String>,
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "resource-ref")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long)]
    pub(crate) plan_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ExecuteLoopback {
    pub(crate) job: String,
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
    #[arg(long, default_value = "peer:loopback")]
    pub(crate) target_peer: String,
    #[arg(long = "stage")]
    pub(crate) stages: Vec<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_refs: Vec<String>,
    #[arg(long = "resource-ref")]
    pub(crate) resource_refs: Vec<String>,
    #[arg(long)]
    pub(crate) request_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}
