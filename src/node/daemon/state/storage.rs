//! State-root authority, layout, receipt refs, and Preserves file I/O.

pub(super) trait NodeStateAuthority {
    fn acquire_node_state_root(&self) -> crate::error::Result<crate::node_state::DirectoryRoot>;
}

impl NodeStateAuthority for crate::node_state::DirectoryRoot {
    fn acquire_node_state_root(&self) -> crate::error::Result<crate::node_state::DirectoryRoot> {
        Ok(self.clone())
    }
}

pub(super) fn read_ledger_artifact<Root: NodeStateAuthority + ?Sized>(
    source: &Root,
    artifact_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    let root = source.acquire_node_state_root()?;
    let ledger = root.ledger_store()?;
    crate::ledger::read_artifact_with_root(&ledger, artifact_ref)
}

pub(super) fn import_artifact<Root: NodeStateAuthority + ?Sized>(
    source: &Root,
    value: &preserves::IOValue,
) -> crate::error::Result<String> {
    let root = source.acquire_node_state_root()?;
    let ledger = root.ledger_store()?;
    let imported = crate::ledger::import_artifact_with_root(&ledger, value)?;
    let receipt_path = super::paths::node_leaf_path(
        "receipts",
        &format!("ledger-import-{}.preserves", super::paths::ref_file_stem(&imported.artifact_ref)),
    )?;
    write_preserves(&root, &receipt_path, &imported.receipt_value)?;
    Ok(imported.artifact_ref)
}

pub(super) fn index_receipt_refs<Root: ?Sized>(root: &Root) -> crate::error::Result<Vec<String>> {
    let root_ref = state_root_profile_ref(root)?;
    let mut refs = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        refs.push(local_ref("node-index-verify", &format!("{root_ref}:{name}"))?);
    }
    Ok(refs)
}

pub(super) fn resource_receipt_refs(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Vec<String>> {
    Ok(vec![local_ref("node-resource-profile", &state_root_profile_ref(root)?)?])
}

pub(super) fn capability_receipt_refs(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Vec<String>> {
    Ok(vec![local_ref("node-authority-profile", &state_root_profile_ref(root)?)?])
}

pub(super) fn profile_metadata_refs(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Vec<String>> {
    let mut refs = vec![local_ref(
        "node-production-profile-metadata",
        &format!("{}:{}", crate::preserves_rail::PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA, state_root_profile_ref(root)?),
    )?];
    let profile_resolution = fixed_node_path(super::settings::PROFILE_RESOLUTION_FILE)?;
    if root.try_exists(&profile_resolution)? {
        let resolution_value = read_preserves(root, &profile_resolution)?;
        refs.extend(profile_resolution_metadata_refs(&resolution_value)?);
        refs.push(crate::preserves_rail::canonical_hash(&resolution_value)?);
    }
    Ok(refs)
}

fn profile_resolution_metadata_refs(value: &preserves::IOValue) -> crate::error::Result<Vec<String>> {
    let fields = value
        .collect_simple_record("node-profile-config-resolution-v1", Some(9))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-profile-config-resolution-v1 ...>"))?;
    Ok(vec![super::fields::record_ref_string(&fields[3], "profile")?])
}

pub(super) fn state_root_profile_ref<Root: ?Sized>(_root: &Root) -> crate::error::Result<String> {
    local_ref("node-state-root-profile", "operator-selected-node-state-v1")
}

pub(super) fn local_ref(kind: &str, label: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("node-daemon-local-ref-v1", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(label),
    ]))
}

pub(super) fn ensure_state_layout<Root: NodeStateAuthority + ?Sized>(source: &Root) -> crate::error::Result<()> {
    source.acquire_node_state_root()?.create_layout()
}

pub(super) fn validate_state_root(state_root: &std::path::Path) -> crate::error::Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(crate::error::Failure::invalid_harness("node daemon requires explicit state root"));
    }
    if state_root == std::path::Path::new(".") {
        return Err(crate::error::Failure::invalid_harness(
            "node daemon state root cannot be ambient current directory",
        ));
    }
    Ok(())
}

pub(super) fn validate_node_id(node_id: &str) -> crate::error::Result<()> {
    if node_id.trim().is_empty() {
        Err(crate::error::Failure::invalid_harness("node daemon id must not be empty"))
    } else {
        Ok(())
    }
}

pub(super) fn fixed_node_path(value: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse(value)
}

pub(super) fn write_preserves(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
    value: &preserves::IOValue,
) -> crate::error::Result<()> {
    root.write(path, crate::preserves_rail::to_text(value)?.as_bytes())
}

pub(super) fn read_preserves(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
) -> crate::error::Result<preserves::IOValue> {
    let text = root.read_to_string(path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    crate::preserves_rail::parse_text(&text)
}
