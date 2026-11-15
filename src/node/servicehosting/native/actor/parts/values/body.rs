impl<A, P, J> crate::node_service_hosting::Host<A, super::Runtime<P, J>>
where
    A: AuthorityPort,
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    /// Materialize exactly the current semantic-state bytes, verifying their canonical native
    /// identity.
    pub fn actor_state(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        self.actor_materialize(Input::State(member), logical_tick)
    }

    /// Materialize exactly the current checkpoint bytes, not a synthetic checkpoint projection.
    pub fn actor_checkpoint_value(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        self.actor_materialize(Input::CheckpointValue(member), logical_tick)
    }

    /// Describe the admitted manifest without performing an actor effect or granting authority.
    pub(crate) fn actor_manifest(
        &self,
        member: &str,
    ) -> crate::error::Result<&crate::system_extension::AdmittedManifest> {
        Ok(self.runtime.service(member)?.host().manifest())
    }

    pub(crate) fn actor_admission_preview(
        &self,
        member: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.actor_current(member, logical_tick)?;
        self.request("actor-effect", member, input_ref, logical_tick)
    }

    pub(crate) fn actor_authorize(
        &mut self,
        member: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<String> {
        self.actor_admission_preview(member, input_ref, logical_tick)?;
        self.admit("actor-effect", member, input_ref, logical_tick)
    }

    pub(crate) fn actor_delivery_preview<C: crate::fabric_time::TimerClockAdapter>(
        &self,
        member: &str,
        bridge: &crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
        commit: Option<&crate::addressable_actor::delivery::Commit>,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        bridge.preview(self.runtime.service(member)?.host(), request, commit)
    }

    pub(crate) fn actor_delivery_apply<C: crate::fabric_time::TimerClockAdapter>(
        &mut self,
        member: &str,
        bridge: &mut crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
    ) -> crate::error::Result<crate::coordination_delivery::Outcome> {
        bridge.apply(&mut self.authority, self.runtime.service(member)?.host(), request)
    }
    fn actor_materialize(
        &mut self,
        input: Input<'_>,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        self.actor_admit(input, logical_tick)?;
        let (_, member) = input.parts();
        let current = self.actor_current(member, logical_tick)?;
        let reference = match input {
            Input::State(_) => current.state_ref.as_deref(),
            Input::CheckpointValue(_) => current.checkpoint_ref.as_deref(),
            Input::Observe(_) | Input::Checkpoint(_) | Input::Sleep(_) | Input::Drain(_) | Input::Rewake(_) => {
                return Err(crate::node_service_hosting::invalid("invalid actor materialization operation"));
            }
        }
        .ok_or_else(|| crate::node_service_hosting::invalid("native actor has no materialized state/checkpoint"))?;
        let service = self.runtime.service(member)?;
        let value = service
            .host()
            .executor()
            .values()
            .lock()
            .map_err(|_| crate::node_service_hosting::invalid("native actor value port lock poisoned"))?
            .materialize(reference, crate::node_service_hosting::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| {
                crate::node_service_hosting::invalid(format!("native actor materialization failed: {error:?}"))
            })?;
        crate::system_extension::admit_callback_value(
            &value,
            crate::node_service_hosting::MAX_HOST_PAYLOAD_BYTES as u64,
        )
        .map_err(|error| {
            crate::node_service_hosting::invalid(format!("native actor materialized value denied: {error:?}"))
        })?;
        if value.value_ref != reference {
            return Err(crate::node_service_hosting::invalid("native actor materialization returned another identity"));
        }
        Ok(value)
    }
}
