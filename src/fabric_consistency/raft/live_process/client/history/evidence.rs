use super::*;

pub(super) fn is_checkpoint_bound(
    fields: &preserves::Record<preserves::Value<preserves::IOValue>>,
    input_ref: &str,
    participant: &str,
    receipt_ref: &str,
    sample_request: &str,
    active: &Observation,
) -> crate::error::Result<bool> {
    let request = crate::fabric_consistency::raft::tests::test_ref(REQUEST_LABEL);
    let is_bound = canonical::required_string(&fields[0], "sample input")? == input_ref
        && canonical::required_string(&fields[1], "sample participant")? == participant
        && canonical::required_string(&fields[4], "owner receipt")? == receipt_ref
        && sample_request == request
        && active.node_id == participant
        && active
            .admitted_voters
            .iter()
            .map(String::as_str)
            .eq(crate::fabric_simulation::bounded_client::CLIENT_PARTICIPANTS)
        && active.commit_index == INITIAL_LOG_INDEX
        && active.last_applied == INITIAL_LOG_INDEX
        && active.quorum_loss_request_uncommitted;
    // Validate canonical quorum evidence and distinct admitted membership,
    // never a service-supplied boolean claiming a quorum was observed.
    let is_quorum_bound = participant != crate::fabric_simulation::bounded_client::CLIENT_PARTICIPANTS[0]
        || validate_commit_quorum(active).is_ok();
    Ok(is_bound && is_quorum_bound)
}

pub(super) fn is_terminal_bound(terminal: &Observation, active: &Observation, participant: &str) -> bool {
    terminal.node_id == participant
        && terminal.process_id != active.process_id
        && terminal.endpoint_identity == active.endpoint_identity
        && terminal.commit_index == active.commit_index
        && terminal.last_applied == active.last_applied
        && terminal.clean_shutdown
        && terminal.quorum_loss_request_uncommitted
}

pub(super) fn local_recovery(
    directory: &std::path::Path,
    participant: &str,
    input_ref: &str,
) -> crate::error::Result<(crate::fabric_simulation::bounded_client::RecoverySource, u64)> {
    let sample = read_value(&directory.join(format!("{participant}-client-reopen.preserves")))?;
    let fields = crate::preserves_rail::simple_record_fields(&sample, "molten-client-native-sample-v1", 10)?;
    let owner = read_value(&directory.join(format!("{participant}-client-reopen-owner.preserves")))?;
    let observed = parse(&owner)?;
    let reference = crate::preserves_rail::canonical_hash(&owner)?;
    if canonical::required_string(&fields[0], "reopen input")? != input_ref
        || canonical::required_string(&fields[1], "reopen participant")? != participant
        || canonical::required_string(&fields[4], "reopen owner")? != reference
        || observed.node_id != participant
        || canonical::required_string(&fields[9], "recovery lineage")? != observed.recovery_ref
    {
        return Err(crate::error::Failure::invalid_harness("reopen receipt lineage mismatch"));
    }
    crate::preserves_rail::validate_content_ref(&observed.recovery_ref)?;
    let recovery = match canonical::required_string(&fields[8], "recovery source")?.as_str() {
        "local"
            if observed.request_completed
                && observed.commit_index == INITIAL_LOG_INDEX
                && (observed.application_applied || observed.application_restored)
                && (observed.durable_record_count > 0 || observed.durable_snapshot_count > 0) =>
        {
            crate::fabric_simulation::bounded_client::RecoverySource::Local(reference)
        }
        "absent" if observed.durable_record_count == 0 && observed.durable_snapshot_count == 0 => {
            crate::fabric_simulation::bounded_client::RecoverySource::Absent(reference)
        }
        "unavailable" => crate::fabric_simulation::bounded_client::RecoverySource::Unavailable,
        _ => return Err(crate::error::Failure::invalid_harness("unproven native recovery source")),
    };
    Ok((recovery, observed.term))
}

