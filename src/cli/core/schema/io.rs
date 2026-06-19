pub(super) fn cli_schema_ref(kind: &str, label: &str) -> molten::error::Result<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("schema-cli-ref", vec![
        molten::preserves_rail::string(kind),
        molten::preserves_rail::string(label),
    ]))
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn emit_named_receipt(
    path: Option<&std::path::PathBuf>,
    label: &str,
    receipt: &preserves::IOValue,
) -> molten::error::Result<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
