pub(super) struct Scope {
    pub configuration: super::config::Binding,
    pub authority: super::authority::Gate,
    pub runtime: crate::collaboration_scope::native_runtime::Worker,
    pub recovery_group: Option<String>,
    pending: Vec<crate::system_extension::TypedEffectRequest>,
    logical_tick: Option<u64>,
}

impl Scope {
    pub(super) fn new(
        configuration: super::config::Binding,
        authority: super::authority::Gate,
        runtime: crate::collaboration_scope::native_runtime::Worker,
        recovery_group: Option<String>,
    ) -> Self {
        Self {
            configuration,
            authority,
            runtime,
            recovery_group,
            pending: Vec::new(),
            logical_tick: None,
        }
    }
}

// r[impl molten.system_extension.native_effect_authority]
// r[impl molten.system_extension.native_effect_uncertainty]
impl crate::node_service_hosting::NativeEffectRouter for Scope {
    fn admit(&mut self, input: crate::node_service_hosting::NativeEffectRouteInput<'_>) -> crate::error::Result<()> {
        let instance = input.instance;
        if !self.pending.is_empty()
            || instance.service_id != self.configuration.service_id
            || instance.manifest_ref != self.configuration.manifest_ref
            || instance.lifecycle.generation != self.configuration.generation
            || instance.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || instance
                .unresolved
                .iter()
                .any(|operation| operation.kind == crate::system_extension::NativeOperationKind::Effect)
            || input.group_id != self.recovery_group.as_deref()
            || input.callback.invocation.generation != instance.lifecycle.generation
            || input.callback.invocation.incarnation != instance.lifecycle.incarnation
            || crate::preserves_rail::canonical_hash(&input.callback.value)? != input.callback.receipt_ref
        {
            return Err(super::super::invalid(
                "scope effect runtime is stale, uncertain, or bound to another native service",
            ));
        }
        if input.callback.approved_effects.is_empty() {
            return Err(super::super::invalid("scope callback has no admitted effects"));
        }
        for effect in &input.callback.approved_effects {
            if effect.generation != self.configuration.generation
                || effect.incarnation != instance.lifecycle.incarnation
            {
                return Err(super::super::invalid(
                    "scope effect generation/incarnation differs from active native instance",
                ));
            }
        }
        self.pending.clone_from(&input.callback.approved_effects);
        self.logical_tick = Some(input.callback.invocation.logical_tick);
        Ok(())
    }
}

impl crate::system_extension::FabricEffectPort for Scope {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<crate::system_extension::PortEffectOutput> {
        let position = self.pending.iter().position(|pending| pending == effect).ok_or_else(|| {
            crate::fabric::FabricPortError::malformed("effect lacks current native callback admission")
        })?;
        let tick = self
            .logical_tick
            .ok_or_else(|| crate::fabric::FabricPortError::malformed("missing current effect tick"))?;
        // Consume this exact routing permission even if authority or the provider
        // later rejects: native intent/uncertainty owns any explicit recovery.
        self.pending.swap_remove(position);
        self.authority
            .admit_route(
                super::authority::RouteInput {
                    binding: &self.configuration,
                    actual_binding: binding,
                    effect,
                },
                tick,
            )
            .map_err(|_| crate::fabric::FabricPortError::malformed("current scope route authority denied"))?;
        crate::system_extension::FabricEffectPort::route(&mut self.runtime, binding, effect)
    }
}
