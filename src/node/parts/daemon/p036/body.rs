
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

fn verify_restart_state(state_root: &Path) -> Result<()> {
    let startup_path = state_root.join(STARTUP_FILE);
    if startup_path.exists() {
        let shutdown_path = state_root.join(SHUTDOWN_FILE);
        if !shutdown_path.exists() {
            return Err(MoltenError::invalid_harness(
                "node daemon restart denied: previous startup has no clean shutdown receipt",
            ));
        }
        let startup_value = read_preserves(&startup_path)?;
        let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
        let shutdown_ref = crate::preserves_rail::canonical_hash(&read_preserves(&shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &index_receipt_refs(state_root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            },
        )?;
        let health = crate::node_runtime::parse_node_health_receipt(&health_value)?;
        write_preserves(&state_root.join(HEALTH_FILE), &health_value)?;
        if health.decision != "pass" {
            return Err(MoltenError::invalid_harness(format!(
                "node daemon restart recovery denied receipt={}",
                health.receipt_ref
            )));
        }
        fs::remove_file(shutdown_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn default_adapter_bindings(state_root: &Path) -> Result<Vec<crate::node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref =
            local_ref("node-adapter-profile", &format!("{}:{name}", state_root_profile_ref(state_root)?))?;
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

fn index_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    let root_ref = state_root_profile_ref(state_root)?;
    let mut refs = Vec::with_capacity(crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        refs.push(local_ref("node-index-verify", &format!("{root_ref}:{name}"))?);
    }
    Ok(refs)
}

fn resource_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-resource-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn capability_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-authority-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn profile_metadata_refs(state_root: &Path) -> Result<Vec<String>> {
    let mut refs = vec![local_ref(
        "node-production-profile-metadata",
        &format!(
            "{}:{}",
            crate::preserves_rail::PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            state_root_profile_ref(state_root)?
        ),
    )?];
    let profile_resolution = state_root.join(PROFILE_RESOLUTION_FILE);
    if profile_resolution.exists() {
        let resolution_value = read_preserves(&profile_resolution)?;
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

fn state_root_profile_ref(state_root: &Path) -> Result<String> {
    local_ref("node-state-root-profile", &state_root.display().to_string())
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("node-daemon-local-ref-v1", vec![
        crate::preserves_rail::string(kind),
        crate::preserves_rail::string(label),
    ]))
}

fn ensure_state_layout(state_root: &Path) -> Result<()> {
    fs::create_dir_all(state_root).map_err(MoltenError::from)?;
    for child in [
        "identity",
        "ledger",
        "registry",
        "chunks",
        "storage",
        "cache",
        "remote-dataspace",
        "services",
        "jobs",
        "coordination",
        "plugin-host",
        "catalog-mcp",
        "control",
        CONTROL_INBOX_DIR,
        CONTROL_OUTBOX_DIR,
        CONTROL_INGRESS_DIR,
        CONTROL_IDEMPOTENCY_DIR,
        CONTROL_SERVICE_DIR,
        "receipts",
    ] {
        fs::create_dir_all(state_root.join(child)).map_err(MoltenError::from)?;
    }
    Ok(())
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

fn write_preserves(path: &Path, value: &IoValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, crate::preserves_rail::to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IoValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    crate::preserves_rail::parse_text(&text)
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

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p004/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p005/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p006/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p007/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p008/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p009/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p010/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p011/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p012/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/parts/daemon/tests/m000/p013/body.rs"));
}
