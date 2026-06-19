type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Put(Put),
    Get(Get),
    Status(Status),
    List(List),
    Show(Show),
    Invalidate(Invalidate),
    IndexRebuild(IndexRebuild),
}

#[derive(Debug, clap::Args)]
pub(crate) struct Put {
    pub(crate) input: FilePath,
    #[arg(long)]
    pub(crate) cache: FilePath,
    #[arg(long)]
    pub(crate) output: Option<FilePath>,
    #[arg(long)]
    pub(crate) operation: String,
    #[arg(long, default_value = "v1")]
    pub(crate) version: String,
    #[arg(long = "dependency")]
    pub(crate) dependencies: Vec<String>,
    #[arg(long = "dependency-closure-hash")]
    pub(crate) dependency_closure_hash: Option<String>,
    #[arg(long = "handler-profile-ref")]
    pub(crate) handler_profile_ref: Option<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_refs: Vec<String>,
    #[arg(long = "revocation-ref")]
    pub(crate) revocation_refs: Vec<String>,
    #[arg(long = "tool-ref")]
    pub(crate) tool_ref: Option<String>,
    #[arg(long, default_value = "local")]
    pub(crate) tool_version: String,
    #[arg(long = "assumption-ref")]
    pub(crate) assumption_refs: Vec<String>,
    #[arg(long, default_value = "pure")]
    pub(crate) tier: String,
    #[arg(long, default_value = "pass")]
    pub(crate) status: String,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "diagnostic")]
    pub(crate) diagnostics: Vec<String>,
    #[arg(long)]
    pub(crate) key_out: Option<FilePath>,
    #[arg(long)]
    pub(crate) value_out: Option<FilePath>,
    #[arg(long)]
    pub(crate) receipt_out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Get {
    pub(crate) key_ref: String,
    #[arg(long)]
    pub(crate) cache: FilePath,
    #[arg(long = "current-policy-ref")]
    pub(crate) current_policy_refs: Vec<String>,
    #[arg(long = "current-capability-ref")]
    pub(crate) current_capability_refs: Vec<String>,
    #[arg(long = "current-revocation-ref")]
    pub(crate) current_revocation_refs: Vec<String>,
    #[arg(long = "semantic", default_value = "true")]
    pub(crate) semantic_enabled: bool,
    #[arg(long)]
    pub(crate) out: Option<FilePath>,
    #[arg(long)]
    pub(crate) receipt_out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Status {
    #[arg(long)]
    pub(crate) cache: FilePath,
}

#[derive(Debug, clap::Args)]
pub(crate) struct List {
    #[arg(long)]
    pub(crate) cache: FilePath,
    #[arg(long)]
    pub(crate) operation: Option<String>,
    #[arg(long)]
    pub(crate) tier: Option<String>,
    #[arg(long)]
    pub(crate) status: Option<String>,
    #[arg(long = "dependency-ref")]
    pub(crate) dependency_ref: Option<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_ref: Option<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_ref: Option<String>,
    #[arg(long = "revocation-ref")]
    pub(crate) revocation_ref: Option<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_ref: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Show {
    pub(crate) reference: String,
    #[arg(long)]
    pub(crate) cache: FilePath,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Invalidate {
    #[arg(long)]
    pub(crate) cache: FilePath,
    #[arg(long = "key-ref")]
    pub(crate) key_ref: Option<String>,
    #[arg(long = "dependency-ref")]
    pub(crate) dependency_ref: Option<String>,
    #[arg(long = "policy-ref")]
    pub(crate) policy_ref: Option<String>,
    #[arg(long = "capability-ref")]
    pub(crate) capability_ref: Option<String>,
    #[arg(long = "revocation-ref")]
    pub(crate) revocation_ref: Option<String>,
    #[arg(long)]
    pub(crate) operation: Option<String>,
    #[arg(long, default_value = "manual-invalidate")]
    pub(crate) reason: String,
    #[arg(long = "apply-ref")]
    pub(crate) apply_refs: Vec<String>,
    #[command(flatten)]
    pub(crate) retention: crate::RetentionEvidenceArgs,
    #[arg(long)]
    pub(crate) receipt_out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct IndexRebuild {
    #[arg(long)]
    pub(crate) cache: FilePath,
    #[arg(long)]
    pub(crate) receipt_out: Option<FilePath>,
}
