pub(super) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn write_optional_preserves(
    out: Option<&std::path::PathBuf>,
    value: &preserves::IOValue,
) -> molten::error::Result<bool> {
    let text = molten::preserves_rail::to_text(value)?;
    if let Some(path) = out {
        write_file(path, &text)?;
        Ok(true)
    } else {
        println!("{text}");
        Ok(false)
    }
}

pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}

pub(super) fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}
