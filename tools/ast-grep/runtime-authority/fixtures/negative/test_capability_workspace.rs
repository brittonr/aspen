fn isolated_workspace() -> std::io::Result<cap_tempfile::TempDir> {
    cap_tempfile::tempdir(cap_tempfile::ambient_authority())
}

fn explicit_selected_export(
    source: &cap_std::fs::Dir,
    destination: &cap_std::fs::Dir,
) -> std::io::Result<()> {
    let bytes = source.read("receipts/run.preserves")?;
    destination.create_dir_all("selected")?;
    destination.write("selected/run.preserves", bytes)
}
