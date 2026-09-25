
fn read_quorum_evidence(
    before: &ReplicaState,
    event: &ReplicaEvent,
    source_ref: String,
) -> crate::error::Result<Option<(u64, ValidatedReplicaQuorumEvidence)>> {
    let ReplicaEvent::Message { envelope } = event else {
        return Ok(None);
    };
    let RaftMessage::ReadAcknowledgement {
        term,
        follower_id,
        request_ref,
        ..
    } = &envelope.message
    else {
        return Ok(None);
    };
    let pending = before
        .pending_reads
        .get(request_ref)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("read-currentness evidence lost its pending read"))?;
    let mut acknowledgement_members = pending.acknowledgements.iter().cloned().collect::<Vec<_>>();
    acknowledgement_members.push(follower_id.clone());
    let validated = validate_replica_quorum_evidence(&ReplicaQuorumEvidence {
        boundary: ReplicaQuorumEvidenceBoundary::ReadCurrentness,
        group_binding_ref: before.profile.group_binding_ref.clone(),
        membership_ref: before.membership.membership_ref.clone(),
        config_epoch: before.membership.config_epoch,
        term: *term,
        index: pending.required_index,
        admitted_voters: before.membership.voters.clone(),
        acknowledgement_members,
        source_ref,
    })?;
    Ok(Some((pending.required_index, validated)))
}
