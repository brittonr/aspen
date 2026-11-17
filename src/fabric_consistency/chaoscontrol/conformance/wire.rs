//! Lossless, source-bound encoding of supplied Molten committed-apply facts.
//! Validation identifies data; neither an application receipt reference nor this
//! encoder attests that a guest executed the referenced application path.

/// A canonical producer chain with fixed-width digest references fits below
/// this bound, including all u64 decimal fields. Extra JSON whitespace is not
/// an unbounded transport channel.
pub const MAX_INLINE_PROJECTION_BYTES: u64 = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Issue {
    WorkloadProfile,
    ProtocolProfile,
    Reference,
    Group,
    Participant,
    Generation,
    SourceSequence,
    CommandBounds,
    Transition,
    Envelope,
    MissingInline,
    ProjectionBounds,
    ProjectionEncoding,
    ProjectionMismatch,
    LossAccounting,
    CanonicalBinding,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::WorkloadProfile => "workload-profile",
            Self::ProtocolProfile => "protocol-profile",
            Self::Reference => "reference",
            Self::Group => "group",
            Self::Participant => "participant",
            Self::Generation => "generation",
            Self::SourceSequence => "source-sequence",
            Self::CommandBounds => "command-bounds",
            Self::Transition => "transition",
            Self::Envelope => "envelope",
            Self::MissingInline => "missing-inline",
            Self::ProjectionBounds => "projection-bounds",
            Self::ProjectionEncoding => "projection-encoding",
            Self::ProjectionMismatch => "projection-mismatch",
            Self::LossAccounting => "loss-accounting",
            Self::CanonicalBinding => "canonical-binding",
        })
    }
}

