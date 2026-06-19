type FilePath = std::path::PathBuf;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Install {
        payload: FilePath,
        #[arg(long)]
        registry: FilePath,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long = "dependency")]
        dependencies: Vec<String>,
        #[arg(long = "schema-ref")]
        schema_refs: Vec<String>,
        #[arg(long)]
        effect_manifest_ref: Option<String>,
        #[arg(long)]
        artifact_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    List {
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        kind: Option<String>,
    },
    View {
        artifact_ref: String,
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        payload: bool,
    },
    NameSet {
        #[arg(long)]
        registry: FilePath,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    NameShow {
        #[arg(long)]
        registry: FilePath,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
    },
    Deps {
        artifact_ref: String,
        #[arg(long)]
        registry: FilePath,
    },
    Closure {
        artifact_ref: String,
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Impact {
        artifact_ref: String,
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    IndexRebuild {
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}
