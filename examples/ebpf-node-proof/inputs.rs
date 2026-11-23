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
    let profile_path = molten::node_state::RelativePath::parse(&selection.checked_profile_path)?;
    if !profile_path.as_path().strip_prefix("registry").is_ok_and(|path| !path.as_os_str().is_empty()) {
        return Err("checked profile must be root-relative under registry/".into());
    }
    let directory = molten::node_state::RelativePath::parse(&selection.endpoint_directory)?;
    if !directory.as_path().strip_prefix("receipts").is_ok_and(|path| !path.as_os_str().is_empty()) {
        return Err("endpoint directory must be root-relative under receipts/".into());
    }
    let endpoint_path = directory.join_segment(&selection.endpoint_leaf)?;
    root.existing_namespace(molten::node_state::NamespaceKind::Registry)?
        .ok_or("initialized Registry namespace unavailable")?;
    root.existing_namespace(molten::node_state::NamespaceKind::Receipts)?
        .ok_or("initialized Receipts namespace unavailable")?;
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

#[cfg(test)]
mod tests {
    fn selected(directory: &str) -> molten::fabric_observability::nodeadmission::NodeObservationSelection {
        let reference = molten::preserves_rail::content_ref_from_bytes(b"fixture provenance");
        molten::fabric_observability::nodeadmission::NodeObservationSelection {
            source_validation_ref: reference.clone(),
            checked_profile_path: "registry/observation/profile.json".into(),
            checked_profile_ref: molten::preserves_rail::content_ref_from_bytes(b"checked profile"),
            endpoint_directory: directory.into(),
            endpoint_leaf: "epoch.json".into(),
            endpoint_content_ref: molten::preserves_rail::content_ref_from_bytes(b"exact endpoint bytes"),
            epoch: 7,
            authorization_tick: 1,
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            onix_receipt_ref: reference,
        }
    }

    // r[verify linux_ebpf_observability.boundary]
    #[test]
    fn copied_window_reopens_through_receipt_scope() -> super::super::Result<()> {
        let workspace = crate::test_support::process_workspace("observer-provision-reopens")?;
        let root = molten::node_state::DirectoryRoot::open_existing(workspace.as_ref())?;
        root.create_layout()?;
        let selection = selected("receipts/provider");
        super::install(&root, &selection, b"checked profile", b"exact endpoint bytes")?;

        let receipt_scope = root.existing_namespace(molten::node_state::NamespaceKind::Receipts)?.unwrap();
        let provider = receipt_scope
            .existing_subdir(&molten::node_state::RelativePath::parse("provider")?)?
            .expect("signed receipt scope must contain the provisioned endpoint");
        let leaf = molten::node_state::RelativePath::parse("epoch.json")?;
        let read = provider.read(&leaf, 32)?;
        assert_eq!(molten::preserves_rail::content_ref_from_bytes(&read), selection.endpoint_content_ref);
        assert_eq!(provider.unix_mode(&leaf)?.map(|mode| mode & 0o7777), Some(0o444));
        let profile = root.read(&molten::node_state::RelativePath::parse(&selection.checked_profile_path)?, 32)?;
        assert_eq!(molten::preserves_rail::content_ref_from_bytes(&profile), selection.checked_profile_ref);
        Ok(())
    }

    #[test]
    fn foreign_window_scope_is_refused_before_writing() -> super::super::Result<()> {
        let workspace = crate::test_support::Workspace::new("observer-provision-denies-scope")?;
        let root = molten::node_state::DirectoryRoot::from_dir(workspace.state()?.dir().try_clone()?);
        root.create_layout()?;
        let selection = selected("registry/not-a-receipt-scope");
        assert!(super::install(&root, &selection, b"checked profile", b"exact endpoint bytes").is_err());
        let registry = root.existing_namespace(molten::node_state::NamespaceKind::Registry)?.unwrap();
        assert!(registry.existing_subdir(&molten::node_state::RelativePath::parse("observation")?)?.is_none());
        Ok(())
    }

