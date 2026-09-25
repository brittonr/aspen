
fn validate_fault_kind(kind: &str) -> Result<()> {
    match kind {
        "network-delay"
        | "network-drop"
        | "network-partition"
        | "network-rejoin"
        | "asymmetric-latency"
        | "crash-restart"
        | "duplicate-send-after-restart"
        | "receipt-write-readback"
        | "missing-artifact"
        | "permission-denied-state-root"
        | "bounded-disk-pressure"
        | "unsupported-host-feature"
        | "tampered-fault-receipt"
        | "wrong-topology"
        | "log-only-pass" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM fault kind {other}"))),
    }
}

fn validate_host_support(status: &str) -> Result<()> {
    match status {
        "supported" | "unavailable" | "denied" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM host-support status {other}"))),
    }
}

fn validate_vm_evidence_scope(scope: &str) -> Result<()> {
    match scope {
        NIXOS_VM_SCOPE_FIXTURE_METADATA
        | NIXOS_VM_SCOPE_EXECUTABLE_VM
        | NIXOS_VM_SCOPE_AGGREGATE_INDEX
        | NIXOS_VM_SCOPE_DIAGNOSTIC_ONLY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported nixos VM evidence scope {other}"))),
    }
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

fn node_values(nodes: &[String]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(nodes.len());
    for node in nodes {
        values.push(record("node", vec![string(node)]));
    }
    Ok(values)
}

fn validate_strings(label: &str, values: &[String], maximum: usize) -> Result<()> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "nixos VM {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    for value in values {
        validate_text_field(label, value)?;
    }
    Ok(())
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IoValue>> {
    validate_strings(label, values, maximum)?;
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(string(value));
    }
    Ok(output)
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_ref_slice("artifact", refs)?;
    let mut values = Vec::with_capacity(refs.len());
    for reference in refs {
        values.push(string(reference));
    }
    Ok(values)
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_text_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn status(is_passing: bool) -> &'static str {
    if is_passing { "pass" } else { "deny" }
}