impl std::error::Error for Issue {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionContext {
    pub group_ref: String,
    pub source_sequence: u64,
    pub boundary_ref: String,
    pub scheduler_ref: String,
    pub transition_class_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyBinding {
    pub group_ref: String,
    pub participant_ref: String,
    pub operation_ref: String,
    pub command_ref: String,
    pub command_bytes: Vec<u8>,
    pub lifecycle_generation: u64,
    pub application_receipt_ref: String,
    pub chain: chaoscontrol_smr::ChainObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundObservation {
    pub envelope: chaoscontrol_smr::observation::ProtocolObservation,
    pub apply: ApplyBinding,
}

pub fn project(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    context: &ProjectionContext,
    observation: &super::super::ChaosControlChainObservation,
) -> Result<BoundObservation, Issue> {
    let (protocol_ref, participant) = checks::projection(protocol, plan, context, observation)?;
    let chain = chaoscontrol_smr::ChainObservation {
        event_sequence: context.source_sequence,
        profile_ref: plan.profile_ref.clone(),
        replica_id: observation.replica_ref[chaoscontrol_smr::DIGEST_PREFIX.len()..].to_owned(),
        command_index: observation.command_index,
        prior_digest: observation.prior_digest.clone(),
        digest: observation.next_digest.clone(),
        application_state_ref: observation.application_state_ref.clone(),
    };
    chaoscontrol_smr::validate_observation_transition(plan, &chain, &observation.command_bytes)
        .map_err(|_| Issue::Transition)?;
    let bytes = serde_json::to_vec(&chain).map_err(|_| Issue::ProjectionEncoding)?;
    let length = u64::try_from(bytes.len()).map_err(|_| Issue::ProjectionBounds)?;
    if length > MAX_INLINE_PROJECTION_BYTES {
        return Err(Issue::ProjectionBounds);
    }
    let envelope = chaoscontrol_smr::observation::ProtocolObservation {
        schema: chaoscontrol_smr::observation::SCHEMA.to_owned(),
        profile_ref: protocol_ref.clone(),
        protocol_ref: protocol.protocol_ref.clone(),
        projection_schema_ref: protocol.projection_schema_ref.clone(),
        participant_ref: participant.participant_ref.clone(),
        producer_ref: participant.producer_ref.clone(),
        guest_id: participant.guest_id,
        generation: participant.generation,
        source_sequence: context.source_sequence,
        transition_class_ref: context.transition_class_ref.clone(),
        boundary_ref: context.boundary_ref.clone(),
        projection_ref: chaoscontrol_smr::observation::projection_ref(&bytes),
        projection: Some(bytes),
        scheduler_ref: context.scheduler_ref.clone(),
        dropped: 0,
    };
    envelope.admit(protocol, &protocol_ref).map_err(|_| Issue::Envelope)?;
    Ok(BoundObservation {
        envelope,
        apply: ApplyBinding {
            group_ref: observation.group_ref.clone(),
            participant_ref: observation.replica_ref.clone(),
            operation_ref: observation.operation_ref.clone(),
            command_ref: observation.command_ref.clone(),
            command_bytes: observation.command_bytes.clone(),
            lifecycle_generation: observation.lifecycle_generation,
            application_receipt_ref: observation.application_receipt_ref.clone(),
            chain,
        },
    })
}

pub fn validate(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    expected_group_ref: &str,
    observation: &BoundObservation,
) -> Result<(), Issue> {
    let protocol_ref = checks::admit_profiles(protocol, plan)?;
    checks::reference(expected_group_ref)?;
    let binding = &observation.apply;
    checks::binding_shape(binding)?;
    if binding.group_ref != expected_group_ref {
        return Err(Issue::Group);
    }
    checks::command_bounds(&binding.command_bytes, plan.profile.bounds.maximum_command_bytes)?;
    if binding.chain.profile_ref != plan.profile_ref {
        return Err(Issue::WorkloadProfile);
    }
    let envelope = &observation.envelope;
    if envelope.participant_ref != binding.participant_ref {
        return Err(Issue::Participant);
    }
    if envelope.generation != binding.lifecycle_generation {
        return Err(Issue::Generation);
    }
    if envelope.source_sequence != binding.chain.event_sequence {
        return Err(Issue::SourceSequence);
    }
    if envelope.dropped != 0 {
        return Err(Issue::LossAccounting);
    }
    // Preflight untrusted inline bytes and string fields before producer admission
    // serializes the complete envelope. Hash-only SDK records are never repaired.
    for value in [
        &envelope.profile_ref,
        &envelope.protocol_ref,
        &envelope.projection_schema_ref,
        &envelope.participant_ref,
        &envelope.producer_ref,
        &envelope.transition_class_ref,
        &envelope.boundary_ref,
        &envelope.projection_ref,
        &envelope.scheduler_ref,
    ] {
        checks::reference(value)?;
    }
    if envelope.schema != chaoscontrol_smr::observation::SCHEMA {
        return Err(Issue::Envelope);
    }
    let bytes = envelope.projection.as_ref().ok_or(Issue::MissingInline)?;
    let length = u64::try_from(bytes.len()).map_err(|_| Issue::ProjectionBounds)?;
    if length > MAX_INLINE_PROJECTION_BYTES || length > protocol.bounds.record_bytes {
        return Err(Issue::ProjectionBounds);
    }
    envelope.admit(protocol, &protocol_ref).map_err(|_| Issue::Envelope)?;
    let inline: chaoscontrol_smr::ChainObservation =
        serde_json::from_slice(bytes).map_err(|_| Issue::ProjectionEncoding)?;
    if inline != binding.chain {
        return Err(Issue::ProjectionMismatch);
    }
    chaoscontrol_smr::validate_observation_transition(plan, &binding.chain, &binding.command_bytes)
        .map_err(|_| Issue::Transition)
}

/// Identity of retained Molten provenance, independent of JSON formatting and
/// distinct from the producer's protocol-projection identity. This is not proof
/// that the supplied receipt or command was actually committed.
pub fn binding_ref(binding: &ApplyBinding) -> Result<String, Issue> {
    checks::binding_shape(binding)?;
    let expected = chaoscontrol_smr::chain_transition(
        &binding.chain.profile_ref,
        binding.chain.command_index,
        &binding.chain.prior_digest,
        &binding.command_bytes,
    )
    .map_err(|_| Issue::Transition)?;
    if expected != binding.chain.digest {
        return Err(Issue::Transition);
    }
    let chain = &binding.chain;
    let value = crate::preserves_rail::record("molten-chaoscontrol-apply-binding-v1", vec![
        crate::preserves_rail::string(&binding.group_ref),
        crate::preserves_rail::string(&binding.participant_ref),
        crate::preserves_rail::string(&binding.operation_ref),
        crate::preserves_rail::string(&binding.command_ref),
        preserves::IOValue::bytes(binding.command_bytes.clone()),
        crate::preserves_rail::u64_value(binding.lifecycle_generation),
        crate::preserves_rail::string(&binding.application_receipt_ref),
        crate::preserves_rail::record("chaoscontrol-smr-chain-observation-v1", vec![
            crate::preserves_rail::u64_value(chain.event_sequence),
            crate::preserves_rail::string(&chain.profile_ref),
            crate::preserves_rail::string(&chain.replica_id),
            crate::preserves_rail::u64_value(chain.command_index),
            crate::preserves_rail::string(&chain.prior_digest),
            crate::preserves_rail::string(&chain.digest),
            crate::preserves_rail::string(&chain.application_state_ref),
        ]),
    ]);
    crate::preserves_rail::canonical_hash(&value).map_err(|_| Issue::CanonicalBinding)
}

#[path = "wire/checks.rs"]
mod checks;
#[cfg(test)]
#[path = "wire/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "wire/intaketests.rs"]
mod intaketests;
#[cfg(test)]
#[path = "wiretests.rs"]
mod tests;
#[cfg(test)]
#[path = "wire/validationtests.rs"]
mod validationtests;
