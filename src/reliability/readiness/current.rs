//! Capability-rooted current-node evidence, reused by signer and verifier.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub source_binding_ref: String,
    pub source_trust_ref: String,
    pub startup_ref: String,
    pub health_ref: String,
    pub run_ref: String,
}

pub fn read(root: &crate::node_state::DirectoryRoot, health_path: &str) -> crate::error::Result<Node> {
    let trust_ref = source_trust_ref(root)?;
    let source = crate::node_daemon::admit_startup_source_gate_with_root(root)?;
    read_with_admitted_source(root, health_path, &source, &trust_ref)
}

/// The source token authenticates an immutable signed Nix store cohort. The local trust selection
/// must remain byte-identical; changed selections require full source admission, never cached
/// trust.
pub fn read_with_admitted_source(
    root: &crate::node_state::DirectoryRoot,
    health_path: &str,
    source: &crate::node_runtime::ValidatedNodeSourceGate,
    expected_trust_ref: &str,
) -> crate::error::Result<Node> {
    let trust_ref = source_trust_ref(root)?;
    if trust_ref != expected_trust_ref {
        return Err(denied("source trust selection changed; full re-admission required"));
    }
    let startup = crate::node_runtime::parse_node_startup_receipt(&artifact(root, "startup-receipt.preserves")?)?;
    let health = crate::node_runtime::parse_node_health_receipt(&artifact(root, health_path)?)?;
    if startup.decision != "pass"
        || health.decision != "pass"
        || health.startup_receipt_ref != startup.receipt_ref
        || health.shutdown_receipt_ref.is_some()
        || !health.open_job_refs.is_empty()
        || startup.source_gate_receipt_refs.len() != 1
        || startup.source_gate_receipt_refs[0] != source.gate_ref()
        || startup.source_gate_validation_refs.len() != 1
        || startup.source_gate_validation_refs[0] != source.validation_ref()
    {
        return Err(denied("current canonical startup/health does not bind admitted source"));
    }
    let binding = crate::node_runtime::parse_node_run_binding(&artifact(root, "control/current-run.preserves")?)?;
    if binding.startup_receipt_ref != startup.receipt_ref {
        return Err(denied("stale current-run startup"));
    }
    let lock = artifact(root, "control/node.lock.preserves")?;
    let fields = lock
        .collect_simple_record("node-control-lock-v2", Some(7))
        .ok_or_else(|| denied("missing canonical active node lock"))?;
    if crate::preserves_rail::required_string_field(&fields[0], "node lock schema")?
        != crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA_V2
    {
        return Err(denied("wrong active node lock schema"));
    }
    let state = crate::preserves_rail::record_string_field(&fields[1], "state-root", "lock state root")?;
    let startup_ref = crate::preserves_rail::record_string_field(&fields[2], "startup", "lock startup")?;
    let run_ref = crate::preserves_rail::record_string_field(&fields[6], "run", "lock current run")?;
    if state != binding.state_root_ref || startup_ref != startup.receipt_ref || run_ref != binding.run_ref {
        return Err(denied("active node lock does not bind current run"));
    }
    let completion = optional(root, "control/current-run-shutdown.preserves")?
        .as_ref()
        .map(crate::node_runtime::parse_node_run_shutdown_observation)
        .transpose()?;
    let shutdown = optional(root, "shutdown-receipt.preserves")?
        .as_ref()
        .map(crate::node_runtime::parse_node_shutdown_receipt)
        .transpose()?;
    let lifecycle = crate::node_runtime::classify_node_run_lifecycle(&crate::node_runtime::NodeRunLifecycleInput {
        operation: "shutdown",
        is_duplicate: false,
        current_run_ref: &binding.run_ref,
        current_startup_receipt_ref: &startup.receipt_ref,
        active_run_ref: Some(&run_ref),
        shutdown_observation: completion.as_ref(),
        shutdown_receipt: shutdown.as_ref(),
        control_receipt: None,
    });
    if lifecycle.lifecycle != crate::node_runtime::NodeRunLifecycle::Active {
        return Err(denied("node lifecycle is not currently active"));
    }
    if source_trust_ref(root)? != trust_ref {
        return Err(denied("source trust changed during current-run check"));
    }
    Ok(Node {
        source_binding_ref: source.binding_ref().into(),
        startup_ref: startup.receipt_ref,
        health_ref: health.receipt_ref,
        run_ref,
        source_trust_ref: trust_ref,
    })
}

pub fn artifact(root: &crate::node_state::DirectoryRoot, path: &str) -> crate::error::Result<preserves::IOValue> {
    let path = crate::node_state::RelativePath::parse(path)?;
    let text = root.read_to_string(&path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    crate::preserves_rail::parse_text(&text)
}

fn optional(root: &crate::node_state::DirectoryRoot, path: &str) -> crate::error::Result<Option<preserves::IOValue>> {
    if !root.try_exists(&crate::node_state::RelativePath::parse(path)?)? {
        return Ok(None);
    }
    artifact(root, path).map(Some)
}

fn denied(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn source_trust_ref(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<String> {
    let path = crate::node_state::RelativePath::parse("source-gate-trust.json")?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&root.read(&path, 16_384)?))
}
