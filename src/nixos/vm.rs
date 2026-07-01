use preserves::IOValue;

type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const NIXOS_VM_NODE_EVIDENCE_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_NODE_EVIDENCE_SCHEMA;
const NIXOS_VM_TEST_RUN_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_TEST_RUN_SCHEMA;
const NIXOS_VM_TOPOLOGY_SCHEMA: &str = crate::preserves_rail::NIXOS_VM_TOPOLOGY_SCHEMA;

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

const MAX_VM_NODES: usize = 16;
const MAX_VM_REFS: usize = 256;
const MAX_VM_TEXT_FIELDS: usize = 128;
const _: () = assert!(MAX_VM_NODES <= 100_000);
const _: () = assert!(MAX_VM_REFS <= 100_000);
const _: () = assert!(MAX_VM_TEXT_FIELDS <= 100_000);

pub struct NixosVmTopologyInput<'a> {
    pub nodes: &'a [String],
    pub package_ref: &'a str,
    pub package_path: &'a str,
    pub network: &'a str,
    pub nix_inputs: &'a [String],
    pub caveats: &'a [String],
}

pub struct NixosVmNodeEvidenceInput<'a> {
    pub node: &'a str,
    pub state_root: &'a str,
    pub identity_receipt_ref: Option<&'a str>,
    pub startup_receipt_ref: &'a str,
    pub health_receipt_ref: &'a str,
    pub control_loop_receipt_ref: &'a str,
    pub heartbeat_receipt_ref: &'a str,
    pub shutdown_receipt_ref: Option<&'a str>,
    pub log_refs: &'a [String],
}

pub struct NixosVmTestRunInput<'a> {
    pub decision: &'a str,
    pub topology_ref: &'a str,
    pub scenario: &'a str,
    pub fault_profile: &'a str,
    pub node_evidence_refs: &'a [String],
    pub child_workflow_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub log_refs: &'a [String],
    pub caveats: &'a [String],
}

pub fn topology_value(input: &NixosVmTopologyInput<'_>) -> Result<IOValue> {
    validate_nodes(input.nodes)?;
    validate_text_field("package ref", input.package_ref)?;
    validate_text_field("package path", input.package_path)?;
    validate_text_field("network", input.network)?;
    Ok(record("nixos-vm-topology-v1", vec![
        string(NIXOS_VM_TOPOLOGY_SCHEMA),
        record("nodes", vec![sequence(node_values(input.nodes)?)]),
        record("package", vec![record("molten-package", vec![
            record("ref", vec![string(input.package_ref)]),
            record("path", vec![string(input.package_path)]),
        ])]),
        record("network", vec![string(input.network)]),
        record("nix-inputs", vec![sequence(string_values("nix input", input.nix_inputs, MAX_VM_REFS)?)]),
        record("caveats", vec![sequence(string_values(
            "topology caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("headless-topology", "pass"),
            check_value("explicit-state-roots", "pass"),
            check_value("no-undeclared-host-state", "pass"),
        ])]),
    ]))
}

pub fn node_evidence_value(input: &NixosVmNodeEvidenceInput<'_>) -> Result<IOValue> {
    validate_text_field("node", input.node)?;
    validate_text_field("state root", input.state_root)?;
    validate_optional_ref("identity receipt", input.identity_receipt_ref)?;
    validate_content_ref(input.startup_receipt_ref)?;
    validate_content_ref(input.health_receipt_ref)?;
    validate_content_ref(input.control_loop_receipt_ref)?;
    validate_content_ref(input.heartbeat_receipt_ref)?;
    validate_optional_ref("shutdown receipt", input.shutdown_receipt_ref)?;
    validate_ref_slice("node log", input.log_refs)?;
    Ok(record("nixos-vm-node-evidence-v1", vec![
        string(NIXOS_VM_NODE_EVIDENCE_SCHEMA),
        record("node", vec![string(input.node)]),
        record("state-root", vec![string(input.state_root)]),
        record("identity-receipt", vec![optional_ref_value(input.identity_receipt_ref)]),
        record("startup-receipt", vec![string(input.startup_receipt_ref)]),
        record("health-receipt", vec![string(input.health_receipt_ref)]),
        record("control-loop-receipt", vec![string(input.control_loop_receipt_ref)]),
        record("heartbeat-receipt", vec![string(input.heartbeat_receipt_ref)]),
        record("shutdown-receipt", vec![optional_ref_value(input.shutdown_receipt_ref)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("checks", vec![sequence(vec![
            check_value("startup-receipt-bound", "pass"),
            check_value("health-receipt-bound", "pass"),
            check_value("control-loop-under-systemd", "pass"),
            check_value("logs-diagnostic-only", "pass"),
        ])]),
    ]))
}

pub fn test_run_value(input: &NixosVmTestRunInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("scenario", input.scenario)?;
    validate_text_field("fault profile", input.fault_profile)?;
    validate_ref_slice("node evidence", input.node_evidence_refs)?;
    validate_ref_slice("child workflow", input.child_workflow_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_ref_slice("log", input.log_refs)?;
    Ok(record("nixos-vm-test-run-v1", vec![
        string(NIXOS_VM_TEST_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("topology", vec![string(input.topology_ref)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-profile", vec![string(input.fault_profile)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("child-workflows", vec![sequence(ref_values(input.child_workflow_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "diagnostic",
            input.diagnostics,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "test caveat",
            input.caveats,
            MAX_VM_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("terminal-output-diagnostic-only", "pass"),
            check_value("vm-evidence-does-not-grant-authority", "pass"),
            check_value("skip-is-not-pass-evidence", "pass"),
        ])]),
    ]))
}

fn validate_nodes(nodes: &[String]) -> Result<()> {
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("nixos VM topology requires at least one node"));
    }
    if nodes.len() > MAX_VM_NODES {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM topology node count {} exceeds bound {MAX_VM_NODES}",
            nodes.len()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for node in nodes {
        validate_text_field("node", node)?;
        if !seen.insert(node.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate nixos VM node {node}")));
        }
    }
    Ok(())
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("nixos VM {label} must not be empty")));
    }
    Ok(())
}

fn validate_optional_ref(label: &str, reference: Option<&str>) -> Result<()> {
    if let Some(value) = reference {
        validate_content_ref(value)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid nixos VM {label} ref {value}: {error}")))?;
    }
    Ok(())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_VM_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM {label} ref count {} exceeds bound {MAX_VM_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid nixos VM {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported nixos VM decision {other}; expected pass, deny, unavailable, or skipped"
        ))),
    }
}

fn node_values(nodes: &[String]) -> Result<Vec<IOValue>> {
    let mut values = Vec::with_capacity(nodes.len());
    for node in nodes {
        values.push(record("node", vec![string(node)]));
    }
    Ok(values)
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IOValue>> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn ref_values(refs: &[String]) -> Result<Vec<IOValue>> {
    validate_ref_slice("artifact", refs)?;
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(string(reference));
    }
    Ok(values)
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    match reference {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn check_value(name: &'static str, status: &'static str) -> IOValue {
    record("check", vec![string(name), string(status)])
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
