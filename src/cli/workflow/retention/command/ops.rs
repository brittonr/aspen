use std::path::PathBuf;

use clap::Args;

use crate::RetentionEvidenceArgs;

#[derive(Debug, Args)]
pub(crate) struct Explain {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: Option<String>,
    #[arg(long)]
    pub(crate) retention_class: Option<String>,
    #[arg(long)]
    pub(crate) action: Option<String>,
    #[arg(long)]
    pub(crate) subsystem: Option<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct BundleExport {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) explain: PathBuf,
    #[arg(long)]
    pub(crate) out: PathBuf,
    #[arg(long, default_value = "internal")]
    pub(crate) profile: String,
}

#[derive(Debug, Args)]
pub(crate) struct BundleVerify {
    #[arg(long)]
    pub(crate) bundle: PathBuf,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct GcPlan {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = "generic")]
    pub(crate) subsystem: String,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long, default_value = "delete")]
    pub(crate) action: String,
    #[command(flatten)]
    pub(crate) retention: RetentionEvidenceArgs,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct GcApplyPlan {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) plan_ref: String,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct GcAudit {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) execution_ref: String,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct Check {
    #[arg(long)]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) object_ref: String,
    #[arg(long)]
    pub(crate) object_kind: String,
    #[arg(long)]
    pub(crate) retention_class: String,
    #[arg(long, default_value = "eligibility")]
    pub(crate) action: String,
    #[arg(long)]
    pub(crate) requester_ref: String,
    #[arg(long = "reference-index-complete", default_value = "true")]
    pub(crate) is_reference_index_complete: bool,
    #[arg(long = "retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "remote-ref")]
    pub(crate) remote_refs: Vec<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long, default_value = "false")]
    pub(crate) has_delete_authority: bool,
    #[arg(long = "remote-gc-clearance")]
    pub(crate) has_remote_gc_clearance: bool,
    #[arg(long)]
    pub(crate) receipt_out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RunFixture {
    #[arg(long)]
    pub(crate) out: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct Show {
    pub(crate) artifact: PathBuf,
}
