type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Envelope {
        #[command(subcommand)]
        command: EnvelopeCommand,
    },
    PublishLocal {
        #[arg(long)]
        transport_root: FilePath,
        #[arg(long)]
        envelope: FilePath,
        #[arg(long)]
        node: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    DeliverLocal {
        #[arg(long)]
        transport_root: FilePath,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        envelope_ref: String,
        #[arg(long)]
        receiver_peer: String,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    RunTwoPeer {
        #[arg(long)]
        transport_root: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    Gate {
        #[arg(long)]
        delivery_log: FilePath,
        #[arg(long = "admission-receipt")]
        admission_receipts: Vec<FilePath>,
        #[arg(long = "turn-context-ref")]
        turn_context_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum EnvelopeCommand {
    Build {
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        from_actor: String,
        #[arg(long)]
        to_peer: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        payload: FilePath,
        #[arg(long = "content-ref")]
        content_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: FilePath,
    },
}
