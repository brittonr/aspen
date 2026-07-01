use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub(crate) struct Class {
    #[arg(long)]
    pub(crate) class_name: String,
    #[arg(long, default_value_t = 0)]
    pub(crate) minimum_age_seconds: u64,
    #[arg(long)]
    pub(crate) maximum_age_seconds: Option<u64>,
    #[arg(long)]
    pub(crate) deletion_authority_ref: String,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "secret-redaction-hook", default_value = "false")]
    pub(crate) has_secret_redaction_hook: bool,
    #[arg(long = "remote-gc-plan", default_value = "false")]
    pub(crate) has_remote_gc_plan: bool,
    #[arg(long = "compaction", default_value = "false")]
    pub(crate) has_compaction: bool,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Pin {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long)]
    pub(crate) source: String,
    #[arg(long)]
    pub(crate) reason: String,
    #[arg(long)]
    pub(crate) owner_ref: String,
    #[arg(long)]
    pub(crate) expiry_ref: Option<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long, default_value = "true")]
    pub(crate) has_authority: bool,
    #[arg(long)]
    pub(crate) pin_out: Option<PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Unpin {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) pin_ref: String,
    #[arg(long)]
    pub(crate) requester_ref: String,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long, default_value = "true")]
    pub(crate) has_authority: bool,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Admit {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) kind: String,
    #[arg(long, default_value = "pass")]
    pub(crate) decision: String,
    #[arg(long)]
    pub(crate) requester_ref: String,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long)]
    pub(crate) action: String,
    #[arg(long = "bound-ref")]
    pub(crate) bound_refs: Vec<String>,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "remote-ref")]
    pub(crate) remote_refs: Vec<String>,
    #[arg(long = "reference-index-complete")]
    pub(crate) is_reference_index_complete: bool,
    #[arg(long = "stale")]
    pub(crate) is_stale: bool,
    #[arg(long = "revoked-ref")]
    pub(crate) revoked_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Record {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = "pass")]
    pub(crate) decision: String,
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
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "stale")]
    pub(crate) is_stale: bool,
    #[arg(long = "revoked-ref")]
    pub(crate) revoked_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Request {
    #[arg(long)]
    pub(crate) root: PathBuf,
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
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Respond {
    #[arg(long)]
    pub(crate) root: PathBuf,
    pub(crate) request: PathBuf,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "stale")]
    pub(crate) is_stale: bool,
    #[arg(long = "revoked-ref")]
    pub(crate) revoked_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Import {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) request: PathBuf,
    #[arg(long)]
    pub(crate) response: PathBuf,
    #[arg(long)]
    pub(crate) expected_peer_ref: Option<String>,
    #[arg(long)]
    pub(crate) expected_remote_ref: Option<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}
