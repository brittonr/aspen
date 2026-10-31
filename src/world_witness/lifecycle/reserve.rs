pub(crate) fn acquire<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &mut crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    stage: &super::durable::Stage,
) -> crate::error::Result<Option<crate::world_witness::provider::Reservation>> {
    if let Some(bytes) = store.reservation(&stage.operation_ref)? {
        return serde_json::from_slice(&bytes).map(Some).map_err(crate::world_witness::store::failure);
    }
    let minimum = match &stage.staged.retained {
        molten_core::world_witness::RetainedWitnessState::Present(highest) => highest.checkpoint.revision,
        _ => 0,
    };
    // The fresh signed query, not the head generation, selects the external ordinal.
    let observation = provider.query(minimum).map_err(crate::world_witness::store::failure)?;
    let claim = crate::world_witness::provider::ordinal::plan(&observation.snapshot, &candidate(stage)?)
        .map_err(crate::world_witness::store::failure)?;
    let result = provider.reserve(observation.snapshot.revision, claim);
    let reservation = match result {
        Ok(reservation) => reservation,
        Err(error) => {
            if let Some(reservation) = &error.reservation {
                let bytes = serde_json::to_vec(reservation).map_err(crate::world_witness::store::failure)?;
                // An unrecorded partial reservation leaves the outcome uncertain, never denied.
                if store.save_reservation(&stage.operation_ref, &bytes).is_err() {
                    return Ok(None);
                }
            }
            return if error.kind == crate::world_witness::provider::ErrorKind::Uncertain {
                Ok(None)
            } else {
                Err(crate::world_witness::store::failure(error))
            };
        }
    };
    let bytes = serde_json::to_vec(&reservation).map_err(crate::world_witness::store::failure)?;
    if store.save_reservation(&stage.operation_ref, &bytes).is_err() {
        return Ok(None);
    }
    Ok(Some(reservation))
}

pub(crate) fn candidate(
    stage: &super::durable::Stage,
) -> crate::error::Result<crate::world_witness::provider::ordinal::Candidate> {
    let predecessor = match &stage.staged.retained {
        molten_core::world_witness::RetainedWitnessState::Present(highest) => Some(
            crate::world_witness::provider::normalize::claim_digest(&highest.subject.claim_ref)
                .map_err(crate::world_witness::store::failure)?,
        ),
        _ => None,
    };
    Ok(crate::world_witness::provider::ordinal::Candidate {
        digest: crate::world_witness::provider::normalize::claim_digest(&stage.staged.transition.claim_ref)
            .map_err(crate::world_witness::store::failure)?,
        predecessor,
    })
}
