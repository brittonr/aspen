use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::deterministic_replay;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

#[derive(Debug, Subcommand)]
pub(crate) enum ReplayFixtureCommand {
    Record {
        #[arg(long)]
        out: PathBuf,
    },
    Verify {
        fixture: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Tamper {
        fixture: PathBuf,
        #[arg(long, default_value = "effect-response")]
        kind: String,
        #[arg(long)]
        out: PathBuf,
    },
    Rollup {
        #[arg(long = "receipt")]
        receipts: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Index {
        #[arg(long = "receipt")]
        receipts: Vec<PathBuf>,
        #[arg(long = "rollup")]
        rollups: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
    },
}

pub(crate) fn run_replay_fixture_command(command: ReplayFixtureCommand) -> Result<()> {
    match command {
        ReplayFixtureCommand::Record { out } => {
            let fixture = deterministic_replay::record_fixture_value()?;
            write_file(&out, &to_text(&fixture.value)?)?;
            println!(
                "deterministic replay fixture written to {} ref={} identity={} final_state={}",
                out.display(),
                fixture.record_ref,
                fixture.identity_ref,
                fixture.final_state_ref
            );
            Ok(())
        }
        ReplayFixtureCommand::Verify { fixture, receipt_out } => {
            read_preserves_file(&fixture)?;
            let receipt =
                deterministic_replay::verify_fixture_value(deterministic_replay::ReplayFixtureVariant::Baseline)?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "deterministic replay verify ref={} decision={} divergence={}",
                    receipt.receipt_ref,
                    receipt.decision,
                    receipt.divergence.as_str()
                ),
            );
            Ok(())
        }
        ReplayFixtureCommand::Tamper { fixture, kind, out } => {
            read_preserves_file(&fixture)?;
            let variant = replay_fixture_variant_from_kind(&kind)?;
            let receipt = deterministic_replay::verify_fixture_value(variant)?;
            write_file(&out, &to_text(&receipt.value)?)?;
            println!(
                "deterministic replay tamper receipt written to {} ref={} divergence={}",
                out.display(),
                receipt.receipt_ref,
                receipt.divergence.as_str()
            );
            Ok(())
        }
        ReplayFixtureCommand::Rollup { receipts, out } => {
            let mut inputs = Vec::with_capacity(receipts.len());
            for receipt in receipts {
                let value = read_preserves_file(&receipt)?;
                inputs.push(deterministic_replay::ReplayRollupInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            let rollup = deterministic_replay::rollup_replay_receipts(&inputs)?;
            write_file(&out, &to_text(&rollup.value)?)?;
            println!(
                "deterministic replay rollup written to {} ref={} decision={} total={} pass={} deny={}",
                out.display(),
                rollup.rollup_ref,
                rollup.decision,
                rollup.total_count,
                rollup.pass_count,
                rollup.deny_count
            );
            Ok(())
        }
        ReplayFixtureCommand::Index { receipts, rollups, out } => {
            let mut inputs = Vec::with_capacity(receipts.len() + rollups.len());
            for receipt in receipts {
                let value = read_preserves_file(&receipt)?;
                inputs.push(deterministic_replay::ReplayIndexInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            for rollup in rollups {
                let value = read_preserves_file(&rollup)?;
                inputs.push(deterministic_replay::ReplayIndexInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            let index = deterministic_replay::index_replay_evidence(&inputs)?;
            write_file(&out, &to_text(&index.value)?)?;
            println!(
                "deterministic replay index written to {} ref={} decision={} total={} pass={} deny={} raw_receipts={} rollups={}",
                out.display(),
                index.index_ref,
                index.decision,
                index.total_count,
                index.pass_count,
                index.deny_count,
                index.raw_receipt_count,
                index.rollup_count
            );
            Ok(())
        }
        ReplayFixtureCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            let reference = canonical_hash(&value)?;
            println!("deterministic replay artifact ref={reference}");
            println!("{}", to_text(&value)?);
            Ok(())
        }
    }
}

fn replay_fixture_variant_from_kind(kind: &str) -> Result<deterministic_replay::ReplayFixtureVariant> {
    match kind {
        "identity" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedIdentity),
        "scheduler" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedScheduler),
        "input" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedInput),
        "effect-request" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedEffectRequest),
        "effect-response" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedEffectResponse),
        "policy" | "policy-decision" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedPolicyDecision),
        "action" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedAction),
        "receipt" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedReceipt),
        "output" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedOutput),
        "state" | "state-hash" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedStateHash),
        "live-effect" | "missing-effect" => Ok(deterministic_replay::ReplayFixtureVariant::MissingRecordedEffect),
        _ => Err(MoltenError::invalid_harness(format!("unsupported replay fixture tamper kind {kind}"))),
    }
}

fn write_optional_preserves(out: Option<&PathBuf>, value: &preserves::IOValue) -> Result<bool> {
    if let Some(path) = out {
        write_file(path, &to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", to_text(value)?);
        Ok(false)
    }
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
