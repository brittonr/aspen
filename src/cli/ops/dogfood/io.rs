use std::io::Read;
use std::io::Write;

pub(super) fn read_preserves_file(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let mut file = molten::materialization::open_explicit_input_file(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn write_file(path: &std::path::Path, contents: &str) -> molten::error::Result<()> {
    let mut file = molten::materialization::create_explicit_output_file(path)?;
    file.write_all(contents.as_bytes()).map_err(molten::error::MoltenError::from)?;
    file.flush().map_err(molten::error::MoltenError::from)
}
