#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Put {
        input: std::path::PathBuf,
        #[arg(long)]
        cache: std::path::PathBuf,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long, default_value = "v1")]
        version: String,
        #[arg(long = "dependency")]
        dependencies: Vec<String>,
        #[arg(long = "dependency-closure-hash")]
        dependency_closure_hash: Option<String>,
        #[arg(long = "handler-profile-ref")]
        handler_profile_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "revocation-ref")]
        revocation_refs: Vec<String>,
        #[arg(long = "tool-ref")]
        tool_ref: Option<String>,
        #[arg(long, default_value = "local")]
        tool_version: String,
        #[arg(long = "assumption-ref")]
        assumption_refs: Vec<String>,
        #[arg(long, default_value = "pure")]
        tier: String,
        #[arg(long, default_value = "pass")]
        status: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        key_out: Option<std::path::PathBuf>,
        #[arg(long)]
        value_out: Option<std::path::PathBuf>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Get {
        key_ref: String,
        #[arg(long)]
        cache: std::path::PathBuf,
        #[arg(long = "current-policy-ref")]
        current_policy_refs: Vec<String>,
        #[arg(long = "current-capability-ref")]
        current_capability_refs: Vec<String>,
        #[arg(long = "current-revocation-ref")]
        current_revocation_refs: Vec<String>,
        #[arg(long = "semantic", default_value = "true")]
        semantic_enabled: bool,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Status {
        #[arg(long)]
        cache: std::path::PathBuf,
    },
    List {
        #[arg(long)]
        cache: std::path::PathBuf,
        #[arg(long)]
        operation: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "revocation-ref")]
        revocation_ref: Option<String>,
        #[arg(long = "evidence-ref")]
        evidence_ref: Option<String>,
    },
    Show {
        reference: String,
        #[arg(long)]
        cache: std::path::PathBuf,
    },
    Invalidate {
        #[arg(long)]
        cache: std::path::PathBuf,
        #[arg(long = "key-ref")]
        key_ref: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "revocation-ref")]
        revocation_ref: Option<String>,
        #[arg(long)]
        operation: Option<String>,
        #[arg(long, default_value = "manual-invalidate")]
        reason: String,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: crate::RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    IndexRebuild {
        #[arg(long)]
        cache: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
}
