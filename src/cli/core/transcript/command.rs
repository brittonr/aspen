type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Parse {
        markdown: FilePath,
        #[arg(long)]
        out: FilePath,
        #[arg(long = "dependency")]
        dependency_refs: Vec<String>,
        #[arg(long = "dependency-closure-hash")]
        dependency_closure_hash: Option<String>,
        #[arg(long = "artifact-ref")]
        artifact_refs: Vec<String>,
        #[arg(long = "schema-ref")]
        schema_refs: Vec<String>,
        #[arg(long = "handler-profile-ref")]
        handler_profile_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "effect-ref")]
        effect_manifest_refs: Vec<String>,
        #[arg(long = "revocation-ref")]
        revocation_refs: Vec<String>,
        #[arg(long = "seed-ref")]
        seed_ref: Option<String>,
        #[arg(long = "logical-time")]
        logical_time: Option<u64>,
        #[arg(long = "expected-ref")]
        expected_refs: Vec<String>,
        #[arg(long = "resolution-ref")]
        resolution_refs: Vec<String>,
    },
    Run {
        transcript: FilePath,
        #[arg(long)]
        cache: Option<FilePath>,
        #[arg(long, default_value = "fresh")]
        state: String,
        #[arg(long)]
        save_root: Option<FilePath>,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
        #[arg(long)]
        failure_out: Option<FilePath>,
    },
    Show {
        transcript: FilePath,
    },
    Render {
        transcript: FilePath,
        #[arg(long)]
        receipt: Option<FilePath>,
        #[arg(long)]
        out: FilePath,
    },
}