pub(super) fn reached(
    input: &crate::fabric_simulation::bounded_client::Input,
    directory: &std::path::Path,
) -> crate::error::Result<bool> {
    let path = directory.join("client-mutant-reached.preserves");
    if !path.try_exists().map_err(crate::error::Failure::from)? {
        return Ok(false);
    }
    let value = read_value(&path)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "molten-client-mutant-reached-v1", 4)?;
    if canonical::required_string(&fields[0], "mutant input")?
        != crate::fabric_simulation::bounded_client::input_ref(input)?
        || canonical::required_string(&fields[1], "target property")?
            != crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY
        || crate::fabric_simulation::bounded_client::unsigned_integer(&fields[3])? <= 1
    {
        return Err(crate::error::Failure::invalid_harness("semantic mutant attribution mismatch"));
    }
    crate::preserves_rail::validate_content_ref(&canonical::required_string(&fields[2], "handler receipt")?)?;
    Ok(true)
}

pub(super) fn parse_health(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::fabric_durability::AdapterHealth> {
    let reason = match canonical::required_string(value, "adapter health")?.as_str() {
        "serving" => return Ok(crate::fabric_durability::AdapterHealth::Serving),
        "outcome-unknown" => crate::fabric_durability::QuarantineReason::OutcomeUnknown,
        "corrupt-or-inconsistent" => crate::fabric_durability::QuarantineReason::CorruptOrInconsistent,
        "repaired-on-open" => crate::fabric_durability::QuarantineReason::RepairedOnOpen,
        "reconciliation-mismatch" => crate::fabric_durability::QuarantineReason::ReconciliationMismatch,
        "reference-unavailable" => crate::fabric_durability::QuarantineReason::ReferenceUnavailable,
        _ => return Err(crate::error::Failure::invalid_harness("unsupported adapter health observation")),
    };
    Ok(crate::fabric_durability::AdapterHealth::Quarantined(reason))
}

pub(super) fn terminal_health(
    directory: &std::path::Path,
    participant: &str,
    input_ref: &str,
    terminal: &Observation,
    previous: crate::fabric_durability::AdapterHealth,
) -> crate::error::Result<crate::fabric_durability::AdapterHealth> {
    let sample = read_value(&directory.join(format!("{participant}-client-terminal.preserves")))?;
    let fields = crate::preserves_rail::simple_record_fields(&sample, "molten-client-native-sample-v1", 10)?;
    let owner = read_value(&directory.join(format!("{participant}-client-terminal-owner.preserves")))?;
    let mut expected = terminal.clone();
    expected.clean_shutdown = false;
    if parse(&owner)? != expected
        || canonical::required_string(&fields[0], "terminal input")? != input_ref
        || canonical::required_string(&fields[1], "terminal participant")? != participant
        || canonical::required_string(&fields[4], "terminal owner")? != crate::preserves_rail::canonical_hash(&owner)?
    {
        return Err(crate::error::Failure::invalid_harness("terminal health receipt mismatch"));
    }
    let observed = parse_health(&fields[5])?;
    // No observed reconciliation is synthesized just because this sample serves.
    Ok(match previous {
        crate::fabric_durability::AdapterHealth::Serving => observed,
        quarantined => quarantined,
    })
}

pub(super) fn validate_owner_cohort(
    records: &[(crate::fabric_simulation::bounded_client::Participant, Observation)],
) -> crate::error::Result<()> {
    let expected_group = crate::fabric_consistency::raft::tests::active_group().binding_ref;
    let mut process_ids = std::collections::BTreeSet::new();
    let mut endpoint_ids = std::collections::BTreeSet::new();
    for (_participant, owner) in records {
        if owner.group_binding_ref != expected_group
            || !process_ids.insert(owner.process_id)
            || !endpoint_ids.insert(&owner.endpoint_identity)
        {
            return Err(crate::error::Failure::invalid_harness("participant process or group identity mismatch"));
        }
    }
    if records.len() == STATIC_VOTER_COUNT {
        let leader = &records[0].1;
        for (_participant, owner) in &records[1..] {
            if owner.membership_ref != leader.membership_ref
                || owner.config_epoch != leader.config_epoch
                || owner.commit_index != leader.commit_index
                || owner.last_applied != leader.last_applied
            {
                return Err(crate::error::Failure::invalid_harness("cohort committed boundary mismatch"));
            }
        }
        let snapshot_peer = &records[2].1;
        if snapshot_peer.snapshot_ref.is_empty() || snapshot_peer.snapshot_ref != leader.snapshot_ref {
            return Err(crate::error::Failure::invalid_harness("snapshot peer lineage mismatch"));
        }
    }
    Ok(())
}
