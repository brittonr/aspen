pub struct ReadInput<'a> {
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
    pub input: molten_core::world_witness::CurrentnessInput,
}

/// Repeated reads query the authenticated external high-water index. A locally
/// enrolled checkpoint is not an independent freshness observation.
pub fn observe_current_head<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    mut read: ReadInput<'_>,
) -> crate::error::Result<molten_core::world_witness::CurrentnessAdmission> {
    let canonical = crate::world_head::canonical_world_head_claim(&read.claim.claim)?;
    if canonical.bytes != read.claim.bytes || canonical.claim_ref != read.claim.claim_ref {
        return Err(crate::world_witness::store::failure("current head claim is not canonical"));
    }
    let current = store
        .current(&read.claim.claim.branch_id)?
        .ok_or_else(|| crate::world_witness::store::failure("current head is missing"))?;
    if current.head != read.claim.claim.successor_head
        || current.generation != read.claim.claim.successor_generation
        || current.policy_ref != read.claim.claim.policy_ref
    {
        return Err(crate::world_witness::store::failure("claim is not the current local head"));
    }
    read.input.retained = store.retained(&current.branch_id)?;
    let molten_core::world_witness::RetainedWitnessState::Present(highest) = &read.input.retained else {
        return Err(crate::world_witness::store::failure("independent current read requires retained highest state"));
    };
    if highest.subject.claim_ref != read.claim.claim_ref || highest.subject.branch_id != current.branch_id {
        return Err(crate::world_witness::store::failure("retained highest state substitutes claim or branch"));
    }
    let bytes = store
        .reservation(&read.input.operation_id)?
        .ok_or_else(|| crate::world_witness::store::failure("current head lacks exact durable external reservation"))?;
    let reservation: crate::world_witness::provider::Reservation =
        serde_json::from_slice(&bytes).map_err(crate::world_witness::store::failure)?;
    let observation = provider
        .observe_current(&reservation.claim, highest.checkpoint.revision)
        .map_err(crate::world_witness::store::failure)?;
    read.input.subject = molten_core::world_witness::WitnessSubject {
        branch_id: current.branch_id,
        claim_ref: canonical.claim_ref,
        generation: current.generation,
    };
    read.input.now = provider.observed_at()?;
    read.input.local_generation = current.generation;
    read.input.observation = Some(
        provider
            .normalize(&observation, crate::world_witness::provider::normalize::Input {
                contract: &read.input.policy.provider,
                subject: &read.input.subject,
                retained: &read.input.retained,
                observed_at: read.input.now,
                operation_id: &read.input.operation_id,
                reservation: &reservation,
            })
            .map_err(crate::world_witness::store::failure)?,
    );
    molten_core::world_witness::validate_committed_currentness(&read.input)
        .map_err(|issue| crate::world_witness::store::failure(format!("current head witness denied: {issue:?}")))
}
