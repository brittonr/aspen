//! Application shell for admitted transport effect routing.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the shell visibly composes transport contracts with system-extension effect routing"
)]

use std::collections::BTreeMap;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::FabricPortError;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.shell]

pub struct RegisteredTransportEffectPort<A: TransportCommandShell> {
    adapter: A,
    context: ExtensionTransportContext,
    profile: CanonicalTransportProfile,
    requests: BTreeMap<String, TransportCommand>,
}

impl<A: TransportCommandShell> RegisteredTransportEffectPort<A> {
    pub fn new(adapter: A, context: ExtensionTransportContext, profile: CanonicalTransportProfile) -> Result<Self> {
        if adapter.profile_id() != profile.profile.profile_id {
            return Err(MoltenError::invalid_harness("registered transport adapter profile mismatch"));
        }
        Ok(Self {
            adapter,
            context,
            profile,
            requests: BTreeMap::new(),
        })
    }

    pub fn register(&mut self, request_ref: String, command: TransportCommand) -> Result<()> {
        crate::preserves_rail::validate_content_ref(&request_ref)?;
        if self.requests.insert(request_ref.clone(), command).is_some() {
            return Err(MoltenError::invalid_harness(format!("transport request {request_ref} is already registered")));
        }
        Ok(())
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }

    pub(crate) fn adapter_mut(&mut self) -> &mut A {
        &mut self.adapter
    }

    pub(crate) fn execute_effect(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> FabricPortResult<(TransportCommand, CanonicalTransportTransition)> {
        if binding.binding.key.port_id != FABRIC_TRANSPORT_PORT_ID
            || binding.binding.key.version != FABRIC_TRANSPORT_PORT_VERSION
        {
            return Err(FabricPortError::malformed("transport effect routed through the wrong fabric port"));
        }
        if binding.binding.implementation_profile != self.adapter.profile_id() {
            return Err(FabricPortError::malformed("transport effect profile substitution denied"));
        }
        match &effect.target {
            crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key => {}
            crate::system_extension::EffectTarget::FabricPort(_) => {
                return Err(FabricPortError::malformed("transport effect target does not match its bound port"));
            }
            crate::system_extension::EffectTarget::Ambient(_) => {
                return Err(FabricPortError::malformed("ambient effect cannot route through a transport port"));
            }
        }
        let command = self
            .requests
            .get(&effect.request_ref)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("transport effect request is not registered"))?;
        self.context
            .admit_command(&self.profile, &command, effect.accounted_bytes)
            .map_err(FabricPortError::from)?;
        if effect.generation != command.generation() {
            return Err(FabricPortError::malformed(
                "transport effect generation does not match its registered command",
            ));
        }
        let transition = self.adapter.execute_command(&command)?;
        Ok((command, transition))
    }
}

// r[impl molten.fabric_transport.port_contract]
// r[impl molten.fabric_transport.session_streams]
impl<A: TransportCommandShell> crate::system_extension::FabricEffectPort for RegisteredTransportEffectPort<A> {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> FabricPortResult<crate::system_extension::PortEffectOutput> {
        let (_command, transition) = self.execute_effect(binding, effect)?;
        Ok(crate::system_extension::PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref: transition.transition_ref,
        })
    }
}
