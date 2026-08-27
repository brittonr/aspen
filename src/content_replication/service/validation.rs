use super::*;

pub(super) fn require_active(instance: &ServiceInstance) -> Result<()> {
    if instance.state != LifecycleState::Active {
        return Err(MoltenError::invalid_harness("content-replication reconcile requires an active instance"));
    }
    Ok(())
}

pub(super) fn validate_authority(manifest: &Manifest, observation: &AuthorityObservation) -> Result<()> {
    validate_ref(&observation.observation_ref, "replication authority observation")?;
    if !observation.admitted
        || observation.authority_ref != manifest.authority_ref
        || observation.service_id != manifest.service_id
        || observation.generation != manifest.generation
    {
        return Err(MoltenError::invalid_harness("content-replication authority observation denied or drifted"));
    }
    Ok(())
}

pub(super) fn validate_identity(manifest: &Manifest, observation: &IdentityObservation) -> Result<()> {
    validate_ref(&observation.observation_ref, "replication identity observation")?;
    if !observation.current
        || observation.identity_ref != manifest.identity_ref
        || observation.service_id != manifest.service_id
        || observation.generation != manifest.generation
    {
        return Err(MoltenError::invalid_harness("content-replication identity observation is stale or mismatched"));
    }
    Ok(())
}

pub(super) fn validate_membership(manifest: &Manifest, observation: &MembershipObservation) -> Result<()> {
    validate_ref(&observation.observation_ref, "replication membership observation")?;
    if !observation.current || observation.membership_epoch != manifest.membership_epoch {
        return Err(MoltenError::invalid_harness("content-replication membership observation is stale"));
    }
    Ok(())
}

pub(super) fn validate_placement(manifest: &Manifest, observation: &PlacementObservation) -> Result<()> {
    validate_ref(&observation.observation_ref, "replication placement observation")?;
    if !observation.current
        || observation.membership_epoch != manifest.membership_epoch
        || observation.placement_epoch != manifest.placement_epoch
    {
        return Err(MoltenError::invalid_harness("content-replication placement observation is stale"));
    }
    Ok(())
}

pub(super) fn validate_resources(manifest: &Manifest, plan: &Plan, observation: &ResourceObservation) -> Result<()> {
    validate_ref(&observation.reservation_ref, "replication resource reservation")?;
    if !observation.admitted || observation.plan_ref != plan.plan_ref || observation.generation != manifest.generation {
        return Err(MoltenError::invalid_harness("content-replication resource reservation denied or drifted"));
    }
    Ok(())
}

pub(super) fn validate_pin(manifest: &Manifest, action: &Action, observation: &PinObservation) -> Result<()> {
    validate_ref(&observation.pin_ref, "replication retention pin")?;
    if !observation.admitted
        || observation.operation_id != action.operation_id
        || observation.content_ref != action.content_ref
        || observation.generation != manifest.generation
    {
        return Err(MoltenError::invalid_harness("content-replication retention pin denied or drifted"));
    }
    Ok(())
}

pub(super) fn validate_cleanup(manifest: &Manifest, action: &Action, observation: &CleanupObservation) -> Result<()> {
    validate_ref(&observation.cleanup_ref, "replication cleanup admission")?;
    if !observation.admitted
        || observation.operation_id != action.operation_id
        || observation.content_ref != action.content_ref
        || observation.generation != manifest.generation
    {
        return Err(MoltenError::invalid_harness("content-replication cleanup admission denied or drifted"));
    }
    Ok(())
}

pub(super) fn validate_envelope(manifest: &Manifest, action: &Action, envelope: &TransferEnvelope) -> Result<()> {
    validate_ref(&envelope.transfer_ref, "replication transfer observation")?;
    validate_ref(&envelope.transport_verification_ref, "replication transport verification")?;
    let manifest_ref = manifest
        .contents
        .iter()
        .find(|content| content.content_ref == action.content_ref)
        .map(|content| content.manifest_ref.as_str());
    if envelope.operation_id != action.operation_id
        || envelope.content_ref != action.content_ref
        || Some(envelope.manifest_ref.as_str()) != manifest_ref
        || envelope.source_peer.as_str() != action.source_peer.as_deref().unwrap_or("")
        || envelope.target_peer != action.target_peer
        || envelope.generation != manifest.generation
        || envelope.membership_epoch != manifest.membership_epoch
        || envelope.placement_epoch != manifest.placement_epoch
        || envelope.encoded_bytes != action.encoded_bytes
        || envelope.protected != action.preserve_protected_form
    {
        return Err(MoltenError::invalid_harness("content-replication transfer envelope is stale or mismatched"));
    }
    Ok(())
}

pub(super) fn validate_verification(
    manifest: &Manifest,
    action: &Action,
    observation: &VerificationObservation,
) -> Result<()> {
    validate_ref(&observation.verification_ref, "replication verification observation")?;
    let replica = &observation.replica;
    if !observation.identity_verified
        || !observation.authorization_admitted
        || observation.operation_id != action.operation_id
        || replica.content_ref != action.content_ref
        || replica.peer_id != action.target_peer
        || replica.generation != manifest.generation
        || replica.membership_epoch != manifest.membership_epoch
        || replica.placement_epoch != manifest.placement_epoch
        || !replica.present
        || !replica.identity_verified
        || replica.protected != action.preserve_protected_form
    {
        return Err(MoltenError::invalid_harness("content-replication verification is incomplete or stale"));
    }
    Ok(())
}

pub(super) fn validate_ref(reference: &str, field: &str) -> Result<()> {
    if !valid_ref(reference) {
        return Err(MoltenError::invalid_harness(format!("{field} is not a canonical BLAKE3 reference")));
    }
    Ok(())
}
