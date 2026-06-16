use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::harness::failure_value;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::transcripts;

#[derive(Debug, Subcommand)]
pub(crate) enum TranscriptCommand {
    Parse {
        markdown: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long = "dependency")]
        dependency_refs: Vec<String>,
        #[arg(long = "dependency-closure-hash")]
        dependency_closure_hash: Option<String>,
        #[arg(long = "handler-profile-ref")]
        handler_profile_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "revocation-ref")]
        revocation_refs: Vec<String>,
        #[arg(long = "seed-ref")]
        seed_ref: Option<String>,
        #[arg(long = "expected-ref")]
        expected_refs: Vec<String>,
    },
    Run {
        transcript: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long, default_value = "fresh")]
        state: String,
        #[arg(long)]
        save_root: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Show {
        transcript: PathBuf,
    },
    Render {
        transcript: PathBuf,
        #[arg(long)]
        receipt: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
}

pub(crate) fn run_transcript_command(command: TranscriptCommand) -> Result<()> {
    match command {
        TranscriptCommand::Parse {
            markdown,
            out,
            dependency_refs,
            dependency_closure_hash,
            handler_profile_ref,
            policy_refs,
            capability_refs,
            revocation_refs,
            seed_ref,
            expected_refs,
        } => {
            let source = fs::read_to_string(&markdown).map_err(MoltenError::from)?;
            let transcript = transcripts::parse_markdown(&source, &transcripts::TranscriptParseInput {
                dependency_refs,
                dependency_closure_hash,
                handler_profile_ref,
                policy_refs,
                capability_refs,
                revocation_refs,
                seed_ref,
                expected_refs,
            })?;
            write_file(&out, &to_text(&transcript.value)?)?;
            println!(
                "transcript parse ok transcript={} stanzas={} out={}",
                transcript.transcript_ref,
                transcript.stanzas.len(),
                out.display()
            );
            Ok(())
        }
        TranscriptCommand::Run {
            transcript,
            cache,
            state,
            save_root,
            out,
            receipt_out,
            failure_out,
        } => {
            let artifact = match read_transcript_input(&transcript) {
                Ok(artifact) => artifact,
                Err(error) => {
                    write_optional_failure(failure_out.as_ref(), "parse", &error, None)?;
                    return Err(error);
                }
            };
            let mode = transcripts::TranscriptRunMode::parse(&state)?;
            let run = transcripts::run_transcript(&artifact, &transcripts::TranscriptRunInput {
                mode,
                cache_root: cache,
                save_root,
            })?;
            if let Some(path) = out.as_ref() {
                write_file(path, &transcripts::render_transcript(&artifact, Some(&run))?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "transcript run receipt", &run.receipt_value)?;
            eprintln!(
                "transcript run decision={} transcript={} receipt={}",
                run.decision, run.transcript_ref, run.receipt_ref
            );
            if run.decision == "deny" || run.decision == "error" {
                let error = MoltenError::invalid_harness(format!("transcript run decision {}", run.decision));
                write_optional_failure(failure_out.as_ref(), "run", &error, Some(vec![run.receipt_value]))?;
                return Err(error);
            }
            Ok(())
        }
        TranscriptCommand::Show { transcript } => {
            let artifact = read_transcript_input(&transcript)?;
            println!("{}", to_text(&artifact.value)?);
            Ok(())
        }
        TranscriptCommand::Render {
            transcript,
            receipt,
            out,
        } => {
            let artifact = read_transcript_input(&transcript)?;
            let run = receipt
                .as_ref()
                .map(|path| {
                    let receipt_value = read_preserves_file(path)?;
                    let receipt = transcripts::parse_transcript_run_receipt(&receipt_value)?;
                    Ok::<transcripts::TranscriptRun, MoltenError>(transcripts::TranscriptRun {
                        transcript_ref: receipt.transcript_ref,
                        decision: receipt.decision,
                        stanza_outcomes: Vec::new(),
                        receipt_ref: receipt.receipt_ref,
                        receipt_value,
                        cache_receipt_value: None,
                        state_root: None,
                    })
                })
                .transpose()?;
            write_file(&out, &transcripts::render_transcript(&artifact, run.as_ref())?)?;
            println!("transcript render ok transcript={} out={}", artifact.transcript_ref, out.display());
            Ok(())
        }
    }
}

fn read_transcript_input(path: &Path) -> Result<transcripts::TranscriptArtifact> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    if let Ok(value) = parse_text(&text)
        && let Ok(transcript) = transcripts::parse_transcript_artifact(&value)
    {
        return Ok(transcript);
    }
    transcripts::parse_markdown(&text, &transcripts::TranscriptParseInput {
        dependency_refs: Vec::new(),
        dependency_closure_hash: None,
        handler_profile_ref: None,
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        seed_ref: None,
        expected_refs: Vec::new(),
    })
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

fn write_optional_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn emit_failure(path: Option<&PathBuf>, failure: &preserves::IOValue) -> Result<()> {
    let failure_text = to_text(failure)?;
    let failure_ref = canonical_hash(failure)?;
    if let Some(path) = path {
        write_file(path, &failure_text)?;
        eprintln!("failure {failure_ref} written to {}", path.display());
    } else {
        println!("{failure_text}");
        eprintln!("failure {failure_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
