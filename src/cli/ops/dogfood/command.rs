#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    LocalNode {
        #[arg(long)]
        state_root: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        release_gate_out: Option<std::path::PathBuf>,
        #[arg(long)]
        replay_verify_out: Option<std::path::PathBuf>,
        #[arg(long)]
        replay_index_out: Option<std::path::PathBuf>,
    },
    NixReleaseExport {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    NixReleaseVerify {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        evidence: std::path::PathBuf,
        #[arg(long)]
        receipt_out: std::path::PathBuf,
    },
    ReleaseBundleExport {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    ReleaseBundleVerify {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        bundle: std::path::PathBuf,
        #[arg(long)]
        receipt_out: std::path::PathBuf,
        #[arg(long = "signed-member")]
        signed_members: Vec<std::path::PathBuf>,
        #[arg(long)]
        require_signed_members: bool,
        #[arg(long, default_value = "release-evidence")]
        signed_purpose: String,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long, default_value = "local-release-key")]
        signed_key: String,
        #[arg(long)]
        signed_key_ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleasePromote {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        bundle_verify: std::path::PathBuf,
        #[arg(long)]
        receipt_out: std::path::PathBuf,
        #[arg(long)]
        signed_key_ledger: std::path::PathBuf,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
        #[arg(long)]
        source_evidence: String,
        #[arg(long)]
        octet_evidence: String,
        #[arg(long)]
        cairn_evidence: String,
    },
    ReleasePromotionSummary {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        signed_key_ledger: Option<std::path::PathBuf>,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleaseExport {
        #[arg(long)]
        output_path: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        manifest_out: std::path::PathBuf,
    },
    ReleaseExportVerify {
        #[arg(long)]
        bundle: std::path::PathBuf,
        #[arg(long)]
        receipt_out: std::path::PathBuf,
    },
    Show {
        artifact: std::path::PathBuf,
    },
}