    // r[verify linux_ebpf_observability.admission]
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires MOLTEN_TEST_EBPF_REPLAY_ROOT containing guarded OnixOS fixture output"]
    fn retained_handoff_keeps_exact_receipt_bytes() -> super::super::Result<()> {
        let selected_root = std::env::var_os("MOLTEN_TEST_EBPF_REPLAY_ROOT").ok_or("select a guarded capture root")?;
        let artifacts = molten::node_state::DirectoryRoot::open_existing(std::path::Path::new(&selected_root))?;
        let read = |leaf, limit| artifacts.read(&molten::node_state::RelativePath::parse(leaf)?, limit);
        let profile_bytes = read("runtime/generation-1/expected-profile.json", 65_536)?;
        let window_bytes = read("runtime/generation-1/final-window.json", 4_194_304)?;
        let binding_bytes = read("runtime/generation-1/expected-binding.json", 16_384)?;
        let original: serde_json::Value = serde_json::from_slice(&read("molten-positive.json", 65_536)?)?;
        let expected: molten::fabric_observability::EbpfBinding = serde_json::from_slice(&binding_bytes)?;
        let export = molten::fabric_observability::decode_ebpf_profile(&profile_bytes)
            .map_err(|_| "retained checked profile denied")?;
        assert_eq!(export.binding, expected);
        let profile = molten::fabric_observability::ebpf_observation_profile(&export).map_err(|_| "invalid profile")?;
        let descriptors = molten::fabric_observability::ebpf_metric_descriptors(&export)
            .map_err(|_| "invalid retained descriptor mapping")?;
        let adapter = molten::fabric_observability::checked_ebpf_profile(export, &profile, &descriptors)
            .map_err(|_| "invalid retained adapter")?;
        let window = molten::fabric_observability::decode_ebpf_window(&window_bytes, adapter.max_bytes as u64)
            .map_err(|_| "retained window denied")?;

        let mut selection = selected("receipts/provider");
        selection.epoch = 1;
        selection.checked_profile_ref = molten::preserves_rail::content_ref_from_bytes(&profile_bytes);
        selection.endpoint_content_ref = molten::preserves_rail::content_ref_from_bytes(&window_bytes);
        let workspace = crate::test_support::Workspace::new("observer-retained-handoff")?;
        let state = molten::node_state::DirectoryRoot::from_dir(workspace.state()?.dir().try_clone()?);
        state.create_layout()?;
        super::check_profile(&selection, &profile_bytes)?;
        super::install(&state, &selection, &profile_bytes, &window_bytes)?;
        let receipts = state.existing_namespace(molten::node_state::NamespaceKind::Receipts)?.unwrap();
        let provider = receipts.existing_subdir(&molten::node_state::RelativePath::parse("provider")?)?.unwrap();
        let observe = |reference: &str| {
            molten::fabric_observability::observe_linux_ebpf(molten::fabric_observability::EbpfServiceInput {
                profile: &profile,
                adapter: &adapter,
                capability: Some(molten::fabric_observability::EbpfReadCapability {
                    root: &provider,
                    leaf: molten::node_state::RelativePath::parse(&selection.endpoint_leaf)?,
                    content_ref: Some(reference),
                }),
                control: molten::fabric_observability::EbpfServiceControl {
                    epoch: selection.epoch,
                    now: window.end,
                    deadline: window.end,
                    cancelled: false,
                },
                context: molten::fabric_observability::ObservationContext {
                    source_id: "linux-ebpf".into(),
                    source_ref: expected.adapter.clone(),
                    profile_ref: profile.profile_ref.clone(),
                    scope: molten::fabric_observability::ClaimScope::LocalComponent,
                    generation: selection.epoch,
                    observed_tick: window.end,
                    valid_until_tick: window.end,
                    resource_ref: expected.scope.clone(),
                    evidence_refs: Vec::new(),
                    non_claims: molten::fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
                },
            })
        };
        let accepted = observe(&selection.endpoint_content_ref)?;
        assert_eq!(accepted.status.artifact.kind.as_str(), original["status"]);
        assert_eq!(accepted.health.artifact.state.as_str(), original["health"]);
        assert_eq!(accepted.samples.len(), original["sample_refs"].as_array().ok_or("missing original samples")?.len());
        assert_eq!(serde_json::to_value(accepted.accounting)?, original["accounting"]);
        assert!(!accepted.deny_required_readiness);

        let replacement = read("runtime/generation-2/final-window.json", 4_194_304)?;
        let other_reference = molten::preserves_rail::content_ref_from_bytes(&replacement);
        let denied = observe(&other_reference)?;
        assert!(denied.deny_required_readiness);
        assert!(denied.samples.is_empty());
        assert_eq!(denied.status.artifact.kind, molten::fabric_observability::AdapterOutcomeKind::Corrupt);
        Ok(())
    }
}
