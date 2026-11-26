/// Independently provisioned placement inputs, never a caller-authored Active assignment.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub assignment_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub role_id: String,
    pub role_kind: String,
    pub node_id: String,
    pub service_generation: u64,
    pub assignment_epoch: u64,
    pub fencing_token: u64,
    pub fencing_profile_ref: String,
    pub resource_reservation_ref: String,
    pub placement_plan_ref: String,
    pub authority_ref: String,
    pub durable_state_ref: Option<String>,
    pub predecessor_assignment_ref: Option<String>,
    pub predecessor_epoch: Option<u64>,
}

impl Selection {
    pub fn proposal(&self) -> crate::fabric_membership::AssignmentProposal {
        crate::fabric_membership::AssignmentProposal {
            assignment_id: self.assignment_id.clone(),
            extension_id: self.extension_id.clone(),
            service_id: self.service_id.clone(),
            role_id: self.role_id.clone(),
            role_kind: self.role_kind.clone(),
            node_id: self.node_id.clone(),
            service_generation: self.service_generation,
            assignment_epoch: self.assignment_epoch,
            fencing_token: self.fencing_token,
            fencing_profile_ref: self.fencing_profile_ref.clone(),
            resource_reservation_ref: self.resource_reservation_ref.clone(),
            placement_plan_ref: self.placement_plan_ref.clone(),
            authority_ref: self.authority_ref.clone(),
            durable_state_ref: self.durable_state_ref.clone(),
            predecessor_assignment_ref: self.predecessor_assignment_ref.clone(),
            predecessor_epoch: self.predecessor_epoch,
        }
    }

    pub fn validate(&self, actor_key_ref: &str) -> crate::error::Result<()> {
        if self.extension_id != "molten.addressable-counter"
            || self.service_id != actor_key_ref
            || self.service_generation != 1
            || self.predecessor_assignment_ref.is_some()
            || self.predecessor_epoch.is_some()
        {
            return Err(super::invalid("counter placement must select the exact initial actor service"));
        }
        crate::fabric_membership::propose_assignment(&self.proposal())
            .map_err(|issues| super::invalid(format!("counter placement proposal denied: {issues:?}")))?;
        Ok(())
    }
}

use crate::addressable_actor::native::SigningPort;
use crate::fabric_membership::*;
use crate::node_service_hosting::AuthorityPort;

mod effects;

pub(super) fn initialize(session: &mut super::Session, path: &str) -> crate::error::Result<serde_json::Value> {
    let proposal: Selection = super::read_json(&session.root, path)?;
    proposal.validate(&session.member())?;
    if proposal.node_id != session.configuration.node_id
        || proposal.assignment_id != session.configuration.assignment_id
        || proposal.role_id != session.configuration.role_id
        || proposal.fencing_profile_ref != session.configuration.profile.placement_profile_ref
    {
        return Err(super::invalid("placement proposal differs from selected actor configuration"));
    }
    let input = serde_json::to_string(&proposal).map_err(super::problem)?;
    let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "actor-placement-proposal-input-v1",
        vec![crate::preserves_rail::string(input)],
    ))?;
    let empty = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "uninitialized-actor-placement-v1",
        vec![],
    ))?;
    authorize(session, "actor-placement-initialize", &proposal.assignment_id, &empty, &input_ref)?;
    let current = session.placement.initialize(&proposal.proposal()).map_err(super::problem)?;
    Ok(
        serde_json::json!({"assignment_ref": current.assignment_ref, "state": current.assignment.state.as_str(),
        "authority": "exact-current-operator-invocation", "semantic_outcome": "progress"}),
    )
}

pub(super) fn step(session: &mut super::Session, input: &serde_json::Value) -> crate::error::Result<serde_json::Value> {
    let kind = match input.get("kind").and_then(serde_json::Value::as_str) {
        Some("reserve") => AssignmentCommandKind::Reserve,
        Some("assign") => AssignmentCommandKind::Assign,
        Some("acknowledge") => AssignmentCommandKind::Acknowledge,
        Some("activate") => AssignmentCommandKind::Activate,
        _ => return Err(super::invalid("bounded actor placement supports reserve, assign, acknowledge and activate")),
    };
    let current = session
        .placement
        .current(&session.configuration.assignment_id)
        .map_err(super::problem)?
        .ok_or_else(|| super::invalid("actor placement is not initialized"))?;
    let native = session.observe()?;
    let effect_port_ref = validated_effect_port(session, &current, &native)?;
    let input_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("actor-placement-step-input-v1", vec![
            crate::preserves_rail::string(&current.assignment_ref),
            crate::preserves_rail::string(kind.as_str()),
            crate::preserves_rail::string(&native.manifest_ref),
            crate::preserves_rail::u64_value(session.tick),
            crate::preserves_rail::string(&session.membership.admission_ref),
        ]))?;
    authorize(
        session,
        "actor-placement-step",
        &current.assignment.assignment_id,
        &current.assignment_ref,
        &input_ref,
    )?;
    let command = AssignmentCommand {
        kind,
        assignment_id: current.assignment.assignment_id.clone(),
        service_generation: current.assignment.service_generation,
        assignment_epoch: current.assignment.assignment_epoch,
        fencing_token: current.assignment.fencing_token,
        transition_ref: input_ref,
        uncertain_old_owner: false,
    };
    // The durable current role slot supplies the enforced epoch/token, not caller booleans.
    let authority = AssignmentAuthoritySnapshot {
        fencing_profile: FencingProfile {
            schema: FENCING_PROFILE_SCHEMA.to_string(),
            profile_id: "counter-node-local-fencing-v1".to_string(),
            profile_ref: current.assignment.fencing_profile_ref.clone(),
            authority_ref: current.assignment.authority_ref.clone(),
            effect_port_ref,
            enforcement: FencingEnforcement::NodeLocalDurable,
        },
        enforced_assignment_epoch: current.assignment.assignment_epoch,
        enforced_fencing_token: current.assignment.fencing_token,
        required_enforcement: FencingEnforcement::NodeLocalDurable,
    };
    let mut native = effects::Native {
        owner: &mut session.owner,
        signing: &mut session.signing,
    };
    let mut lifecycle = effects::Lifecycle::new(&mut native, session.tick);
    let outcome = execute_and_abort_pre_effect(
        &mut session.placement,
        &mut lifecycle,
        &current.assignment,
        &command,
        &authority,
    )?;
    step_result(session, outcome)
}

