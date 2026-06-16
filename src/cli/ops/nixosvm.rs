use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::nixos_vm;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::content_ref_from_bytes;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use preserves::IOValue;

#[derive(Debug, Subcommand)]
pub(crate) enum NixosVmCommand {
    Topology {
        #[arg(long = "node")]
        nodes: Vec<String>,
        #[arg(long)]
        package_ref: String,
        #[arg(long)]
        package_path: String,
        #[arg(long, default_value = "nixos-test-private")]
        network: String,
        #[arg(long = "nix-input")]
        nix_inputs: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    NodeEvidence {
        #[arg(long)]
        node: String,
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        identity: Option<PathBuf>,
        #[arg(long)]
        startup: PathBuf,
        #[arg(long)]
        health: PathBuf,
        #[arg(long)]
        control_loop: PathBuf,
        #[arg(long)]
        heartbeat: PathBuf,
        #[arg(long)]
        shutdown: Option<PathBuf>,
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
        #[arg(long = "child-ref")]
        child_refs: Vec<String>,
        #[arg(long = "log")]
        logs: Vec<PathBuf>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "non-replayable-vm-observations")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_nixos_vm_command(command: NixosVmCommand) -> Result<()> {
    match command {
        NixosVmCommand::Topology {
            nodes,
            package_ref,
            package_path,
            network,
            nix_inputs,
            caveats,
            out,
        } => {
            let value = nixos_vm::topology_value(&nixos_vm::NixosVmTopologyInput {
                nodes: &nodes,
                package_ref: &package_ref,
                package_path: &package_path,
                network: &network,
                nix_inputs: &nix_inputs,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("nixos-vm topology ref={reference} nodes={} network={network}", nodes.len()),
            );
            Ok(())
        }
        NixosVmCommand::NodeEvidence {
            node,
            state_root,
            identity,
            startup,
            health,
            control_loop,
            heartbeat,
            shutdown,
            logs,
            out,
        } => {
            let identity_ref = optional_preserves_ref(identity.as_ref())?;
            let startup_ref = preserves_file_ref(&startup)?;
            let health_ref = preserves_file_ref(&health)?;
            let control_loop_ref = preserves_file_ref(&control_loop)?;
            let heartbeat_ref = preserves_file_ref(&heartbeat)?;
            let shutdown_ref = optional_preserves_ref(shutdown.as_ref())?;
            let log_refs = raw_file_refs(&logs)?;
            let state_root_text = state_root.display().to_string();
            let value = nixos_vm::node_evidence_value(&nixos_vm::NixosVmNodeEvidenceInput {
                node: &node,
                state_root: &state_root_text,
                identity_receipt_ref: identity_ref.as_deref(),
                startup_receipt_ref: &startup_ref,
                health_receipt_ref: &health_ref,
                control_loop_receipt_ref: &control_loop_ref,
                heartbeat_receipt_ref: &heartbeat_ref,
                shutdown_receipt_ref: shutdown_ref.as_deref(),
                log_refs: &log_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("nixos-vm node-evidence ref={reference} node={node}"));
            Ok(())
        }
        NixosVmCommand::RunReceipt {
            topology,
            node_evidence,
            scenario,
            fault_profile,
            child_refs,
            logs,
            decision,
            replay_status,
            diagnostics,
            caveats,
            out,
        } => {
            let topology_ref = preserves_file_ref(&topology)?;
            let node_evidence_refs = preserves_file_refs(&node_evidence)?;
            let log_refs = raw_file_refs(&logs)?;
            let value = nixos_vm::test_run_value(&nixos_vm::NixosVmTestRunInput {
                decision: &decision,
                topology_ref: &topology_ref,
                scenario: &scenario,
                fault_profile: &fault_profile,
                node_evidence_refs: &node_evidence_refs,
                child_workflow_refs: &child_refs,
                replay_status: &replay_status,
                diagnostics: &diagnostics,
                log_refs: &log_refs,
                caveats: &caveats,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("nixos-vm test-run ref={reference} decision={decision} scenario={scenario}"),
            );
            Ok(())
        }
        NixosVmCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            let reference = canonical_hash(&value)?;
            let rendered = to_text(&value)?;
            let kind = nixos_vm_kind(&rendered);
            println!("nixos-vm {kind} ref={reference} path={}", artifact.display());
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

fn optional_preserves_ref(path: Option<&PathBuf>) -> Result<Option<String>> {
    match path {
        Some(value) => preserves_file_ref(value).map(Some),
        None => Ok(None),
    }
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

fn nixos_vm_kind(text: &str) -> &'static str {
    if text.contains("nixos-vm-topology-v1") {
        "topology"
    } else if text.contains("nixos-vm-node-evidence-v1") {
        "node-evidence"
    } else if text.contains("nixos-vm-test-run-v1") {
        "test-run"
    } else {
        "artifact"
    }
}
