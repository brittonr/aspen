//! Receipt summaries for supervisors, control, and current runs.

pub(super) fn supervisor_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Ok(policy) = super::supervision::parse_control_supervisor_policy(value) {
        return Ok(Some(format!(
            "node control supervisor policy ref={} restarts={} stale_lock_recovery={}",
            policy.policy_ref, policy.max_restarts, policy.stale_lock_recovery
        )));
    }
    if let Ok(receipt) = super::supervision::parse_control_supervisor_receipt(value) {
        return Ok(Some(format!(
            "node control supervisor decision={} operation={} policy={}",
            receipt.decision,
            receipt.operation,
            receipt.supervisor_policy_ref.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[7], "ticks")?,
            super::fields::record_sequence_len(&fields[8], "heartbeats")?,
            super::fields::record_string(&fields[12], "stopped")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[7], "ticks")?,
            super::fields::record_sequence_len(&fields[8], "heartbeats")?,
            super::fields::record_string(&fields[12], "stopped")?
        )));
    }
    Ok(None)
}

pub(super) fn control_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-queue-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control queue decision={} phase={} request={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "phase")?,
            super::fields::record_string(&fields[4], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-operation-receipt-v1", Some(8)) {
        return Ok(Some(format!(
            "node control operation decision={} operation={} request={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "operation")?,
            super::fields::record_string(&fields[3], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control heartbeat decision={} startup={} processed={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_string(&fields[5], "processed-count")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-loop-receipt-v1", Some(11)) {
        return Ok(Some(format!(
            "node control loop decision={} startup={} processed={} stopped={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_sequence_len(&fields[5], "processed-requests")?,
            super::fields::record_string(&fields[7], "stopped")?
        )));
    }
    Ok(None)
}

pub(super) fn run_state_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-lock-v2", Some(7)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA_V2,
            "node control lock",
        )?;
        return Ok(Some(format!(
            "node control lock startup={} owner={} run={}",
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_string(&fields[3], "owner")?,
            super::fields::record_string(&fields[6], "run")?
        )));
    }
    if let Ok(binding) = crate::node_runtime::parse_node_run_binding(value) {
        return Ok(Some(format!(
            "node control run binding run={} startup={}",
            binding.run_ref, binding.startup_receipt_ref
        )));
    }
    if let Ok(observation) = crate::node_runtime::parse_node_run_shutdown_observation(value) {
        return Ok(Some(format!(
            "node control run shutdown observation run={} startup={} shutdown={}",
            observation.run_ref, observation.startup_receipt_ref, observation.shutdown_receipt_ref
        )));
    }
    Ok(None)
}
