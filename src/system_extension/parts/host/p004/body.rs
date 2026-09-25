
fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("system-extension {label} denied: {issues:?}"))
}
