struct Committed<'a> {
    reference: String,
    value: preserves::IOValue,
    index: u64,
    record: &'a crate::fabric_consistency::raft::ReplicaRecord,
}
pub(super) fn complete(
    runtime: &super::Driver,
    command: &crate::fabric_consistency::raft::Command,
    effect: &crate::system_extension::TypedEffectRequest,
) -> crate::error::Result<Option<crate::system_extension::PortEffectOutput>> {
    let application = runtime.application();
    application.store.ensure_serving()?;
    let replica = runtime.service.state();
    let Some(index) = replica.completed_requests.get(&command.request_ref) else {
        return Ok(None);
    };
    if replica.commit_index < *index || replica.last_applied < *index {
        return Ok(None);
    }
    let Some(record) = runtime.service.evidence().records().iter().rev().find(|record| {
        record.kind == crate::fabric_consistency::raft::ReplicaKind::Commit
            && record.index >= *index
            && record.quorum_evidence_ref.is_some()
            && record.quorum_members.len() >= 2
    }) else {
        return Ok(None);
    };
    let reference = application.store.result(&command.request_ref)?;
    let value = application.store.receipt(&reference)?;
    validate_request(&value, &command.request_ref)?;
    let committed = Committed {
        reference,
        value,
        index: *index,
        record,
    };
    let value = completion_value(runtime, command, effect, committed)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok(Some(crate::system_extension::PortEffectOutput {
        output_schema_ref: super::super::COMMIT_OUTPUT_SCHEMA.to_string(),
        output_ref: reference.clone(),
        materialized_output: Some(crate::system_extension::NativeCallbackValue {
            value_ref: reference,
            bytes,
        }),
    }))
}
fn completion_value(
    runtime: &super::Driver,
    command: &crate::fabric_consistency::raft::Command,
    effect: &crate::system_extension::TypedEffectRequest,
    committed: Committed<'_>,
) -> crate::error::Result<preserves::IOValue> {
    let position = position_value(runtime.service.state(), committed.index, committed.record)?;
    Ok(crate::preserves_rail::record("collaboration-committed-command-v1", vec![
        crate::preserves_rail::string(&effect.request_ref),
        crate::preserves_rail::string(&command.command_ref),
        crate::preserves_rail::string(&command.request_ref),
        crate::preserves_rail::string(committed.reference),
        committed.value,
        crate::preserves_rail::string(runtime.application().state_ref().as_str()),
        position,
        crate::preserves_rail::string("committed-outcome-not-authority-or-exactly-once-or-production-proof"),
    ]))
}
fn position_value(
    replica: &crate::fabric_consistency::raft::ReplicaState,
    index: u64,
    record: &crate::fabric_consistency::raft::ReplicaRecord,
) -> crate::error::Result<preserves::IOValue> {
    let quorum = record
        .quorum_evidence_ref
        .as_deref()
        .ok_or_else(|| super::super::denied(super::super::Denial::Partition))?;
    Ok(crate::preserves_rail::record("collaboration-live-commit-position-v1", vec![
        crate::preserves_rail::string(&replica.profile.group_binding_ref),
        crate::preserves_rail::u64_value(replica.profile.service_generation),
        crate::preserves_rail::u64_value(index),
        crate::preserves_rail::u64_value(replica.commit_index),
        crate::preserves_rail::u64_value(replica.last_applied),
        crate::preserves_rail::string(&record.evidence_ref),
        crate::preserves_rail::string(quorum),
    ]))
}
fn validate_request(receipt: &preserves::IOValue, request: &str) -> crate::error::Result<()> {
    let fields = super::super::decode::fields(receipt, "collaboration-commit-receipt-v1", 7)?;
    let requests = super::super::decode::items(&fields[4], super::super::MAX_OPERATIONS)?;
    if !requests.iter().any(|value| value.as_string().is_some_and(|value| value == request)) {
        return Err(super::super::denied(super::super::Denial::Domain));
    }
    let outcomes = super::super::decode::items(&fields[6], super::super::MAX_OPERATIONS)?;
    let mut matches = 0;
    for outcome in outcomes {
        let fields = super::super::decode::fields(&outcome, "collaboration-command-outcome-v1", 2)?;
        if super::super::decode::string(&fields[0])? == request {
            matches += 1;
        }
    }
    if matches != 1 {
        return Err(super::super::denied(super::super::Denial::Domain));
    }
    Ok(())
}
