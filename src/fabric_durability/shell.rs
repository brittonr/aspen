//! Application shell for admitted durable effect routing.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the shell visibly composes durable contracts with system-extension effect routing"
)]

use std::collections::BTreeMap;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::FabricPortError;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.shell]

pub struct RegisteredDurableEffectPort<A: DurableCommandShell> {
    adapter: A,
    requests: BTreeMap<String, DurablePortCommand>,
}

impl<A: DurableCommandShell> RegisteredDurableEffectPort<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            requests: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, request_ref: String, command: DurablePortCommand) -> Result<()> {
        crate::preserves_rail::validate_content_ref(&request_ref)?;
        if self.requests.insert(request_ref.clone(), command).is_some() {
            return Err(MoltenError::invalid_harness(format!("durable request {request_ref} is already registered")));
        }
        Ok(())
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }
}

// r[impl molten.fabric_durability.port_contracts]
impl<A: DurableCommandShell> crate::system_extension::FabricEffectPort for RegisteredDurableEffectPort<A> {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> FabricPortResult<crate::system_extension::PortEffectOutput> {
        if binding.binding.implementation_profile != self.adapter.profile_id() {
            return Err(FabricPortError::malformed("durable effect profile substitution denied"));
        }
        match &effect.target {
            crate::system_extension::EffectTarget::FabricPort(key) if key == &binding.binding.key => {}
            crate::system_extension::EffectTarget::FabricPort(_) => {
                return Err(FabricPortError::malformed("durable effect target does not match its bound port"));
            }
            crate::system_extension::EffectTarget::Ambient(_) => {
                return Err(FabricPortError::malformed("ambient effect cannot route through a durable port"));
            }
        }
        let command = self
            .requests
            .get(&effect.request_ref)
            .cloned()
            .ok_or_else(|| FabricPortError::malformed("durable effect request is not registered"))?;
        if binding.binding.key.port_id != command.port_id() {
            return Err(FabricPortError::malformed("durable effect command does not match its bound port"));
        }
        if effect.generation != command.generation() {
            return Err(FabricPortError::malformed("durable effect generation does not match its registered command"));
        }
        let transition = self.adapter.execute_command(&command)?;
        Ok(crate::system_extension::PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref: transition.transition_ref,
        })
    }
}
