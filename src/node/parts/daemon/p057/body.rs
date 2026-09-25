
fn validate_state_root(state_root: &Path) -> Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("node daemon requires explicit state root"));
    }
    if state_root == Path::new(".") {
        return Err(MoltenError::invalid_harness("node daemon state root cannot be ambient current directory"));
    }
    Ok(())
}

fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.trim().is_empty() {
        Err(MoltenError::invalid_harness("node daemon id must not be empty"))
    } else {
        Ok(())
    }
}

fn fixed_node_path(value: &str) -> Result<crate::node_state::NodeStatePath> {
    crate::node_state::NodeStatePath::parse(value)
}

fn write_preserves(
    root: &crate::node_state::NodeStateRoot,
    path: &crate::node_state::NodeStatePath,
    value: &IoValue,
) -> Result<()> {
    root.write(path, crate::preserves_rail::to_text(value)?.as_bytes())
}

fn read_preserves(
    root: &crate::node_state::NodeStateRoot,
    path: &crate::node_state::NodeStatePath,
) -> Result<IoValue> {
    let text = root.read_to_string(path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    crate::preserves_rail::parse_text(&text)
}

fn diagnostic_node_state_path(
    state_root: &Path,
    locator: &crate::node_state::NodeStatePath,
) -> PathBuf {
    state_root.join(locator.as_path())
}

pub fn config_path(state_root: &Path) -> PathBuf {
    state_root.join(CONFIG_FILE)
}

pub fn startup_path(state_root: &Path) -> PathBuf {
    state_root.join(STARTUP_FILE)
}

pub fn shutdown_path(state_root: &Path) -> PathBuf {
    state_root.join(SHUTDOWN_FILE)
}
