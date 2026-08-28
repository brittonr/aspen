use super::*;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorStatus {
    pub schema: String,
    pub actor_key_ref: String,
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub lifecycle_sequence: u64,
    pub revision: u64,
    pub phase: ActorPhase,
    pub checkpoint_ref: Option<String>,
    pub durable_state_ref: Option<String>,
    pub active_wake_ref: Option<String>,
    pub unknown_effect_ref: Option<String>,
    pub mailbox_revision: u64,
    pub last_activity_tick: u64,
    pub completed_event_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub truncated: bool,
    pub payloads_rendered: bool,
    pub authorizes_mutation: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ActorStatusProjectionInput<'a> {
    pub maximum_events: usize,
    pub evidence_refs: &'a [String],
}

// r[impl molten.addressable_actor.verification]
pub fn project_actor_status(
    state: &ActorState,
    input: ActorStatusProjectionInput<'_>,
) -> Result<ActorStatus, ActorIssue> {
    if !validate_actor_state(state).is_empty() || input.maximum_events == 0 {
        return Err(ActorIssue::StateIdentityMismatch);
    }
    if input.evidence_refs.iter().any(|reference| !crate::fabric::valid_blake3_ref(reference)) {
        return Err(ActorIssue::MalformedReference);
    }
    let completed_event_refs =
        state.completed_event_refs.iter().take(input.maximum_events).cloned().collect::<Vec<_>>();
    Ok(ActorStatus {
        schema: ACTOR_STATUS_SCHEMA.to_string(),
        actor_key_ref: state.actor_key_ref.clone(),
        profile_ref: state.profile_ref.clone(),
        system_extension_manifest_ref: state.system_extension_manifest_ref.clone(),
        placement_ref: state.placement_ref.clone(),
        extension_generation: state.extension_generation,
        lifecycle_sequence: state.lifecycle_sequence,
        revision: state.revision,
        phase: state.phase,
        checkpoint_ref: state.checkpoint_ref.clone(),
        durable_state_ref: state.durable_state_ref.clone(),
        active_wake_ref: state.active_wake_ref.clone(),
        unknown_effect_ref: state.unknown_effect_ref.clone(),
        mailbox_revision: state.mailbox_revision,
        last_activity_tick: state.last_activity_tick,
        truncated: completed_event_refs.len() != state.completed_event_refs.len(),
        completed_event_refs,
        evidence_refs: input.evidence_refs.to_vec(),
        payloads_rendered: false,
        authorizes_mutation: false,
    })
}
