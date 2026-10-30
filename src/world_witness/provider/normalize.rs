pub struct Input<'a> {
    pub contract: &'a molten_core::world_witness::ProviderContract,
    pub subject: &'a molten_core::world_witness::WitnessSubject,
    pub retained: &'a molten_core::world_witness::RetainedWitnessState,
    pub observed_at: u64,
    pub operation_id: &'a str,
    pub reservation: &'a super::types::Reservation,
}

/// This index signs exact state bindings, not a Merkle inclusion or global fork proof.
impl<T: super::Transport, P: super::Ports> super::Index<T, P> {
    pub fn normalize(
        &self,
        observation: &super::types::Observation,
        input: Input<'_>,
    ) -> Result<molten_core::world_witness::WitnessObservation, super::types::Error> {
        if input.contract != &self.contract {
            return Err(super::types::Error::invalid(
                "observation contract differs from independently installed provider trust",
            ));
        }
        let snapshot = release_channel_index::client::verify_reply(
            &self.provider_key,
            &observation.request,
            &observation.reply,
            0,
        )
        .map_err(super::types::Error::invalid)?;
        self.bind(input.reservation)?;
        if snapshot.highest.as_ref() != Some(&input.reservation.claim) {
            return Err(super::types::Error::invalid("reservation claim substitution"));
        }
        if snapshot.pending.is_some() {
            self.require_reserved(input.reservation, observation)?;
        }
        if observation.request.namespace != self.namespace
            || snapshot != observation.snapshot
            || self.namespace.track != input.subject.branch_id.as_str()
        {
            return Err(super::types::Error::invalid("observation namespace/branch substitution"));
        }
        let candidate =
            snapshot.highest.as_ref().ok_or_else(|| super::types::Error::invalid("missing external highest"))?;
        if candidate.digest != claim_digest(&input.subject.claim_ref)? {
            return Err(super::types::Error::invalid("observation claim substitution"));
        }
        let phase = self.phase(&snapshot, candidate)?;
        let checkpoint_id = super::projection::checkpoint_id(&self.provider_key, &self.namespace, &snapshot)?;
        if let molten_core::world_witness::RetainedWitnessState::Present(highest) = input.retained
            && snapshot.revision == highest.checkpoint.revision
            && checkpoint_id != highest.checkpoint.checkpoint_id
        {
            return Ok(molten_core::world_witness::WitnessObservation::Fork {
                provider_id: input.contract.provider_id.clone(),
                checkpoint_ids: std::collections::BTreeSet::from([
                    highest.checkpoint.checkpoint_id.clone(),
                    checkpoint_id,
                ]),
            });
        }
        let predecessor = consistency(input.retained, input.subject, candidate, snapshot.revision)?;
        let checkpoint = molten_core::world_witness::SignedCheckpoint {
            provider_id: input.contract.provider_id.clone(),
            checkpoint_id: checkpoint_id.clone(),
            revision: snapshot.revision,
            signature_verified: true,
            observed_at: input.observed_at,
        };
        let subject = input.subject.clone();
        let operation_id = input.operation_id.to_owned();
        Ok(molten_core::world_witness::WitnessObservation::Available(Box::new(
            molten_core::world_witness::AvailableWitnessObservation {
                contract: input.contract.clone(),
                checkpoint,
                append: molten_core::world_witness::AppendObservation {
                    operation_id,
                    subject: subject.clone(),
                    accepted: true,
                },
                inclusion: molten_core::world_witness::InclusionObservation {
                    subject: subject.clone(),
                    checkpoint_id: checkpoint_id.clone(),
                    verified: true,
                },
                consistency: molten_core::world_witness::ConsistencyObservation {
                    predecessor,
                    checkpoint_id: checkpoint_id.clone(),
                    verified: true,
                },
                quorum: molten_core::world_witness::QuorumObservation {
                    checkpoint_id: checkpoint_id.clone(),
                    witnesses: std::collections::BTreeSet::from([input.contract.provider_id.clone()]),
                    verified: true,
                },
                current_head: molten_core::world_witness::CurrentHeadObservation {
                    subject,
                    checkpoint_id,
                    verified: true,
                    phase,
                },
            },
        )))
    }

    fn phase(
        &self,
        snapshot: &release_channel_index::model::Snapshot,
        candidate: &release_channel_index::model::Claim,
    ) -> Result<molten_core::world_witness::CurrentHeadPhase, super::types::Error> {
        if release_channel_index::client::require_current(snapshot, candidate).is_ok() {
            return Ok(molten_core::world_witness::CurrentHeadPhase::Committed);
        }
        if snapshot.pending.as_ref().is_some_and(|pending| {
            pending.claim == *candidate && pending.status == release_channel_index::model::PendingStatus::Reserved
        }) {
            return Ok(molten_core::world_witness::CurrentHeadPhase::Reserved);
        }
        Err(super::types::Error::invalid("orphan or high-water gap is not current"))
    }
}

fn consistency(
    retained: &molten_core::world_witness::RetainedWitnessState,
    subject: &molten_core::world_witness::WitnessSubject,
    candidate: &release_channel_index::model::Claim,
    revision: u64,
) -> Result<Option<String>, super::types::Error> {
    match retained {
        molten_core::world_witness::RetainedWitnessState::Empty if candidate.predecessor.is_none() => Ok(None),
        molten_core::world_witness::RetainedWitnessState::Present(highest) => {
            let digest = claim_digest(&highest.subject.claim_ref)?;
            let is_same = candidate.digest == digest && subject.generation == highest.subject.generation;
            let is_advance = candidate.predecessor == Some(digest) && subject.generation > highest.subject.generation;
            if revision < highest.checkpoint.revision || (!is_same && !is_advance) {
                return Err(super::types::Error::invalid("retained witness consistency failure"));
            }
            Ok(Some(highest.checkpoint.checkpoint_id.clone()))
        }
        _ => Err(super::types::Error::invalid("missing retained witness state or invalid enrollment predecessor")),
    }
}

pub fn claim_digest(reference: &molten_core::world_head::WorldHeadClaimRef) -> Result<[u8; 32], super::types::Error> {
    let text = reference
        .as_str()
        .strip_prefix("blake3:")
        .ok_or_else(|| super::types::Error::invalid("unsupported world claim digest"))?;
    let mut digest = [0; 32];
    if text.len() != 64 {
        return Err(super::types::Error::invalid("invalid world claim digest length"));
    }
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|error| super::types::Error::invalid(error.to_string()))?;
    }
    Ok(digest)
}
