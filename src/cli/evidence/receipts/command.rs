#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    List {
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    Show {
        receipt_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    Validate {
        receipt_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    Export {
        receipt_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Key {
        #[command(subcommand)]
        command: Key,
    },
    Sign {
        receipt: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = molten::evidence::PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    VerifySigned {
        signed_receipt: std::path::PathBuf,
        #[arg(long, default_value = molten::evidence::PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Test {
    Sign {
        receipt: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = molten::evidence::PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    Verify {
        signed_receipt: std::path::PathBuf,
        #[arg(long, default_value = molten::evidence::PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Key {
    Import {
        #[arg(long)]
        ledger: std::path::PathBuf,
        #[arg(long)]
        key_id: String,
        #[arg(long)]
        signer: String,
        #[arg(long)]
        trust_root: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    List {
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    Show {
        key_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
    },
    Revoke {
        key_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
        #[arg(long, default_value = "operator-revoked")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
    Rotate {
        old_key_ref: String,
        #[arg(long)]
        ledger: std::path::PathBuf,
        #[arg(long)]
        new_key_id: String,
        #[arg(long)]
        new_key: String,
        #[arg(long, default_value = "rotated")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
    },
}
