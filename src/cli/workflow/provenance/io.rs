type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn write_optional_preserves(out: Option<&FilePath>, value: &preserves::IOValue) -> Outcome<bool> {
    if let Some(path) = out {
        write_file(path, &molten::preserves_rail::to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", molten::preserves_rail::to_text(value)?);
        Ok(false)
    }
}

pub(super) fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn write_file(path: &std::path::Path, contents: &str) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
