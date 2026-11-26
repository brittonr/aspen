//! Ambient host paths beneath a state root, used only for diagnostics and operator display.

pub(super) fn diagnostic_node_state_path(
    state_root: &std::path::Path,
    locator: &crate::node_state::RelativePath,
) -> std::path::PathBuf {
    state_root.join(locator.as_path())
}

pub fn config_path(state_root: &std::path::Path) -> std::path::PathBuf {
    state_root.join(super::settings::CONFIG_FILE)
}

pub fn startup_path(state_root: &std::path::Path) -> std::path::PathBuf {
    state_root.join(super::settings::STARTUP_FILE)
}

pub fn shutdown_path(state_root: &std::path::Path) -> std::path::PathBuf {
    state_root.join(super::settings::SHUTDOWN_FILE)
}
