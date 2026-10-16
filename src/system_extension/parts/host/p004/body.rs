
fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("system-extension {label} denied: {issues:?}"))
}
