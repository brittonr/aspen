type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Put {
        value: FilePath,
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        producer_ref: Option<String>,
        #[arg(long)]
        ref_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Get {
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        migration_recipe: Option<FilePath>,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Recipe {
        #[arg(long)]
        source_schema_ref: String,
        #[arg(long)]
        target_schema_ref: String,
        #[arg(long)]
        transformer_ref: String,
        #[arg(long, default_value = "schema-rename")]
        transformer_kind: String,
        #[arg(long, default_value = "explicit")]
        mode: String,
        #[arg(long)]
        out: FilePath,
    },
    Migrate {
        recipe: FilePath,
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        ref_out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Verify {
        storage_ref: String,
        #[arg(long)]
        store: FilePath,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}
