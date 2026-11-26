//! Read-only node lifecycle state inspection.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeLifecycleFiles {
    pub has_config: bool,
    pub has_identity_receipt: bool,
    pub has_startup: bool,
    pub has_shutdown: bool,
    pub has_active_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLifecycleState {
    Empty,
    Initialized,
    Running,
    Stopped,
    Inconsistent,
}

pub fn inspect_node_lifecycle_state(state_root: &std::path::Path) -> NodeLifecycleState {
    let Ok(root) = crate::node_state::DirectoryRoot::open_existing(state_root) else {
        return NodeLifecycleState::Empty;
    };
    inspect_node_lifecycle_state_with_root(&root).unwrap_or(NodeLifecycleState::Inconsistent)
}

pub fn inspect_node_lifecycle_state_with_root(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<NodeLifecycleState> {
    Ok(node_lifecycle_state(&node_lifecycle_files_with_root(root)?))
}

pub fn node_lifecycle_files(state_root: &std::path::Path) -> NodeLifecycleFiles {
    let Ok(root) = crate::node_state::DirectoryRoot::open_existing(state_root) else {
        return NodeLifecycleFiles {
            has_config: false,
            has_identity_receipt: false,
            has_startup: false,
            has_shutdown: false,
            has_active_lock: false,
        };
    };
    node_lifecycle_files_with_root(&root).unwrap_or(NodeLifecycleFiles {
        has_config: true,
        has_identity_receipt: false,
        has_startup: false,
        has_shutdown: false,
        has_active_lock: true,
    })
}

pub fn node_lifecycle_files_with_root(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<NodeLifecycleFiles> {
    Ok(NodeLifecycleFiles {
        has_config: root.try_exists(&super::storage::fixed_node_path(super::settings::CONFIG_FILE)?)?,
        has_identity_receipt: root
            .try_exists(&super::storage::fixed_node_path(super::settings::IDENTITY_RECEIPT_FILE)?)?,
        has_startup: root.try_exists(&super::storage::fixed_node_path(super::settings::STARTUP_FILE)?)?,
        has_shutdown: root.try_exists(&super::storage::fixed_node_path(super::settings::SHUTDOWN_FILE)?)?,
        has_active_lock: root.try_exists(&super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?)?,
    })
}

pub fn node_lifecycle_state(files: &NodeLifecycleFiles) -> NodeLifecycleState {
    if !files.has_config
        && !files.has_identity_receipt
        && !files.has_startup
        && !files.has_shutdown
        && !files.has_active_lock
    {
        return NodeLifecycleState::Empty;
    }
    if files.has_config
        && files.has_identity_receipt
        && !files.has_startup
        && !files.has_shutdown
        && !files.has_active_lock
    {
        return NodeLifecycleState::Initialized;
    }
    if files.has_config
        && files.has_identity_receipt
        && files.has_startup
        && !files.has_shutdown
        && files.has_active_lock
    {
        return NodeLifecycleState::Running;
    }
    if files.has_config
        && files.has_identity_receipt
        && files.has_startup
        && files.has_shutdown
        && !files.has_active_lock
    {
        return NodeLifecycleState::Stopped;
    }
    NodeLifecycleState::Inconsistent
}

pub(super) fn verify_restart_state(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<()> {
    let startup_path = super::storage::fixed_node_path(super::settings::STARTUP_FILE)?;
    if root.try_exists(&startup_path)? {
        let shutdown_path = super::storage::fixed_node_path(super::settings::SHUTDOWN_FILE)?;
        if !root.try_exists(&shutdown_path)? {
            return Err(crate::error::Failure::invalid_harness(
                "node daemon restart denied: previous startup has no clean shutdown receipt",
            ));
        }
        let startup_value = super::storage::read_preserves(root, &startup_path)?;
        let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
        if !super::tracking::completed_node_run_is_stopped(root, &startup.receipt_ref)? {
            return Err(crate::error::Failure::invalid_harness(
                "node restart has unresolved current-run shutdown observations",
            ));
        }
        let shutdown_ref =
            crate::preserves_rail::canonical_hash(&super::storage::read_preserves(root, &shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &super::storage::index_receipt_refs(root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            },
        )?;
        let health = crate::node_runtime::parse_node_health_receipt(&health_value)?;
        super::storage::write_preserves(
            root,
            &super::storage::fixed_node_path(super::settings::HEALTH_FILE)?,
            &health_value,
        )?;
        if health.decision != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!(
                "node daemon restart recovery denied receipt={}",
                health.receipt_ref
            )));
        }
        root.remove_regular_file(&shutdown_path)?;
    }
    Ok(())
}
