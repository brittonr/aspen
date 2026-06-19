type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Identity {
        shape: FilePath,
        #[arg(long)]
        schema_ref: String,
        #[arg(long, default_value = "structural")]
        mode: String,
        #[arg(long)]
        brand_ref: Option<String>,
        #[arg(long)]
        out: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Alias {
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long, default_value = "storage")]
        scope: String,
        #[arg(long)]
        out: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Compat {
        #[arg(long)]
        expected_identity: FilePath,
        #[arg(long)]
        actual_identity: FilePath,
        #[arg(long)]
        alias: Option<FilePath>,
        #[arg(long)]
        migration_ref: Option<String>,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    SearchFingerprint {
        #[arg(long)]
        registry: FilePath,
        #[arg(long)]
        fingerprint: String,
    },
}
