#[derive(Debug, clap::Args)]
pub(crate) struct Submit {
    #[arg(long)]
    pub(crate) job_id: String,
    #[arg(long)]
    pub(crate) operation_id: String,
    #[arg(long)]
    pub(crate) executable: String,
    #[arg(long = "input")]
    pub(crate) inputs: Vec<String>,
    #[arg(long, default_value = "chunk-manifest")]
    pub(crate) output_mode: String,
    #[arg(long = "input-schema-ref")]
    pub(crate) input_schema_refs: Vec<String>,
    #[arg(long = "output-schema-ref")]
    pub(crate) output_schema_refs: Vec<String>,
    #[arg(long = "effect-ref")]
    pub(crate) effect_manifest_refs: Vec<String>,
    #[arg(long, default_value = "local-echo-v1")]
    pub(crate) handler_profile: String,
    #[arg(long)]
    pub(crate) authority_context_ref: String,
    #[arg(long = "policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "provenance-ref")]
    pub(crate) provenance_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(crate) out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Execute {
    pub(crate) submission: std::path::PathBuf,
    #[arg(long)]
    pub(crate) chunks: std::path::PathBuf,
    #[arg(long)]
    pub(crate) ledger: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Status {
    #[arg(long)]
    pub(crate) ledger: std::path::PathBuf,
    #[arg(long)]
    pub(crate) job: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ReceiptShow {
    pub(crate) receipt_ref: String,
    #[arg(long)]
    pub(crate) ledger: std::path::PathBuf,
}
