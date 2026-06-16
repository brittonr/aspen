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
    Durability {
        #[arg(long)]
        scenario: String,
        #[arg(long = "queued-control-ref")]
        queued_control_refs: Vec<String>,
        #[arg(long = "recovery-ref")]
        recovery_refs: Vec<String>,
        #[arg(long = "ledger-ref")]
        ledger_refs: Vec<String>,
        #[arg(long = "chunk-ref")]
        chunk_refs: Vec<String>,
        #[arg(long = "retention-ref")]
        retention_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    FaultCase {
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        fault_kind: String,
        #[arg(long, default_value = "simulated")]
        injection: String,
        #[arg(long, default_value = "deny-before-side-effects")]
        expected_outcome: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "denial-ref")]
        denial_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "simulated-fault")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    ResourceEnvelope {
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        queue_depth: u64,
        #[arg(long)]
        max_queue_depth: u64,
        #[arg(long)]
        receipt_bytes: u64,
        #[arg(long)]
        max_receipt_bytes: u64,
        #[arg(long)]
        store_bytes: u64,
        #[arg(long)]
        max_store_bytes: u64,
        #[arg(long)]
        delivery_latency_ms: u64,
        #[arg(long)]
        max_delivery_latency_ms: u64,
        #[arg(long)]
        recovery_time_ms: u64,
        #[arg(long)]
        max_recovery_time_ms: u64,
        #[arg(long = "pressure-ref")]
        pressure_refs: Vec<String>,
        #[arg(long = "denial-ref")]
        denial_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    FaultMatrix {
        #[arg(long)]
        scenario: String,
        #[arg(long = "fault-case")]
        fault_cases: Vec<PathBuf>,
        #[arg(long = "fault-kind")]
        fault_kinds: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
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
        #[arg(long = "fault-ref")]
        fault_refs: Vec<String>,
        #[arg(long = "durability-ref")]
        durability_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
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
        ProdSoakCommand::Durability {
            scenario,
            queued_control_refs,
            recovery_refs,
            ledger_refs,
            chunk_refs,
            retention_refs,
            decision,
            diagnostics,
            caveats,
            out,
        } => {
            let value = prod_soak::durability_value(&prod_soak::ProdSoakDurabilityInput {
                decision: &decision,
                scenario: &scenario,
                queued_control_refs: &queued_control_refs,
                recovery_refs: &recovery_refs,
                ledger_refs: &ledger_refs,
                chunk_refs: &chunk_refs,
                retention_refs: &retention_refs,
                diagnostics: &diagnostics,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("prod-soak durability ref={reference} decision={decision} scenario={scenario}"),
            );
            Ok(())
        }
        ProdSoakCommand::FaultCase {
            scenario,
            fault_kind,
            injection,
            expected_outcome,
            evidence_refs,
            denial_refs,
            decision,
            replay_status,
            diagnostics,
            caveats,
            out,
        } => {
            let value = prod_soak::fault_case_value(&prod_soak::ProdSoakFaultCaseInput {
                decision: &decision,
                scenario: &scenario,
                fault_kind: &fault_kind,
                injection: &injection,
                expected_outcome: &expected_outcome,
                evidence_refs: &evidence_refs,
                denial_refs: &denial_refs,
                replay_status: &replay_status,
                diagnostics: &diagnostics,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("prod-soak fault-case ref={reference} decision={decision} fault={fault_kind}"),
            );
            Ok(())
        }
        ProdSoakCommand::ResourceEnvelope {
            scenario,
            queue_depth,
            max_queue_depth,
            receipt_bytes,
            max_receipt_bytes,
            store_bytes,
            max_store_bytes,
            delivery_latency_ms,
            max_delivery_latency_ms,
            recovery_time_ms,
            max_recovery_time_ms,
            pressure_refs,
            denial_refs,
            decision,
            diagnostics,
            caveats,
            out,
        } => {
            let value = prod_soak::resource_envelope_value(&prod_soak::ProdSoakResourceEnvelopeInput {
                decision: &decision,
                scenario: &scenario,
                queue_depth,
                max_queue_depth,
                receipt_bytes,
                max_receipt_bytes,
                store_bytes,
                max_store_bytes,
                delivery_latency_ms,
                max_delivery_latency_ms,
                recovery_time_ms,
                max_recovery_time_ms,
                pressure_refs: &pressure_refs,
                denial_refs: &denial_refs,
                diagnostics: &diagnostics,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "prod-soak resource-envelope ref={reference} decision={decision} queue={queue_depth}/{max_queue_depth}"
                ),
            );
            Ok(())
        }
        ProdSoakCommand::FaultMatrix {
            scenario,
            fault_cases,
            fault_kinds,
            decision,
            diagnostics,
            caveats,
            out,
        } => {
            let fault_case_refs = preserves_file_refs(&fault_cases)?;
            let value = prod_soak::fault_matrix_value(&prod_soak::ProdSoakFaultMatrixInput {
                decision: &decision,
                scenario: &scenario,
                fault_case_refs: &fault_case_refs,
                fault_kinds: &fault_kinds,
                diagnostics: &diagnostics,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("prod-soak fault-matrix ref={reference} decision={decision} faults={}", fault_kinds.len()),
            );
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
            fault_refs,
            durability_refs,
            resource_refs,
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
                fault_refs: &fault_refs,
                durability_refs: &durability_refs,
                resource_refs: &resource_refs,
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
    } else if text.contains("prod-soak-durability-v1") {
        "durability"
    } else if text.contains("prod-soak-fault-case-v1") {
        "fault-case"
    } else if text.contains("prod-soak-resource-envelope-v1") {
        "resource-envelope"
    } else if text.contains("prod-soak-fault-matrix-v1") {
        "fault-matrix"
    } else if text.contains("prod-soak-run-v1") {
        "run"
    } else {
        "artifact"
    }
}
