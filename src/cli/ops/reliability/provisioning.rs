/// Explicit experimental operator provisioning. Keys are JSON arrays of exactly 32 secret bytes.
pub fn run(state_root: std::path::PathBuf, operator_key: std::path::PathBuf, policy: std::path::PathBuf,
    grant: std::path::PathBuf, probe_key: std::path::PathBuf) -> molten::error::Result<()> {
    let root = molten::node_state::DirectoryRoot::open(&state_root)?;
    root.create_layout()?;
    molten::live_reliability::authority::provision(&root, molten::live_reliability::authority::ProvisionInput {
        operator_secret: super::read(&operator_key)?, policy: super::read(&policy)?,
        grant: super::read(&grant)?, probe_secret: super::read(&probe_key)?,
    })
}
