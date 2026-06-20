type FilePath = std::path::PathBuf;
type BaselineCommand = super::baseline::Command;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Gate {
        #[arg(long, default_value = "target/octet")]
        artifacts: FilePath,
        #[arg(long, default_value = "strict-ci")]
        profile: String,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    SourceGate {
        #[command(subcommand)]
        command: SourceGate,
    },
    Baseline {
        #[command(subcommand)]
        command: BaselineCommand,
    },
    Review {
        #[command(subcommand)]
        command: Review,
    },
    Artifacts {
        #[command(subcommand)]
        command: Artifacts,
    },
    Remediation {
        #[command(subcommand)]
        command: Remediation,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Remediation {
    Plan {
        #[arg(long, default_value = "target/octet")]
        artifacts: FilePath,
        #[arg(long = "lib-artifacts")]
        lib_artifacts: Option<FilePath>,
        #[arg(long = "focused-object-corpus")]
        focused_object_corpus: Option<FilePath>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum SourceGate {
    Validate {
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        gate_receipt: FilePath,
        #[arg(long = "source-scope")]
        source_scope: Vec<String>,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Artifacts {
    Import {
        #[arg(long, default_value = "target/octet")]
        artifacts: FilePath,
        #[arg(long)]
        ledger: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Review {
    Write {
        #[arg(long)]
        out: FilePath,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long = "finding-key")]
        finding_keys: Vec<String>,
        #[arg(long, default_value = "manual review")]
        rationale: String,
    },
}
