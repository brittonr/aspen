pub(crate) fn read_preserves_file(path: &std::path::Path) -> molten_node_runtime::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten_node_runtime::error::Failure::from)?;
    molten_node_runtime::preserves_rail::parse_text(&text)
}

pub(crate) fn emit_named_receipt(
    path: Option<&std::path::PathBuf>,
    label: &str,
    receipt: &preserves::IOValue,
) -> molten_node_runtime::error::Result<()> {
    let receipt_text = molten_node_runtime::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten_node_runtime::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

pub(crate) fn write_file(path: &std::path::Path, contents: &str) -> molten_node_runtime::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten_node_runtime::error::Failure::from)?;
    }
    std::fs::write(path, contents).map_err(molten_node_runtime::error::Failure::from)
}
