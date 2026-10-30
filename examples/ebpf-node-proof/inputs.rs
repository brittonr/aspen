use std::os::unix::fs::PermissionsExt;

pub(super) fn read(path: &std::path::Path, maximum: u64) -> super::Result<Vec<u8>> {
    let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
    let leaf = path.file_name().and_then(|value| value.to_str()).ok_or("input leaf unavailable")?;
    let root = molten::node_state::DirectoryRoot::open_existing(parent)?;
    Ok(root.read(&molten::node_state::RelativePath::parse(leaf)?, maximum)?)
}

pub(super) fn source(
    args: &super::Source,
) -> super::Result<(molten::node_state::DirectoryRoot, molten::node_daemon::SelectedNodeOutputValidation)> {
    let root = molten::node_state::DirectoryRoot::open_existing(&args.node_root)?;
    // Existing CLI initialization, not a parallel authoring-only node layout.
    if root.entry_kind(&molten::node_state::RelativePath::parse("identity")?)?.is_none() {
        return Err("run actual molten node init before provisioning".into());
    }
    let bytes = read(&args.source_trust, 16_384)?;
    let trust: serde_json::Value = serde_json::from_slice(&bytes)?;
    if trust.get("output").and_then(serde_json::Value::as_str) != args.signed_output.to_str() {
        return Err("explicit signed output differs from independent source trust".into());
    }
    let locator = molten::node_state::RelativePath::parse("source-gate-trust.json")?;
    if root.entry_kind(&locator)?.is_some() {
        if root.read(&locator, 16_384)? != bytes {
            return Err("existing source trust differs".into());
        }
    } else {
        root.write_new(&locator, &bytes)?;
    }
    let validation = molten::node_daemon::verify_selected_node_output_with_root(&root)?;
    Ok((root, validation))
}

pub(super) fn check_profile(
    selection: &molten::fabric_observability::nodeadmission::NodeObservationSelection,
    bytes: &[u8],
) -> super::Result<()> {
    if molten::preserves_rail::content_ref_from_bytes(bytes) != selection.checked_profile_ref {
        return Err("checked export raw bytes differ from signed intent".into());
    }
    let export = molten::fabric_observability::decode_ebpf_profile(bytes)
        .map_err(|_| "checked profile syntax or canonical Onix roles denied")?;
    let profile = molten::fabric_observability::ebpf_observation_profile(&export)
        .map_err(|_| "checked observation profile denied")?;
    let descriptors =
        molten::fabric_observability::ebpf_metric_descriptors(&export).map_err(|_| "checked metric mapping denied")?;
    molten::fabric_observability::checked_ebpf_profile(export, &profile, &descriptors)
        .map_err(|_| "checked observation adapter denied")?;
    Ok(())
}

pub(super) fn install(
    root: &molten::node_state::DirectoryRoot,
    selection: &molten::fabric_observability::nodeadmission::NodeObservationSelection,
    profile: &[u8],
    endpoint: &[u8],
) -> super::Result<()> {
    // Both immutable copies live only under the already-owned Registry namespace.
    let relative_profile = selection
        .checked_profile_path
        .strip_prefix("registry/")
        .ok_or("checked profile must be root-relative under registry/")?;
    let profile_path = molten::node_state::RelativePath::parse(relative_profile)?;
    let directory = molten::node_state::RelativePath::parse(&selection.endpoint_directory)?;
    let leaf = molten::node_state::RelativePath::parse(&selection.endpoint_leaf)?;
    if leaf.as_path().components().count() != 1 {
        return Err("endpoint must select one regular leaf".into());
    }
    root.existing_namespace(molten::node_state::NamespaceKind::Registry)?
        .ok_or("initialized Registry namespace unavailable")?;
    let endpoint_directory =
        molten::node_state::RelativePath::parse(&format!("registry/{}", directory.as_path().display()))?;
    root.create_dir_all(&endpoint_directory)?;
    let endpoint_path = molten::node_state::RelativePath::parse(&format!(
        "registry/{}/{}",
        selection.endpoint_directory, selection.endpoint_leaf
    ))?;
    let profile_path =
        molten::node_state::RelativePath::parse(&format!("registry/{}", profile_path.as_path().display()))?;
    copy_readonly(root, &profile_path, profile)?;
    copy_readonly(root, &endpoint_path, endpoint)
}

fn copy_readonly(
    root: &molten::node_state::DirectoryRoot,
    path: &molten::node_state::RelativePath,
    bytes: &[u8],
) -> super::Result<()> {
    if let Some(parent) = path.as_path().parent().filter(|path| !path.as_os_str().is_empty()) {
        root.create_dir_all(&molten::node_state::RelativePath::parse(parent.to_str().ok_or("copy parent encoding")?)?)?;
    }
    root.write_new(path, bytes)?;
    let file = root.open_database_file(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o444))?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn seed(path: &std::path::Path) -> super::Result<[u8; 32]> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("operator seed requires a private regular file".into());
    }
    let mut bytes = read(path, 32)?;
    let result = bytes.as_slice().try_into().map_err(|_| "operator seed must be exactly 32 raw bytes".into());
    bytes.fill(0);
    result
}
