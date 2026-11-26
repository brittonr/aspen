//! Local and profile-backed node initialization.

pub fn init_local(input: &super::requests::InitInput<'_>) -> crate::error::Result<super::outcomes::Init> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    init_local_with_root(&root, input)
}

pub fn init_local_with_root(
    root: &crate::node_state::DirectoryRoot,
    input: &super::requests::InitInput<'_>,
) -> crate::error::Result<super::outcomes::Init> {
    super::storage::validate_node_id(input.node_id)?;
    verify_init_state(root).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("node init state verification failed: {error}"))
    })?;
    super::storage::ensure_state_layout(root).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("node init layout creation failed: {error}"))
    })?;
    let policy_refs = vec![super::storage::local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, std::path::PathBuf::from("identity"));
    identity_config.policy_refs = policy_refs.clone();
    let identity_root = root.identity().map_err(|error| {
        crate::error::Failure::invalid_harness(format!("node init identity namespace failed: {error}"))
    })?;
    let identity_resolution =
        crate::node_identity::resolve_with_root(&identity_config, &identity_root).map_err(|error| {
            crate::error::Failure::invalid_harness(format!("node init identity resolution failed: {error}"))
        })?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| crate::error::Failure::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(root).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("node init adapter binding failed: {error}"))
    })?;
    let capability_refs = vec![super::storage::local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![super::storage::local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![super::storage::local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = super::storage::state_root_profile_ref(root)?;
    let profile_resolution =
        crate::node_profile_config::resolve_local_default(&crate::node_profile_config::LocalDefaultInput {
            identity_ref: identity.identity_ref.clone(),
            state_root_ref,
            adapters,
            policy_refs,
            capability_refs,
            resource_refs,
            effect_profile_refs,
        })?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONFIG_FILE)?,
        &profile_resolution.config_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::PROFILE_RESOLUTION_FILE)?,
        &profile_resolution.resolution_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::IDENTITY_RECEIPT_FILE)?,
        &identity_resolution.receipt_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::IDENTITY_FILE)?,
        &identity.value,
    )?;
    Ok(super::outcomes::Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}

pub fn init_with_profile(input: &super::requests::ProfileInitInput<'_>) -> crate::error::Result<super::outcomes::Init> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    init_with_profile_and_root(&root, input)
}

pub fn init_with_profile_and_root(
    root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ProfileInitInput<'_>,
) -> crate::error::Result<super::outcomes::Init> {
    super::storage::validate_node_id(input.node_id)?;
    verify_init_state(root)?;
    super::storage::ensure_state_layout(root)?;
    let policy_refs = vec![super::storage::local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, std::path::PathBuf::from("identity"));
    identity_config.policy_refs = policy_refs;
    let identity_root = root.identity()?;
    let identity_resolution = crate::node_identity::resolve_with_root(&identity_config, &identity_root)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| crate::error::Failure::invalid_harness("node daemon identity resolution denied"))?;
    let profile_resolution = crate::node_profile_config::resolve_backed(&crate::node_profile_config::BackedInput {
        identity_ref: identity.identity_ref.clone(),
        profile: input.profile.clone(),
        overrides: input.overrides.clone(),
    })?;
    if profile_resolution.decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node profile-backed init denied: {}",
            profile_resolution.diagnostics.join("; ")
        )));
    }
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONFIG_FILE)?,
        &profile_resolution.config_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::PROFILE_RESOLUTION_FILE)?,
        &profile_resolution.resolution_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::IDENTITY_RECEIPT_FILE)?,
        &identity_resolution.receipt_value,
    )?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::IDENTITY_FILE)?,
        &identity.value,
    )?;
    Ok(super::outcomes::Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}

fn verify_init_state(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<()> {
    let state = super::inspection::inspect_node_lifecycle_state_with_root(root)?;
    if state == super::inspection::NodeLifecycleState::Empty {
        return Ok(());
    }
    Err(crate::error::Failure::invalid_harness(format!(
        "node daemon init denied: state root already has {state:?} lifecycle state; use an explicit reset before reinitializing"
    )))
}

fn default_adapter_bindings(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<Vec<crate::node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref = super::storage::local_ref(
            "node-adapter-profile",
            &format!("{}:{name}", super::storage::state_root_profile_ref(root)?),
        )?;
        adapters.push(crate::node_runtime::node_adapter_binding(name, &profile_ref)?);
    }
    Ok(adapters)
}
