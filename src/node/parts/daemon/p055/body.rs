
fn control_service_heartbeat_path(heartbeat_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.service-heartbeat.preserves", ref_file_stem(heartbeat_ref)),
    )
}

fn control_service_run_receipt_path(service_run_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.service-run-receipt.preserves", ref_file_stem(service_run_ref)),
    )
}

fn control_supervisor_receipt_path(receipt_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.supervisor-receipt.preserves", ref_file_stem(receipt_ref)),
    )
}

fn write_supervisor_receipt(
    root: &crate::node_state::NodeStateRoot,
    input: &SupervisorReceiptValueInput<'_>,
) -> Result<String> {
    let value = supervisor_receipt_value(input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    write_preserves(root, &control_supervisor_receipt_path(&receipt_ref)?, &value)?;
    import_artifact(root, &value)?;
    Ok(receipt_ref)
}

fn control_ingress_envelope_path(topic: &str, envelope_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(CONTROL_INGRESS_DIR)?
        .join_segment(topic)?
        .join_segment(&format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
}

fn write_ingress_envelope_and_verify(
    root: &crate::node_state::NodeStateRoot,
    topic: &str,
    envelope: &ControlIngressEnvelope,
) -> Result<()> {
    let path = control_ingress_envelope_path(topic, &envelope.envelope_ref)?;
    write_preserves(root, &path, &envelope.value)?;
    let read_value = read_preserves(root, &path)?;
    let read_envelope = parse_control_ingress_envelope(&read_value)?;
    if read_envelope.envelope_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match written {}",
            read_envelope.envelope_ref, envelope.envelope_ref
        )));
    }
    Ok(())
}

fn control_ingress_receipt_path(envelope_ref: &str, phase: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(CONTROL_INGRESS_DIR)?
        .join("receipts")?
        .join_segment(&format!("{}.{}.receipt.preserves", ref_file_stem(envelope_ref), phase))
}
