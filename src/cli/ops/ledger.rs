use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::evidence_chain::ChainForkPolicy;
use molten::evidence_chain::ChainScope;
use molten::iroh_exchange::FetchChainSegmentInput;
use molten::iroh_exchange::PublishChainSegmentInput;
use molten::iroh_exchange::fetch_chain_segment;
use molten::iroh_exchange::publish_chain_segment;
use molten::ledger;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

use crate::RetentionEvidenceArgs;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum LedgerCommand {
    Import {
        artifact: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Export {
        artifact_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Pin {
        artifact_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Gc {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ChainCommand {
    Publish {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        epoch: String,
        #[arg(long)]
        anchor: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Fetch {
        ticket: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        expected_bundle_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_ledger_command(command: LedgerCommand) -> Result<()> {
    match command {
        LedgerCommand::Import {
            artifact,
            ledger,
            receipt_out,
        } => {
            let artifact_value = read_preserves_file(&artifact)?;
            let imported = ledger::import_artifact(&ledger, &artifact_value)?;
            emit_named_receipt(receipt_out.as_ref(), "ledger import receipt", &imported.receipt_value)?;
            println!(
                "ledger import ok artifact={} kind={} ledger={}",
                imported.artifact_ref,
                imported.artifact_kind,
                ledger.display()
            );
            Ok(())
        }
        LedgerCommand::Export {
            artifact_ref,
            ledger,
            out,
            receipt_out,
        } => {
            let exported = ledger::export_artifact(&ledger, &artifact_ref, &out)?;
            emit_named_receipt(receipt_out.as_ref(), "ledger export receipt", &exported.receipt_value)?;
            println!(
                "ledger export ok artifact={} kind={} out={}",
                exported.artifact_ref,
                exported.artifact_kind,
                out.display()
            );
            Ok(())
        }
        LedgerCommand::List { ledger } => {
            for entry in ledger::list_artifacts(&ledger)? {
                println!("{} {}", entry.artifact_ref, entry.artifact_kind);
            }
            Ok(())
        }
        LedgerCommand::Pin { artifact_ref, ledger } => {
            ledger::pin_artifact(&ledger, &artifact_ref)?;
            println!("ledger pin ok artifact={} ledger={}", artifact_ref, ledger.display());
            Ok(())
        }
        LedgerCommand::Gc {
            ledger,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = ledger::gc(&ledger, ledger::LedgerGcInput {
                dry_run,
                retention_evidence: &retention_evidence,
                apply_refs: &apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "ledger gc receipt", &gc.receipt_value)?;
            println!(
                "ledger gc ok decision={} dry_run={} removed={} retention_receipts={}",
                gc.decision,
                gc.dry_run,
                gc.removed_refs.len(),
                gc.retention_receipt_refs.len()
            );
            Ok(())
        }
    }
}

pub(crate) fn run_chain_command(command: ChainCommand) -> Result<()> {
    match command {
        ChainCommand::Publish {
            ledger,
            iroh_store,
            scope,
            id,
            epoch,
            anchor,
            head,
            node,
            fork_policy,
            receipt_out,
        } => {
            let chain = ChainScope::new(scope, id, epoch);
            let policy = parse_chain_fork_policy(&fork_policy)?;
            let published = publish_chain_segment(&PublishChainSegmentInput {
                iroh_root: &iroh_store,
                ledger_root: &ledger,
                chain: &chain,
                anchor_ref: anchor.as_deref(),
                expected_head: head.as_deref(),
                node: &node,
                fork_policy: policy,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "iroh chain exchange receipt", &published.receipt_value)?;
            println!(
                "chain publish ok ticket={} bundle={} chain={}/{}/{}",
                published.ticket,
                published.bundle_ref,
                published.chain.scope,
                published.chain.id,
                published.chain.epoch
            );
            Ok(())
        }
        ChainCommand::Fetch {
            ticket,
            ledger,
            iroh_store,
            expected_bundle_ref,
            peer,
            fork_policy,
            receipt_out,
        } => {
            let policy = parse_chain_fork_policy(&fork_policy)?;
            let fetched = fetch_chain_segment(&FetchChainSegmentInput {
                iroh_root: &iroh_store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                ledger_root: &ledger,
                fork_policy: policy,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "iroh chain exchange receipt", &fetched.receipt_value)?;
            println!(
                "chain fetch ok ticket={} bundle={} chain={}/{}/{}",
                fetched.ticket, fetched.bundle_ref, fetched.chain.scope, fetched.chain.id, fetched.chain.epoch
            );
            Ok(())
        }
    }
}

fn parse_chain_fork_policy(value: &str) -> Result<ChainForkPolicy> {
    match value {
        "reject-unexpected-forks" | "production" | "reject" => Ok(ChainForkPolicy::RejectUnexpectedForks),
        "retain-fork-evidence" | "diagnostic" | "retain" => Ok(ChainForkPolicy::RetainForkEvidence),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported chain fork policy {other}; expected reject-unexpected-forks or retain-fork-evidence"
        ))),
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
