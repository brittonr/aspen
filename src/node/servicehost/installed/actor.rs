//! Narrow installed-owner actor operations; ownership never escapes as a mutable dispatcher.

impl super::Owner {
    /// Describe an exact actor request for an independent current-operation signer.
    pub fn actor_preview(
        &self,
        input: crate::node_service_hosting::native::actor::Input<'_>,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.actor_preview(input, logical_tick)
    }

    /// Borrow admitted manifest metadata, which neither grants authority nor performs an effect.
    pub fn actor_manifest(&self, member: &str) -> crate::error::Result<&crate::system_extension::AdmittedManifest> {
        self.host.actor_manifest(member)
    }

    /// Bind the actor effect input to exact current Host state for an independent signer.
    pub fn actor_admission_preview(
        &self,
        member: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.actor_admission_preview(member, input_ref, logical_tick)
    }

    /// Require the current actor-effect credential. This is not a lifecycle callback or a placement
    /// grant.
    pub fn actor_authorize(
        &mut self,
        member: &str,
        input_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<String> {
        self.host.actor_authorize(member, input_ref, logical_tick)
    }

    /// Observe the actual durable native instance after current exact-operation admission.
    pub fn actor_instance(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.host.actor_instance(member, logical_tick)
    }

    /// Execute a journal-backed checkpoint under a persisted Host lifecycle intent.
    pub fn actor_checkpoint(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.host.actor_checkpoint(member, logical_tick)
    }

    /// Checkpoint and drain without terminal shutdown; native callback processes have completed on
    /// return.
    pub fn actor_sleep(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.host.actor_sleep(member, logical_tick)
    }

    /// Drain an already checkpointed actor without repeating the selected checkpoint callback.
    pub fn actor_drain(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.host.actor_drain(member, logical_tick)
    }

    /// Recover a drained checkpoint with a fresh incarnation, or observe already satisfied Running
    /// state.
    pub fn actor_rewake(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.host.actor_rewake(member, logical_tick)
    }

    /// Materialize current semantic-state bytes under their native content identity and current
    /// read authority.
    pub fn actor_state(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        self.host.actor_state(member, logical_tick)
    }

    /// Materialize actual checkpoint bytes; no state or checkpoint wrapper is synthesized.
    pub fn actor_checkpoint_value(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeCallbackValue> {
        self.host.actor_checkpoint_value(member, logical_tick)
    }

    /// Project exact bounded durable-work staging while the actual native actor remains Drained.
    pub fn actor_submit_dormant_preview(
        &self,
        member: &str,
        presentation: &crate::system_extension::WorkPresentation,
        payload: &crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.actor_submit_dormant_preview(member, presentation, payload, logical_tick)
    }

    /// Retain authorized durable work in the existing Host store without opening native ingress.
    /// Waking does not grant delivery authority: a fresh actual readmission claim is still
    /// required.
    pub fn actor_submit_dormant(
        &mut self,
        member: &str,
        presentation: crate::system_extension::WorkPresentation,
        payload: crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        self.host.actor_submit_dormant(member, presentation, payload, logical_tick)
    }

    /// Obtain the existing Host's actual current durable-work claim after wake, never a synthetic
    /// claim. Its exact signer preview remains `preview(Invocation::Readmit { member, work_ref
    /// }, logical_tick)`.
    pub fn actor_readmit(
        &mut self,
        member: &str,
        work_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::DeliveryClaim> {
        self.host.readmit(member, work_ref, logical_tick)
    }

    /// Sign the existing retained-work delivery request, without delivering it.
    pub fn actor_ingress_preview(
        &self,
        input: &crate::node_service_hosting::DeliveryInput,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.preview(crate::node_service_hosting::preview::Input::Deliver(input), input.logical_tick)
    }

    /// Deliver existing retained Host work through native admission and approved-effect routing.
    pub fn actor_ingress(
        &mut self,
        input: &crate::node_service_hosting::DeliveryInput,
    ) -> crate::error::Result<crate::node_service_hosting::RuntimeDelivery> {
        self.host.actor_ingress(input)
    }

    /// Describe a delivery bridge operation without granting delivery or acknowledgement authority.
    pub fn delivery_preview<C: crate::fabric_time::TimerClockAdapter>(
        &self,
        member: &str,
        bridge: &crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
        commit: Option<&crate::addressable_actor::delivery::Commit>,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.actor_delivery_preview(member, bridge, request, commit)
    }

    /// Apply an independently authorized coordination delivery operation against the retained
    /// native owner.
    pub fn delivery_apply<C: crate::fabric_time::TimerClockAdapter>(
        &mut self,
        member: &str,
        bridge: &mut crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
    ) -> crate::error::Result<crate::coordination_delivery::Outcome> {
        self.host.actor_delivery_apply(member, bridge, request)
    }

    /// Acknowledge only the durable native commit proved by the delivery bridge.
    pub fn delivery_acknowledge<C: crate::fabric_time::TimerClockAdapter>(
        &mut self,
        delivery_member: &str,
        actor_member: &str,
        bridge: &mut crate::addressable_actor::delivery::Bridge<C>,
        request: &molten_core::coordination_delivery::DeliveryRequest,
        commit: &crate::addressable_actor::delivery::Commit,
    ) -> crate::error::Result<crate::addressable_actor::delivery::Acknowledgement> {
        self.host.actor_delivery_acknowledge(delivery_member, actor_member, bridge, request, commit)
    }

    /// Describe the exact protected native commit/state read for an independent current signer.
    pub fn delivery_native_commit_preview(
        &self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.host.actor_delivery_native_commit_preview(member, ingress_request_ref, logical_tick)
    }

    /// Derive proof from actual native durable ingress/callback history, never caller-supplied
    /// success flags.
    pub fn delivery_native_commit(
        &mut self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::addressable_actor::delivery::Commit> {
        self.host.actor_delivery_native_commit(member, ingress_request_ref, logical_tick)
    }

    /// Observe pending native work as absence, preserving authority, storage and corruption errors.
    pub fn delivery_native_commit_optional(
        &mut self,
        member: &str,
        ingress_request_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<Option<crate::addressable_actor::delivery::Commit>> {
        self.host.actor_delivery_native_commit_optional(member, ingress_request_ref, logical_tick)
    }
}
