
fn ref_file_name(reference: &str) -> Result<String> {
    require_ref(reference, "retention file ref")?;
    let name = reference.replace(':', "_");
    ensure_count_at_most(name.len(), MAX_REF_FILE_NAME, "retention ref file name")?;
    Ok(name)
}

fn capability_store_path(suffix: &str) -> Result<LocalStorePath> {
    LocalStorePath::parse(&format!("{STORE_DIR}/{suffix}"))
}

fn capability_ref_path(directory: &str, reference: &str) -> Result<LocalStorePath> {
    capability_store_path(&format!("{directory}/{}.preserves", ref_file_name(reference)?))
}

fn write_store_value_with_root(
    root: &CapabilityRetentionRoot,
    path: &LocalStorePath,
    value: &IoValue,
) -> Result<()> {
    let text = crate::preserves_rail::to_text(value)?;
    root.root().write(path, text.as_bytes())
}

fn read_store_value_with_root(root: &CapabilityRetentionRoot, path: &LocalStorePath) -> Result<IoValue> {
    let text = root.root().read_to_string(path)?;
    crate::preserves_rail::parse_text(&text)
}

fn bundle_path(path: &str) -> Result<LocalStorePath> {
    LocalStorePath::parse(path)
}

fn bundle_artifact_path(directory: &str, reference: &str) -> Result<LocalStorePath> {
    bundle_path(&format!("artifacts/{directory}/{}.preserves", ref_file_name(reference)?))
}

fn write_bundle_value(root: &CapabilityBundleRoot, path: &LocalStorePath, value: &IoValue) -> Result<()> {
    let text = crate::preserves_rail::to_text(value)?;
    root.root().write(path, text.as_bytes())
}

fn read_bundle_value(root: &CapabilityBundleRoot, path: &LocalStorePath) -> Result<IoValue> {
    let text = root.root().read_to_string(path)?;
    crate::preserves_rail::parse_text(&text)
}

fn object_value(object_ref: &str, object_kind: &str) -> IoValue {
    crate::preserves_rail::record("object", vec![
        crate::preserves_rail::string(object_ref),
        crate::preserves_rail::string(object_kind),
    ])
}

fn parse_object_value(value: &Value<IoValue>) -> Result<(String, String)> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("object", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected object record"))?;
    let object_ref = required_string(&fields[0], "object ref")?;
    require_ref(&object_ref, "object ref")?;
    let object_kind = required_string(&fields[1], "object kind")?;
    validate_name(&object_kind, "object kind")?;
    Ok((object_ref, object_kind))
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |number| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(number)]),
    )
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    let mut checks = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(entry);
        let check_fields = check_value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
        push_bounded(
            &mut checks,
            (required_string(&check_fields[0], "check name")?, required_string(&check_fields[1], "check status")?),
            MAX_RETENTION_REFS,
            "retention checks",
        )?;
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing pass check {name}")))
    }
}

fn live_send_publish_ref(send: &crate::node_daemon::ControlLiveSendReceipt) -> String {
    send.transport_receipt_ref.clone().unwrap_or_else(|| send.receipt_ref.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeLiveTransportReceipt {
    receipt_ref: String,
    operation: String,
    decision: String,
    node_id: String,
    envelope_ref: String,
    ingress_receipt_ref: Option<String>,
    diagnostics: Vec<String>,
}

fn parse_node_live_transport_receipt(value: &IoValue) -> Result<NodeLiveTransportReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(13))
        .or_else(|| value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
        "node control live transport receipt",
    )?;
    require_check(&parse_checks(&fields[10])?, "transport-is-not-authority", "node control live transport")?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    Ok(NodeLiveTransportReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision,
        node_id: record_string(&fields[5], "node")?,
        envelope_ref: record_ref(&fields[7], "envelope")?,
        ingress_receipt_ref: record_optional_ref(&fields[8], "ingress-receipt")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
    })
}

fn node_live_control_diagnostics(
    phase: &str,
    control: &crate::node_runtime::ControlRequest,
    expected_target_ref: &str,
    expected_payload_ref: Option<&str>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if control.operation != "gate" {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-operation:{}", control.operation));
    }
    if control.target_ref.as_deref() != Some(expected_target_ref) {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-target"));
    }
    if control.payload_ref.as_deref() != expected_payload_ref {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-payload"));
    }
    diagnostics
}

fn node_live_send_diagnostics(phase: &str, send: &crate::node_daemon::ControlLiveSendReceipt) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(send.diagnostics.len().saturating_add(2));
    for diagnostic in &send.diagnostics {
        diagnostics.push(format!("remote-clearance-live-{phase}:{diagnostic}"));
    }
    if send.decision != "pass" {
        diagnostics.push(format!("remote-clearance-live-{phase}-send-deny:{}", send.decision));
    }
    if send.transport_receipt_ref.is_none() {
        diagnostics.push(format!("remote-clearance-live-{phase}-missing-transport-receipt"));
    }
    diagnostics
}

fn node_live_transport_diagnostics(phase: &str, value: &IoValue) -> Result<Vec<String>> {
    let receipt = parse_node_live_transport_receipt(value)?;
    node_live_transport_diagnostics_from(phase, &receipt)
}

fn node_live_transport_diagnostics_from(phase: &str, receipt: &NodeLiveTransportReceipt) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in &receipt.diagnostics {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}:{diagnostic}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    if receipt.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}-transport-deny:{}:{}", receipt.operation, receipt.decision),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn node_live_receive_binding_diagnostics(
    phase: &str,
    send: &crate::node_daemon::ControlLiveSendReceipt,
    receive: &NodeLiveTransportReceipt,
    expected_ingress_ref: &str,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if receive.operation != "receive" {
        diagnostics.push(format!("remote-clearance-live-{phase}-not-receive:{}", receive.operation));
    }
    if receive.envelope_ref != send.envelope_ref {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-envelope"));
    }
    if receive.node_id != send.to_node {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-node"));
    }
    if receive.ingress_receipt_ref.as_deref() != Some(expected_ingress_ref) {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-ingress"));
    }
    diagnostics
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}
