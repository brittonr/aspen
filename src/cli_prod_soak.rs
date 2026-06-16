use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::content_ref_from_bytes;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::prod_soak;
use preserves::IOValue;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum ProdSoakCommand {
    EvidenceExport {
        #[arg(long)]
        node: String,
        #[arg(long)]
        node_evidence: PathBuf,
        #[arg(long = "artifact")]
        artifacts: Vec<PathBuf>,
        #[arg(long = "log")]
        logs: Vec<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RunReceipt {
        #[arg(long)]
        topology: PathBuf,
        #[arg(long = "node-evidence")]
        node_evidence: Vec<PathBuf>,
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value = "none")]
        fault_profile: String,
        #[arg(long = "peer-ticket-ref")]
        peer_ticket_refs: Vec<String>,
        #[arg(long = "node-control-ref")]
        node_control_refs: Vec<String>,
        #[arg(long = "remote-service-ref")]
        remote_service_refs: Vec<String>,
        #[arg(long = "job-ref")]
        job_refs: Vec<String>,
        #[arg(long = "coordination-ref")]
        coordination_refs: Vec<String>,
        #[arg(long = "evidence-export")]
        evidence_exports: Vec<PathBuf>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "non-replayable-live-observations")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "log")]
        logs: Vec<PathBuf>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_prod_soak_command(command: ProdSoakCommand) -> Result<()> {
    match command {
        ProdSoakCommand::EvidenceExport {
            node,
            node_evidence,
            artifacts,
            logs,
            out,
        } => {
            let node_evidence_ref = preserves_file_ref(&node_evidence)?;
            let artifact_refs = preserves_file_refs(&artifacts)?;
            let log_refs = raw_file_refs(&logs)?;
            let value = prod_soak::evidence_export_value(&prod_soak::ProdSoakEvidenceExportInput {
                node: &node,
                node_evidence_ref: &node_evidence_ref,
                artifact_refs: &artifact_refs,
                log_refs: &log_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("prod-soak evidence-export ref={reference} node={node}"));
            Ok(())
        }
        ProdSoakCommand::RunReceipt {
            topology,
            node_evidence,
            scenario,
            fault_profile,
            peer_ticket_refs,
            node_control_refs,
            remote_service_refs,
            job_refs,
            coordination_refs,
            evidence_exports,
            decision,
            replay_status,
            diagnostics,
            logs,
            caveats,
            out,
        } => {
            let topology_ref = preserves_file_ref(&topology)?;
            let node_evidence_refs = preserves_file_refs(&node_evidence)?;
            let evidence_export_refs = preserves_file_refs(&evidence_exports)?;
            let log_refs = raw_file_refs(&logs)?;
            let value = prod_soak::run_value(&prod_soak::ProdSoakRunInput {
                decision: &decision,
                scenario: &scenario,
                topology_ref: &topology_ref,
                fault_profile: &fault_profile,
                node_evidence_refs: &node_evidence_refs,
                peer_ticket_refs: &peer_ticket_refs,
                node_control_refs: &node_control_refs,
                remote_service_refs: &remote_service_refs,
                job_refs: &job_refs,
                coordination_refs: &coordination_refs,
                evidence_export_refs: &evidence_export_refs,
                replay_status: &replay_status,
                diagnostics: &diagnostics,
                log_refs: &log_refs,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("prod-soak run ref={reference} decision={decision} scenario={scenario}"),
            );
            Ok(())
        }
        ProdSoakCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            let reference = canonical_hash(&value)?;
            let rendered = to_text(&value)?;
            let kind = prod_soak_kind(&rendered);
            println!("prod-soak {kind} ref={reference} path={}", artifact.display());
            Ok(())
        }
    }
}

fn preserves_file_refs(paths: &[PathBuf]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(paths.len());
    for path in paths {
        refs.push(preserves_file_ref(path)?);
    }
    Ok(refs)
}

fn preserves_file_ref(path: &Path) -> Result<String> {
    let value = read_preserves_file(path)?;
    canonical_hash(&value)
}

fn raw_file_refs(paths: &[PathBuf]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(paths.len());
    for path in paths {
        refs.push(raw_file_ref(path)?);
    }
    Ok(refs)
}

fn raw_file_ref(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(MoltenError::from)?;
    Ok(content_ref_from_bytes(&bytes))
}

fn read_preserves_file(path: &Path) -> Result<IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_optional_preserves(path: Option<&PathBuf>, value: &IOValue) -> Result<bool> {
    let text = to_text(value)?;
    if let Some(path) = path {
        write_file(path, &text)?;
        Ok(true)
    } else {
        println!("{text}");
        Ok(false)
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn prod_soak_kind(text: &str) -> &'static str {
    if text.contains("prod-soak-evidence-export-v1") {
        "evidence-export"
    } else if text.contains("prod-soak-run-v1") {
        "run"
    } else {
        "artifact"
    }
}
