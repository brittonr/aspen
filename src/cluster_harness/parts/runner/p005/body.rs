
fn collect_run_files(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let mut files = Vec::new();
    collect_run_files_from(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_run_files_from(
    root: &std::path::Path,
    current: &std::path::Path,
    files: &mut impl crate::bounded::VecSink<String>,
) -> crate::error::Result<()> {
    let mut pending = vec![current.to_path_buf()];
    while let Some(next) = pending.pop() {
        if next.is_dir() {
            for entry in std::fs::read_dir(&next).map_err(crate::error::MoltenError::from)? {
                let entry = entry.map_err(crate::error::MoltenError::from)?;
                let file_type = entry.file_type().map_err(crate::error::MoltenError::from)?;
                if file_type.is_dir() {
                    pending.push(entry.path());
                } else if file_type.is_file() || file_type.is_symlink() {
                    let relative = entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| crate::error::MoltenError::invalid_harness("cluster run file escaped root"))?
                        .to_string_lossy()
                        .replace(std::path::MAIN_SEPARATOR, "/");
                    files.push_item(relative);
                }
            }
        }
    }
    Ok(())
}

fn allowed_unindexed_file(relative_path: &str) -> bool {
    matches!(
        relative_path,
        RUN_INDEX_FILE | VERIFICATION_FILE | FAILURE_BUNDLE_FILE | FAILURE_BUNDLE_VERIFICATION_FILE
    )
}

fn add_verification_companion_diagnostic(
    assessment: &mut molten_core::cluster_harness::RunDirectoryAssessment,
    diagnostic: &str,
    observed: &str,
) {
    assessment.decision = molten_core::cluster_harness::RUN_DIRECTORY_DENY.to_string();
    assessment.diagnostics.push(diagnostic.to_string());
    assessment.diagnostics.sort();
    assessment.diagnostics.dedup();
    assessment.first_divergence = Some(molten_core::cluster_harness::FirstDivergence {
        relative_path: VERIFICATION_FILE.to_string(),
        artifact_kind: VERIFICATION_KIND.to_string(),
        expected: "matching-verification-receipt".to_string(),
        observed: observed.to_string(),
        reason: diagnostic.to_string(),
        diagnostic_only: true,
    });
}

fn write_preserves_path(path: &std::path::Path, value: &IoValue) -> crate::error::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::write(path, crate::preserves_rail::to_text(value)?).map_err(crate::error::MoltenError::from)
}

fn read_preserves_path(path: &std::path::Path) -> crate::error::Result<IoValue> {
    if !is_regular_file_without_symlink(path) {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "expected regular non-symlink Preserves file at {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(crate::error::MoltenError::from)?;
    crate::preserves_rail::parse_text(&text)
}

fn is_regular_file_without_symlink(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
}
