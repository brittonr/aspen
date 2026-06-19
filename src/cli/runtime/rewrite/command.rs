type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Find {
        #[arg(long)]
        registry: FilePath,
        #[arg(long, default_value = "any")]
        pattern_kind: String,
        #[arg(long, default_value = "")]
        pattern: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        matches_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Preview {
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Apply {
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
        #[arg(long)]
        upgrade_plan_out: Option<FilePath>,
        #[arg(long, default_value = "rewrite-session")]
        session_id: String,
    },
    Show {
        artifact: FilePath,
    },
}
