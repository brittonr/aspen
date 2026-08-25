type FilePath = std::path::PathBuf;

#[allow(
    clippy::large_enum_variant,
    reason = "Clap owns the operator input shape; boxing individual release fields would obscure argument ownership"
)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Check {
        artifact: FilePath,
        #[arg(long)]
        failure_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    ReleaseProfile {
        #[arg(long)]
        profile_id: String,
        #[arg(long)]
        tier: String,
        #[arg(long)]
        candidate_ref: Option<String>,
        #[arg(long)]
        source_gate_ref: Option<String>,
        #[arg(long)]
        policy_ref: Option<String>,
        #[arg(long)]
        octet_ref: Option<String>,
        #[arg(long)]
        cairn_ref: Option<String>,
        #[arg(long)]
        stack_provenance_ref: Option<String>,
        #[arg(long)]
        production_profile_ref: Option<String>,
        #[arg(long)]
        expected_generated_export_ref: Option<String>,
        #[arg(long)]
        actual_generated_export_ref: Option<String>,
        #[arg(long)]
        stack_provenance_required: bool,
        #[arg(long = "accepted-valence-policy-hash")]
        accepted_valence_policy_hashes: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
}
