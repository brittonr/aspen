impl<A, P> crate::node_service_hosting::Host<A, super::Runtime<P, crate::system_extension::Durable>>
where
    A: AuthorityPort,
    P: crate::fabric_execution::ExecutionFabricPort,
{
    pub(crate) fn actor_delivery_acknowledge<C: crate::fabric_time::TimerClockAdapter>(
        &mut self,
        delivery_member: &str,
        actor_member: &str,
        bridge: &mut crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
        commit: &crate::addressable_actor::delivery::Commit,
    ) -> crate::error::Result<crate::addressable_actor::delivery::Acknowledgement> {
        bridge.acknowledge(
            &mut self.authority,
            self.runtime.service(delivery_member)?.host(),
            self.runtime.service(actor_member)?.host(),
            request,
            commit,
        )
    }

    pub(crate) fn actor_delivery_native_commit_preview(
        &self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        crate::preserves_rail::ContentRef::parse(ingress_request_ref)?;
        let current = self.actor_current(member, logical_tick)?;
        let input_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "host-native-actor-commit-input-v1",
            vec![
                crate::system_extension::canonical_instance_record(&current)?.value,
                crate::preserves_rail::string(ingress_request_ref),
            ],
        ))?;
        self.request("actor-native-commit", member, &input_ref, logical_tick)
    }

    pub(crate) fn actor_delivery_native_commit(
        &mut self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::addressable_actor::delivery::Commit> {
        self.actor_delivery_native_commit_optional(member, ingress_request_ref, logical_tick)?
            .ok_or_else(|| crate::error::Failure::invalid_harness("native actor ingress has no completed durable operation"))
    }

    pub(crate) fn actor_delivery_native_commit_optional(
        &mut self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<Option<crate::addressable_actor::delivery::Commit>> {
        let request = self.actor_delivery_native_commit_preview(member, ingress_request_ref, logical_tick)?;
        self.admit(&request.operation, member, &request.input_ref, logical_tick)?;
        crate::addressable_actor::delivery::Commit::observe_optional(
            self.runtime.service(member)?.host(),
            ingress_request_ref,
        )
    }
}
