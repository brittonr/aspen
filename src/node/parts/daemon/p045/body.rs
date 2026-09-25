
pub fn init_local_with_root(root: &crate::node_state::NodeStateRoot, input: &InitInput<'_>) -> Result<Init> {
    validate_node_id(input.node_id)?;
    verify_init_state(root)
        .map_err(|error| MoltenError::invalid_harness(format!("node init state verification failed: {error}")))?;
    ensure_state_layout(root)
        .map_err(|error| MoltenError::invalid_harness(format!("node init layout creation failed: {error}")))?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, PathBuf::from("identity"));
    identity_config.policy_refs = policy_refs.clone();
    let identity_root = root
        .identity()
        .map_err(|error| MoltenError::invalid_harness(format!("node init identity namespace failed: {error}")))?;
    let identity_resolution = crate::node_identity::resolve_with_root(&identity_config, &identity_root)
        .map_err(|error| MoltenError::invalid_harness(format!("node init identity resolution failed: {error}")))?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(root)
        .map_err(|error| MoltenError::invalid_harness(format!("node init adapter binding failed: {error}")))?;
    let capability_refs = vec![local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = state_root_profile_ref(root)?;
    let profile_resolution = crate::node_profile_config::resolve_local_default_config(
        &crate::node_profile_config::LocalDefaultConfigInput {
            identity_ref: identity.identity_ref.clone(),
            state_root_ref,
            adapters,
            policy_refs,
            capability_refs,
            resource_refs,
            effect_profile_refs,
        },
    )?;
    write_preserves(root, &fixed_node_path(CONFIG_FILE)?, &profile_resolution.config_value)?;
    write_preserves(
        root,
        &fixed_node_path(PROFILE_RESOLUTION_FILE)?,
        &profile_resolution.resolution_value,
    )?;
    write_preserves(
        root,
        &fixed_node_path(IDENTITY_RECEIPT_FILE)?,
        &identity_resolution.receipt_value,
    )?;
    write_preserves(root, &fixed_node_path(IDENTITY_FILE)?, &identity.value)?;
    Ok(Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}

pub fn init_with_profile(input: &ProfileInitInput<'_>) -> Result<Init> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    init_with_profile_and_root(&root, input)
}

pub fn init_with_profile_and_root(
    root: &crate::node_state::NodeStateRoot,
    input: &ProfileInitInput<'_>,
) -> Result<Init> {
    validate_node_id(input.node_id)?;
    verify_init_state(root)?;
    ensure_state_layout(root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, PathBuf::from("identity"));
    identity_config.policy_refs = policy_refs;
    let identity_root = root.identity()?;
    let identity_resolution = crate::node_identity::resolve_with_root(&identity_config, &identity_root)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let profile_resolution = crate::node_profile_config::resolve_profile_backed_config(
        &crate::node_profile_config::ProfileBackedConfigInput {
            identity_ref: identity.identity_ref.clone(),
            profile: input.profile.clone(),
            overrides: input.overrides.clone(),
        },
    )?;
    if profile_resolution.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node profile-backed init denied: {}",
            profile_resolution.diagnostics.join("; ")
        )));
    }
    write_preserves(root, &fixed_node_path(CONFIG_FILE)?, &profile_resolution.config_value)?;
    write_preserves(
        root,
        &fixed_node_path(PROFILE_RESOLUTION_FILE)?,
        &profile_resolution.resolution_value,
    )?;
    write_preserves(
        root,
        &fixed_node_path(IDENTITY_RECEIPT_FILE)?,
        &identity_resolution.receipt_value,
    )?;
    write_preserves(root, &fixed_node_path(IDENTITY_FILE)?, &identity.value)?;
    Ok(Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}
