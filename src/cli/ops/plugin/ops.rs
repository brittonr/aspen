pub(super) fn install(
    manifest: std::path::PathBuf,
    registry: std::path::PathBuf,
    out: std::path::PathBuf,
) -> molten::error::Result<()> {
    let manifest_value = super::io::read_preserves_file(&manifest)?;
    let receipt = molten::plugin_host::install_plugin(&registry, &manifest_value)?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "plugin install decision={} receipt={} manifest={} out={}",
        receipt.decision,
        receipt.receipt_ref,
        receipt.manifest_ref,
        out.display()
    );
    Ok(())
}

pub(super) fn run_fixture(state_root: std::path::PathBuf, out: std::path::PathBuf) -> molten::error::Result<()> {
    let run = molten::plugin_host::minimal_plugin_fixture(&state_root)?;
    std::fs::create_dir_all(&out).map_err(molten::error::MoltenError::from)?;
    super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&run.report_value)?)?;
    super::io::write_indexed_values(&out, "evidence", &run.evidence_values)?;
    println!(
        "plugin fixture decision={} manifest={} install={} health={} removal={} out={}",
        run.decision,
        run.manifest_ref,
        run.install_receipt_ref,
        run.health_receipt_ref,
        run.removal_receipt_ref,
        out.display()
    );
    Ok(())
}

pub(super) fn show(artifact: std::path::PathBuf) -> molten::error::Result<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::plugin_host::plugin_summary(&value)?);
    Ok(())
}
