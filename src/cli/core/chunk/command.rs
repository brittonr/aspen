#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Put {
        input: std::path::PathBuf,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long, default_value_t = molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE)]
        chunk_size: u64,
        #[arg(long)]
        manifest_out: Option<std::path::PathBuf>,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Verify {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Read {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Range {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        offset: u64,
        #[arg(long)]
        length: u64,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Sync {
        manifest_ref: String,
        #[arg(long)]
        from: std::path::PathBuf,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    IrohPublish {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        iroh_store: std::path::PathBuf,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    IrohFetch {
        ticket: String,
        #[arg(long)]
        iroh_store: std::path::PathBuf,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        expected_manifest_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Pin {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Unpin {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    PinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    UnpinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    IndexStatus {
        #[arg(long)]
        store: std::path::PathBuf,
    },
    IndexRebuild {
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    ReceiptList {
        #[arg(long)]
        store: std::path::PathBuf,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
    },
    Lineage {
        manifest_ref: String,
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        lineage_out: Option<std::path::PathBuf>,
    },
    Gc {
        #[arg(long)]
        store: std::path::PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: crate::RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
}
