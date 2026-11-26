//! Supervisor receipt and ingress envelope writes.

pub(super) fn write_supervisor_receipt(
    root: &crate::node_state::DirectoryRoot,
    input: &super::services::SupervisorReceiptValueInput<'_>,
) -> crate::error::Result<String> {
    let value = super::services::supervisor_receipt_value(input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    super::storage::write_preserves(root, &super::paths::control_supervisor_receipt_path(&receipt_ref)?, &value)?;
    super::storage::import_artifact(root, &value)?;
    Ok(receipt_ref)
}

pub(super) fn write_ingress_envelope_and_verify(
    root: &crate::node_state::DirectoryRoot,
    topic: &str,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<()> {
    let path = super::paths::control_ingress_envelope_path(topic, &envelope.envelope_ref)?;
    super::storage::write_preserves(root, &path, &envelope.value)?;
    let read_value = super::storage::read_preserves(root, &path)?;
    let read_envelope = super::delivery::parse_control_ingress_envelope(&read_value)?;
    if read_envelope.envelope_ref != envelope.envelope_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match written {}",
            read_envelope.envelope_ref, envelope.envelope_ref
        )));
    }
    Ok(())
}
