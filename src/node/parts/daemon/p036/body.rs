
fn validate_ingress_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_member_ref(actual: &str, expected: &str, label: &str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} ref {actual} does not match {expected}")))
    }
}

fn validate_optional_member_ref(value: Option<&IoValue>, expected_ref: Option<&str>, label: &str) -> Result<()> {
    match (value, expected_ref) {
        (Some(value), Some(expected)) => {
            validate_member_ref(&crate::preserves_rail::canonical_hash(value)?, expected, label)
        }
        (Some(_), None) => Err(MoltenError::invalid_harness(format!("{label} value present without ref"))),
        (None, Some(expected)) => {
            Err(MoltenError::invalid_harness(format!("{label} ref {expected} present without value")))
        }
        (None, None) => Ok(()),
    }
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn verify_init_state(root: &crate::node_state::NodeStateRoot) -> Result<()> {
    let state = inspect_node_lifecycle_state_with_root(root)?;
    if state == NodeLifecycleState::Empty {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!(
        "node daemon init denied: state root already has {state:?} lifecycle state; use an explicit reset before reinitializing"
    )))
}

pub fn inspect_node_lifecycle_state(state_root: &Path) -> NodeLifecycleState {
    let Ok(root) = crate::node_state::NodeStateRoot::open_existing(state_root) else {
        return NodeLifecycleState::Empty;
    };
    inspect_node_lifecycle_state_with_root(&root).unwrap_or(NodeLifecycleState::Inconsistent)
}

pub fn inspect_node_lifecycle_state_with_root(
    root: &crate::node_state::NodeStateRoot,
) -> Result<NodeLifecycleState> {
    Ok(node_lifecycle_state(&node_lifecycle_files_with_root(root)?))
}

pub fn node_lifecycle_files(state_root: &Path) -> NodeLifecycleFiles {
    let Ok(root) = crate::node_state::NodeStateRoot::open_existing(state_root) else {
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

pub fn node_lifecycle_files_with_root(root: &crate::node_state::NodeStateRoot) -> Result<NodeLifecycleFiles> {
    Ok(NodeLifecycleFiles {
        has_config: root.try_exists(&fixed_node_path(CONFIG_FILE)?)?,
        has_identity_receipt: root.try_exists(&fixed_node_path(IDENTITY_RECEIPT_FILE)?)?,
        has_startup: root.try_exists(&fixed_node_path(STARTUP_FILE)?)?,
        has_shutdown: root.try_exists(&fixed_node_path(SHUTDOWN_FILE)?)?,
        has_active_lock: root.try_exists(&fixed_node_path(CONTROL_LOCK_FILE)?)?,
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
    if files.has_config && files.has_identity_receipt && !files.has_startup && !files.has_shutdown && !files.has_active_lock {
        return NodeLifecycleState::Initialized;
    }
    if files.has_config && files.has_identity_receipt && files.has_startup && !files.has_shutdown && files.has_active_lock {
        return NodeLifecycleState::Running;
    }
    if files.has_config && files.has_identity_receipt && files.has_startup && files.has_shutdown && !files.has_active_lock {
        return NodeLifecycleState::Stopped;
    }
    NodeLifecycleState::Inconsistent
}

fn verify_restart_state(root: &crate::node_state::NodeStateRoot) -> Result<()> {
    let startup_path = fixed_node_path(STARTUP_FILE)?;
    if root.try_exists(&startup_path)? {
        let shutdown_path = fixed_node_path(SHUTDOWN_FILE)?;
        if !root.try_exists(&shutdown_path)? {
            return Err(MoltenError::invalid_harness(
                "node daemon restart denied: previous startup has no clean shutdown receipt",
            ));
        }
        let startup_value = read_preserves(root, &startup_path)?;
        let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
        let shutdown_ref = crate::preserves_rail::canonical_hash(&read_preserves(root, &shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &index_receipt_refs(root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            },
        )?;
        let health = crate::node_runtime::parse_node_health_receipt(&health_value)?;
        write_preserves(root, &fixed_node_path(HEALTH_FILE)?, &health_value)?;
        if health.decision != "pass" {
            return Err(MoltenError::invalid_harness(format!(
                "node daemon restart recovery denied receipt={}",
                health.receipt_ref
            )));
        }
        root.remove_regular_file(&shutdown_path)?;
    }
    Ok(())
}

fn default_adapter_bindings(root: &crate::node_state::NodeStateRoot) -> Result<Vec<crate::node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref =
            local_ref("node-adapter-profile", &format!("{}:{name}", state_root_profile_ref(root)?))?;
        adapters.push(crate::node_runtime::node_adapter_binding(name, &profile_ref)?);
    }
    Ok(adapters)
}

fn status_request() -> Result<crate::node_runtime::ControlRequest> {
    control_request("status")
}

fn shutdown_request() -> Result<crate::node_runtime::ControlRequest> {
    control_request("shutdown")
}

fn control_request(operation: &str) -> Result<crate::node_runtime::ControlRequest> {
    let authority_refs = vec![local_ref("node-control-authority", operation)?];
    let policy_refs = vec![local_ref("node-control-policy", operation)?];
    let resource_refs = vec![local_ref("node-control-resource", operation)?];
    let value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation,
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })?;
    crate::node_runtime::parse_control_request(&value)
}

#[cfg(test)]
fn test_live_authority_refs(
    state_root: &Path,
    peer_id: &str,
    node_id: &str,
    operation: &str,
    policy_refs: &[String],
) -> Result<Vec<String>> {
    let operations = vec![operation.to_string()];
    let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
        peer_id,
        node_id,
        operations: &operations,
        target_scope: "*",
        resource_scope: "*",
        epoch: 1,
        expires_at: None,
        policy_refs,
        revocation_refs: &[],
        evidence_refs: &[],
    })?;
    let grant = import_control_authority_grant(state_root, &grant_value)?;
    Ok(vec![grant.grant_ref])
}

