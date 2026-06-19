#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    BuildRecord {
        #[arg(long)]
        expected_artifact_ref: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long = "build-param")]
        build_params: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "nix-derivation-ref")]
        nix_derivation_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    VerifyBuild {
        build_record: std::path::PathBuf,
        #[arg(long)]
        actual_artifact_ref: String,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Record {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        trust_state: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "review-ref")]
        review_refs: Vec<String>,
        #[arg(long = "test-ref")]
        test_refs: Vec<String>,
        #[arg(long = "source-gate-ref")]
        source_gate_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "build-record-ref")]
        build_record_refs: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    Fixture {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    Evaluate {
        #[arg(long)]
        operation: String,
        #[arg(long, default_value = "node-control")]
        profile: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long = "provenance")]
        provenance_paths: Vec<std::path::PathBuf>,
        #[arg(long = "build-verification")]
        build_verification_paths: Vec<std::path::PathBuf>,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Show {
        artifact: std::path::PathBuf,
    },
}
