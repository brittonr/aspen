pub(super) fn run_fixture(out: std::path::PathBuf) -> molten::error::Result<()> {
    let run = molten::secrets::run_secrets_fixture()?;
    std::fs::create_dir_all(&out).map_err(molten::error::MoltenError::from)?;
    super::io::write_file(&out.join("report.preserves"), &molten::preserves_rail::to_text(&run.value)?)?;
    super::io::write_file(&out.join("secret.preserves"), &molten::preserves_rail::to_text(&run.secret.value)?)?;
    super::io::write_file(
        &out.join("encrypted-ref.preserves"),
        &molten::preserves_rail::to_text(&run.encrypted.value)?,
    )?;
    super::io::write_file(
        &out.join("redaction-marker.preserves"),
        &molten::preserves_rail::to_text(&run.marker.value)?,
    )?;
    super::io::write_file(
        &out.join("redaction-transform.preserves"),
        &molten::preserves_rail::to_text(&run.transform.value)?,
    )?;
    super::io::write_file(
        &out.join("reveal-denied.preserves"),
        &molten::preserves_rail::to_text(&run.reveal_denied.value)?,
    )?;
    super::io::write_file(
        &out.join("reveal-pass.preserves"),
        &molten::preserves_rail::to_text(&run.reveal_pass.value)?,
    )?;
    super::io::write_file(
        &out.join("decrypt-denied.preserves"),
        &molten::preserves_rail::to_text(&run.decrypt_denied.value)?,
    )?;
    super::io::write_file(
        &out.join("decrypt-pass.preserves"),
        &molten::preserves_rail::to_text(&run.decrypt_pass.value)?,
    )?;
    super::io::write_file(
        &out.join("commitment-replay.preserves"),
        &molten::preserves_rail::to_text(&run.replay.value)?,
    )?;
    super::io::write_file(&out.join("cleanup.preserves"), &molten::preserves_rail::to_text(&run.cleanup.value)?)?;
    super::io::write_file(
        &out.join("private-bundle-profile.preserves"),
        &molten::preserves_rail::to_text(&run.private_bundle.value)?,
    )?;
    super::io::write_indexed_values(&out, "evidence", &run.evidence_values)?;
    super::io::write_file(&out.join("summary.txt"), &molten::secrets::fixture_report_summary(&run.value)?)?;
    println!("secrets fixture ok report={} out={}", run.report_ref, out.display());
    Ok(())
}

pub(super) fn show(artifact: std::path::PathBuf) -> molten::error::Result<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    match molten::secrets::fixture_report_summary(&value) {
        Ok(summary) => println!("{summary}"),
        Err(_) => println!("{}", molten::secrets::secrets_summary(&value)?),
    }
    Ok(())
}
