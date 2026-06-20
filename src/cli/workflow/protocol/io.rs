const LIFECYCLE_INDEX_LIMIT: usize = 256;
const _: () = assert!(LIFECYCLE_INDEX_LIMIT > 0);

type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn write_install(
    out: &std::path::Path,
    manifest_value: &preserves::IOValue,
    install: &molten::protocol_session::ProtocolInstallReceipt,
) -> Outcome<()> {
    std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
    write_file(out.join("manifest.preserves"), &molten::preserves_rail::to_text(manifest_value)?)?;
    write_file(out.join("install-receipt.preserves"), &molten::preserves_rail::to_text(&install.value)?)?;
    write_file(out.join("summary.txt"), &molten::protocol_session::protocol_summary(&install.value)?)?;
    let endpoints_dir = out.join("endpoints");
    std::fs::create_dir_all(&endpoints_dir).map_err(molten::error::MoltenError::from)?;
    write_indexed_values(
        &endpoints_dir,
        "endpoint",
        &install.endpoints.iter().map(|endpoint| endpoint.value.clone()).collect::<Vec<_>>(),
    )
}

pub(super) fn write_lifecycle(
    out: &std::path::Path,
    lifecycle: &molten::protocol_session::RequestResponseLifecycle,
) -> Outcome<()> {
    write_install(out, &lifecycle.manifest_value, &lifecycle.install)?;
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

pub(super) fn read_lifecycle_gate_input(
    dir: &std::path::Path,
) -> Outcome<molten::protocol_session::ProtocolSessionGateInput> {
    Ok(molten::protocol_session::ProtocolSessionGateInput {
        install_receipt: read_preserves_file(&dir.join("install-receipt.preserves"))?,
        initial_states: read_indexed_values(dir, "initial-state")?,
        operation_receipts: read_indexed_values(dir, "operation")?,
        messages: read_indexed_values(dir, "message")?,
        next_states: read_indexed_values(dir, "next-state")?,
    })
}

pub(super) fn emit_named_receipt(path: Option<&FilePath>, label: &str, receipt: &preserves::IOValue) -> Outcome<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} written to {}", path.display());
    } else {
        println!("{receipt_text}");
    }
    Ok(())
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

fn read_indexed_values(dir: &std::path::Path, prefix: &str) -> Outcome<Vec<preserves::IOValue>> {
    let mut values = Vec::with_capacity(LIFECYCLE_INDEX_LIMIT.min(16));
    for index in 0..LIFECYCLE_INDEX_LIMIT {
        let path = dir.join(format!("{prefix}-{index}.preserves"));
        if !path.exists() {
            return Ok(values);
        }
        values.push(read_preserves_file(&path)?);
    }
    let overflow_path = dir.join(format!("{prefix}-{LIFECYCLE_INDEX_LIMIT}.preserves"));
    if overflow_path.exists() {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "protocol lifecycle {prefix} evidence exceeds index limit"
        )));
    }
    Ok(values)
}

fn write_indexed_values(out: &std::path::Path, prefix: &str, values: &[preserves::IOValue]) -> Outcome<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(out.join(format!("{prefix}-{index}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

fn write_file(path: impl AsRef<std::path::Path>, contents: &str) -> Outcome<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
