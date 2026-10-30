impl<P, J> super::Runtime<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    pub fn service(&self, member: &str) -> crate::error::Result<&crate::system_extension::Supervisor<P, J>> {
        self.services.get(member).ok_or_else(|| super::super::invalid("unknown native hosted member"))
    }

    pub fn register_effect_router(
        &mut self,
        member: &str,
        router: Box<dyn super::super::NativeEffectRouter>,
    ) -> crate::error::Result<()> {
        let instance = self.service(member)?.instance().map_err(super::supervisor_error)?;
        if instance.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || !instance.is_accepting_ingress
            || self.routers.contains_key(member)
        {
            return Err(super::super::invalid(
                "effect routing requires one explicitly admitted router for a running member",
            ));
        }
        self.routers.insert(member.to_string(), router);
        Ok(())
    }

    pub(super) fn deliver_routed(
        &mut self,
        member: &str,
        ingress: &crate::system_extension::NativeIngressEnvelope,
        logical_tick: u64,
    ) -> crate::error::Result<super::super::RuntimeDelivery> {
        let service =
            self.services.get_mut(member).ok_or_else(|| super::super::invalid("unknown native hosted member"))?;
        let result = service.ingress(ingress, logical_tick).map_err(super::supervisor_error)?;
        let effects = match &result.dispatch {
            crate::system_extension::DispatchResult::Executed {
                receipt,
                approved_effects,
                ..
            } if !approved_effects.is_empty() => match self.routers.get_mut(member) {
                None => super::super::NativeEffectDisposition::Unrouted,
                Some(router) => route(service, router.as_mut(), receipt, logical_tick)?,
            },
            _ => super::super::NativeEffectDisposition::NotRequested,
        };
        Ok(super::super::RuntimeDelivery {
            admission: result.admission,
            dispatch: result.dispatch,
            effects,
        })
    }
}

// r[impl molten.system_extension.native_effect_completion]
// r[impl molten.system_extension.native_effect_uncertainty]
fn route<P, J>(
    service: &mut crate::system_extension::Supervisor<P, J>,
    router: &mut dyn super::super::NativeEffectRouter,
    receipt: &crate::system_extension::CallbackReceipt,
    logical_tick: u64,
) -> crate::error::Result<super::super::NativeEffectDisposition>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    let current = service.instance().map_err(super::supervisor_error)?;
    if receipt.invocation.generation != current.lifecycle.generation
        || receipt.invocation.incarnation != current.lifecycle.incarnation
        || !current.is_accepting_ingress
    {
        return Err(super::super::invalid("effect callback lost its current native generation/incarnation fence"));
    }
    router.admit(super::super::NativeEffectRouteInput {
        instance: &current,
        callback: receipt,
        group_id: service.host().manifest().manifest().recovery_group.as_ref().map(|group| group.group_id.as_str()),
    })?;
    let completions = service.route_effects(receipt, router).map_err(super::supervisor_error)?;
    if completions.len() != receipt.approved_effects.len() {
        return Err(super::super::invalid("routed effect completion inventory is incomplete; work remains unknown"));
    }
    let mut committed = Vec::with_capacity(completions.len());
    for completion in completions {
        match service.deliver_effect_completion(&completion, logical_tick).map_err(super::supervisor_error)? {
            crate::system_extension::DispatchResult::Executed {
                receipt,
                approved_effects,
                ..
            } if approved_effects.is_empty() => {
                committed.push(receipt.receipt_ref);
            }
            _ => {
                return Err(super::super::invalid(
                    "effect completion callback is unresolved or requested further unrouted effects",
                ));
            }
        }
    }
    Ok(super::super::NativeEffectDisposition::Completed(committed))
}