#[cfg(test)]
fn test_live_peer_bootstrap_refs(
    state_root: &Path,
    peer_id: &str,
    topic: &str,
    policy_refs: &[String],
) -> Result<Vec<String>> {
    let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
        state_root,
        topic,
        policy_refs,
        evidence_refs: &[],
    })?;
    let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
        state_root,
        ticket_value: &ticket.value,
        peer_id,
        sequence: 1,
        expires_at: None,
        policy_refs,
        evidence_refs: &[],
    })?;
    Ok(vec![admission.admission_ref])
}

fn index_receipt_refs<Root: ?Sized>(root: &Root) -> Result<Vec<String>> {
    let root_ref = state_root_profile_ref(root)?;
    let mut refs = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        refs.push(local_ref("node-index-verify", &format!("{root_ref}:{name}"))?);
    }
    Ok(refs)
}

fn resource_receipt_refs(root: &crate::node_state::NodeStateRoot) -> Result<Vec<String>> {
    Ok(vec![local_ref("node-resource-profile", &state_root_profile_ref(root)?)?])
}

fn capability_receipt_refs(root: &crate::node_state::NodeStateRoot) -> Result<Vec<String>> {
    Ok(vec![local_ref("node-authority-profile", &state_root_profile_ref(root)?)?])
}

fn profile_metadata_refs(root: &crate::node_state::NodeStateRoot) -> Result<Vec<String>> {
    let mut refs = vec![local_ref(
        "node-production-profile-metadata",
        &format!(
            "{}:{}",
            crate::preserves_rail::PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            state_root_profile_ref(root)?
        ),
    )?];
    let profile_resolution = fixed_node_path(PROFILE_RESOLUTION_FILE)?;
    if root.try_exists(&profile_resolution)? {
        let resolution_value = read_preserves(root, &profile_resolution)?;
        refs.extend(profile_resolution_metadata_refs(&resolution_value)?);
        refs.push(crate::preserves_rail::canonical_hash(&resolution_value)?);
    }
    Ok(refs)
}

fn profile_resolution_metadata_refs(value: &IoValue) -> Result<Vec<String>> {
    let fields = value
        .collect_simple_record("node-profile-config-resolution-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-profile-config-resolution-v1 ...>"))?;
    Ok(vec![record_ref_string(&fields[3], "profile")?])
}

fn state_root_profile_ref<Root: ?Sized>(_root: &Root) -> Result<String> {
    local_ref("node-state-root-profile", "operator-selected-node-state-v1")
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("node-daemon-local-ref-v1", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(label),
    ]))
}

fn ensure_state_layout<Root: NodeStateAuthority + ?Sized>(source: &Root) -> Result<()> {
    source.acquire_node_state_root()?.create_layout()
}

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
