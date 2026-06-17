#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    List {
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    View {
        reference: String,
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long = "payload")]
        payload_inclusion_enabled: bool,
        #[arg(long = "redacted", default_value = "true")]
        redaction_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Search {
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long = "kind")]
        artifact_kind: Option<String>,
        #[arg(long = "ledger-kind")]
        ledger_kind: Option<String>,
        #[arg(long = "schema-ref")]
        schema_ref: Option<String>,
        #[arg(long = "structural-fingerprint")]
        structural_fingerprint: Option<String>,
        #[arg(long = "effect-ref")]
        effect_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "evidence-ref")]
        evidence_ref: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "dependent-ref")]
        dependent_ref: Option<String>,
        #[arg(long = "receipt-operation")]
        receipt_operation: Option<String>,
        #[arg(long = "receipt-decision")]
        receipt_decision: Option<String>,
        #[arg(long = "transcript-status")]
        transcript_status: Option<String>,
        #[arg(long = "upgrade-status")]
        upgrade_status: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "include-dependents", default_value = "true")]
        dependent_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Deps {
        reference: String,
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        transitive: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Dependents {
        reference: String,
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        transitive: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    ShortId {
        prefix: String,
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long, default_value_t = molten::catalog::DEFAULT_SHORT_ID_MIN_LENGTH)]
        min_length: usize,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Chunks {
        #[arg(long)]
        chunks: std::path::PathBuf,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    McpCall {
        request: std::path::PathBuf,
        #[arg(long)]
        registry: std::path::PathBuf,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        chunks: Option<std::path::PathBuf>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Show {
        artifact: std::path::PathBuf,
    },
}
