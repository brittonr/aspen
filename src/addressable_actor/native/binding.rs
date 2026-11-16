use molten_core::addressable_actor::*;

use crate::fabric_membership::AssignmentPersistence;

pub(super) struct Observation {
    pub instance: crate::system_extension::NativeInstanceRecord,
    pub assignment: crate::fabric_membership::CurrentAssignment,
    pub authority_ref: String,
    pub resources_admitted: bool,
}

pub(super) struct Input<'a> {
    pub configuration: &'a super::Configuration,
    pub input_ref: &'a str,
    pub logical_tick: u64,
}

pub(super) fn observe(
    owner: &mut crate::node_service_host::installed::Owner,
    signing: &mut dyn super::SigningPort,
    placement: &crate::fabric_membership::persistence::Redb,
    input: Input<'_>,
) -> crate::error::Result<Observation> {
    let Input {
        configuration,
        input_ref,
        logical_tick,
    } = input;
    let actor_key_ref = identify_actor_key(&configuration.actor_key);
    super::prepare(
        signing,
        owner,
        crate::node_service_hosting::native::actor::Input::Observe(&actor_key_ref),
        logical_tick,
    )?;
    let instance = owner.actor_instance(&actor_key_ref, logical_tick)?;
    let manifest = owner.actor_manifest(&actor_key_ref)?;
    let assignment = placement
        .current(&configuration.assignment_id)
        .map_err(|error| super::invalid(format!("current actor placement unavailable: {error:?}")))?
        .ok_or_else(|| super::invalid("actor has no current committed placement"))?;
    let role = &assignment.assignment;
    if role.state != crate::fabric_membership::AssignmentState::Active
        || role.uncertain_old_owner
        || role.node_id != configuration.node_id
        || role.role_id != configuration.role_id
        || role.service_id != actor_key_ref
        || role.extension_id != instance.extension_id
        || role.service_generation != instance.lifecycle.generation.get()
        || manifest.manifest_ref() != instance.manifest_ref
        || manifest.manifest().service_id != actor_key_ref
        || instance.profile_ref != configuration.profile.system_extension_profile_ref
        || role.fencing_profile_ref != configuration.profile.placement_profile_ref
        || !instance.unresolved.is_empty()
    {
        return Err(super::invalid("actor native identity, placement, generation or unresolved-effect fence denied"));
    }
    let is_resource_usage_admitted =
        crate::system_extension::validate_resource_usage(&manifest.manifest().resources, instance.usage).is_empty();
    if !is_resource_usage_admitted {
        return Err(super::invalid("actor native resource observation exceeds admitted manifest"));
    }
    // Current verifies operator policy, expiry, revocations and exact holder invocation here.
    signing.prepare(&owner.actor_admission_preview(&actor_key_ref, input_ref, logical_tick)?)?;
    let authority_ref = owner.actor_authorize(&actor_key_ref, input_ref, logical_tick)?;
    crate::preserves_rail::ContentRef::parse(&authority_ref)?;
    Ok(Observation {
        instance,
        assignment,
        authority_ref,
        resources_admitted: is_resource_usage_admitted,
    })
}

impl Observation {
    pub(super) fn host(&self, configuration: &super::Configuration) -> crate::addressable_actor::HostBindingFacts {
        let profile_ref = identify_addressable_actor_profile(&configuration.profile);
        let native = &self.instance;
        let is_authorized = crate::preserves_rail::ContentRef::parse(&self.authority_ref).is_ok();
        crate::addressable_actor::HostBindingFacts {
            schema: ACTOR_HOST_BINDING_SCHEMA.to_string(),
            actor_key_ref: identify_actor_key(&configuration.actor_key),
            profile_ref,
            system_extension_manifest_ref: native.manifest_ref.clone(),
            placement_ref: self.assignment.assignment_ref.clone(),
            extension_generation: native.lifecycle.generation.get(),
            system_extension_generation: native.lifecycle.generation.get(),
            system_extension_phase: native.lifecycle.phase,
            system_extension_checkpoint_ref: native.checkpoint_ref.clone(),
            delivery_profile_ref: configuration.profile.delivery_profile_ref.clone(),
            policy_current: is_authorized,
            capability_current: is_authorized,
            placement_current: self.assignment.assignment.state == crate::fabric_membership::AssignmentState::Active,
            resources_admitted: self.resources_admitted,
            adapter_admitted: native.profile_ref == configuration.profile.system_extension_profile_ref,
        }
    }
}