/// Runs one placement command and, when the lifecycle refused the role effect before any native
/// effect, clears that exact pending intent through the owner's port-verified abort so the next
/// placement step reads a committed current assignment instead of failing on the pending intent.
/// The refusal itself is still returned as an error.
fn execute_and_abort_pre_effect(
    placement: &mut dyn AssignmentPersistence,
    lifecycle: &mut effects::Lifecycle<'_>,
    current: &RoleAssignment,
    command: &AssignmentCommand,
    authority: &AssignmentAuthoritySnapshot,
) -> crate::error::Result<AssignmentExecutionOutcome> {
    let outcome =
        execute_assignment_command(placement, lifecycle, current, command, authority).map_err(super::problem)?;
    let AssignmentExecutionOutcome::Uncertain(uncertain) = &outcome else {
        return Ok(outcome);
    };
    let (AssignmentExecutionPhase::RoleEffect, false, Some(intent_ref)) =
        (&uncertain.phase, uncertain.effect_may_have_happened, uncertain.intent_ref.as_deref())
    else {
        return Ok(outcome);
    };
    placement
        .abort_pre_effect_intent(&current.assignment_id, intent_ref, lifecycle)
        .map_err(|error| super::invalid(format!("placement refusal left an unresolved intent: {error}")))?;
    Err(super::invalid(format!(
        "placement role effect refused before any native effect; pending intent aborted: {}",
        uncertain.message
    )))
}

fn validated_effect_port(
    session: &super::Session,
    current: &CurrentAssignment,
    native: &crate::system_extension::NativeInstanceRecord,
) -> crate::error::Result<String> {
    let manifest = session.owner.actor_manifest(&session.member())?;
    let binding = manifest
        .binding_for(&crate::fabric::FabricPortKey {
            port_id: FABRIC_ASSIGNMENT_PORT_ID.to_string(),
            version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
        })
        .ok_or_else(|| super::invalid("actor has no admitted membership assignment binding"))?;
    let descriptor = crate::fabric_membership::port_descriptors(&session.membership)
        .into_iter()
        .find(|descriptor| descriptor.port_id == FABRIC_ASSIGNMENT_PORT_ID)
        .ok_or_else(|| super::invalid("membership profile has no assignment descriptor"))?;
    let (descriptor_ref, _) = crate::fabric::canonical_port_descriptor(&descriptor)?;
    if binding.descriptor_ref != descriptor_ref
        || session.membership.profile.authority_scope != session.configuration.node_id
        || current.assignment.node_id != session.configuration.node_id
    {
        return Err(super::invalid(
            "placement substituted exact admitted membership profile, descriptor or node scope",
        ));
    }
    if binding.binding.implementation_profile != session.membership.profile.profile_id
        || current.assignment.service_id != session.member()
        || current.assignment.extension_id != native.extension_id
        || current.assignment.service_generation != native.lifecycle.generation.get()
        || !native.unresolved.is_empty()
    {
        return Err(super::invalid("placement native identity, profile or generation fence differs"));
    }
    Ok(binding.binding_ref.clone())
}

fn step_result(
    session: &super::Session,
    outcome: AssignmentExecutionOutcome,
) -> crate::error::Result<serde_json::Value> {
    match outcome {
        AssignmentExecutionOutcome::Committed(receipt) => {
            session.retain(&receipt.value)?;
            let observed = session
                .placement
                .current(&session.configuration.assignment_id)
                .map_err(super::problem)?
                .ok_or_else(|| super::invalid("committed placement disappeared"))?;
            Ok(
                serde_json::json!({"assignment_ref": observed.assignment_ref, "state": observed.assignment.state.as_str(),
                "receipt_ref": receipt.transition_ref, "receipt": crate::preserves_rail::to_text(&receipt.value)?, "semantic_outcome": "progress"}),
            )
        }
        AssignmentExecutionOutcome::Uncertain(uncertain) => {
            Err(super::invalid(format!("placement outcome unknown; never replay role effect: {uncertain:?}")))
        }
    }
}

fn authorize(
    session: &mut super::Session,
    operation: &str,
    subject: &str,
    state_ref: &str,
    input_ref: &str,
) -> crate::error::Result<()> {
    let request = crate::node_service_hosting::OperationRequest {
        operation: operation.to_string(),
        subject: subject.to_string(),
        logical_tick: session.tick,
        state_ref: state_ref.to_string(),
        input_ref: input_ref.to_string(),
    };
    session.signing.prepare(&request)?;
    let mut current = crate::node_service_host::authority::Current::open(
        &session.root,
        &session.bundle_ref,
        crate::world_witness::live_clock()?,
    )?;
    current.admit(&request)?;
    Ok(())
}

#[cfg(test)]
mod tests;
