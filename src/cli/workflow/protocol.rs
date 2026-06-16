use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::protocol_session;

const PROTOCOL_LIFECYCLE_INDEX_LIMIT: usize = 256;
const _: () = assert!(PROTOCOL_LIFECYCLE_INDEX_LIMIT > 0);

#[derive(Debug, Subcommand)]
pub(crate) enum ProtocolCommand {
    Install {
        manifest: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunRequestResponse {
        #[arg(long)]
        out: PathBuf,
    },
    GateLifecycle {
        dir: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        receipt: PathBuf,
    },
}

pub(crate) fn run_protocol_command(command: ProtocolCommand) -> Result<()> {
    match command {
        ProtocolCommand::Install { manifest, out } => {
            let manifest_value = read_preserves_file(&manifest)?;
            let install = protocol_session::install_protocol_manifest_value(&manifest_value)?;
            write_protocol_install(&out, &manifest_value, &install)?;
            println!(
                "protocol install decision={} receipt={} protocol={} endpoints={} out={}",
                install.decision,
                install.receipt_ref,
                install.manifest.protocol_id,
                install.endpoints.len(),
                out.display()
            );
            Ok(())
        }
        ProtocolCommand::RunRequestResponse { out } => {
            let lifecycle = protocol_session::request_response_lifecycle()?;
            write_protocol_lifecycle(&out, &lifecycle)?;
            println!(
                "protocol request-response receipt={} operations={} out={}",
                lifecycle.install.receipt_ref,
                lifecycle.operations.len(),
                out.display()
            );
            Ok(())
        }
        ProtocolCommand::GateLifecycle { dir, receipt_out } => {
            let gate = protocol_session::gate_protocol_session_lifecycle(read_protocol_lifecycle_gate_input(&dir)?)?;
            emit_named_receipt(receipt_out.as_ref(), "protocol session gate receipt", &gate.value)?;
            println!(
                "protocol session gate {} install={} protocol={} sessions={} operations={} diagnostics={}",
                gate.decision,
                gate.install_ref,
                gate.protocol_ref,
                gate.session_ids.len(),
                gate.operation_count,
                gate.diagnostics.len()
            );
            Ok(())
        }
        ProtocolCommand::Show { receipt } => {
            let value = read_preserves_file(&receipt)?;
            println!("{}", protocol_session::protocol_summary(&value)?);
            Ok(())
        }
    }
}

fn write_protocol_install(
    out: &Path,
    manifest_value: &preserves::IOValue,
    install: &protocol_session::ProtocolInstallReceipt,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("manifest.preserves"), &to_text(manifest_value)?)?;
    write_file(&out.join("install-receipt.preserves"), &to_text(&install.value)?)?;
    write_file(&out.join("summary.txt"), &protocol_session::protocol_summary(&install.value)?)?;
    let endpoints_dir = out.join("endpoints");
    fs::create_dir_all(&endpoints_dir).map_err(MoltenError::from)?;
    write_indexed_values(
        &endpoints_dir,
        "endpoint",
        &install.endpoints.iter().map(|endpoint| endpoint.value.clone()).collect::<Vec<_>>(),
    )
}

fn write_protocol_lifecycle(out: &Path, lifecycle: &protocol_session::RequestResponseLifecycle) -> Result<()> {
    write_protocol_install(out, &lifecycle.manifest_value, &lifecycle.install)?;
    write_indexed_values(
        out,
        "initial-state",
        &lifecycle.initial_states.iter().map(|state| state.value.clone()).collect::<Vec<_>>(),
    )?;
    let mut messages = Vec::with_capacity(lifecycle.operations.len());
    let mut receipts = Vec::with_capacity(lifecycle.operations.len());
    let mut next_states = Vec::with_capacity(lifecycle.operations.len());
    for operation in &lifecycle.operations {
        if let Some(message) = &operation.message {
            messages.push(message.value.clone());
        }
        receipts.push(operation.receipt.value.clone());
        if let Some(state) = &operation.next_state {
            next_states.push(state.value.clone());
        }
    }
    write_indexed_values(out, "message", &messages)?;
    write_indexed_values(out, "operation", &receipts)?;
    write_indexed_values(out, "next-state", &next_states)
}

fn read_protocol_lifecycle_gate_input(dir: &Path) -> Result<protocol_session::ProtocolSessionGateInput> {
    Ok(protocol_session::ProtocolSessionGateInput {
        install_receipt: read_preserves_file(&dir.join("install-receipt.preserves"))?,
        initial_states: read_indexed_values(dir, "initial-state")?,
        operation_receipts: read_indexed_values(dir, "operation")?,
        messages: read_indexed_values(dir, "message")?,
        next_states: read_indexed_values(dir, "next-state")?,
    })
}

fn read_indexed_values(dir: &Path, prefix: &str) -> Result<Vec<preserves::IOValue>> {
    let mut values = Vec::with_capacity(PROTOCOL_LIFECYCLE_INDEX_LIMIT.min(16));
    for index in 0..PROTOCOL_LIFECYCLE_INDEX_LIMIT {
        let path = dir.join(format!("{prefix}-{index}.preserves"));
        if !path.exists() {
            return Ok(values);
        }
        values.push(read_preserves_file(&path)?);
    }
    let overflow_path = dir.join(format!("{prefix}-{PROTOCOL_LIFECYCLE_INDEX_LIMIT}.preserves"));
    if overflow_path.exists() {
        return Err(MoltenError::invalid_harness(format!("protocol lifecycle {prefix} evidence exceeds index limit")));
    }
    Ok(values)
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} written to {}", path.display());
    } else {
        println!("{receipt_text}");
    }
    Ok(())
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
