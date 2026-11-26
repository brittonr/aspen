use molten_core::addressable_actor::*;

use crate::fabric_membership::AssignmentLog;

pub(super) struct Executor<'a, C: crate::fabric_time::TimerClockAdapter> {
    pub configuration: &'a super::Configuration,
    pub owner: &'a mut crate::node_service_host::installed::Owner,
    pub placement: &'a mut crate::fabric_membership::persistence::Redb,
    pub delivery: &'a mut crate::addressable_actor::delivery::Bridge<C>,
    pub signing: &'a mut dyn super::SigningPort,
    pub prepared_incarnation: Option<u64>,
    pub work: Option<&'a crate::node_service_hosting::DeliveryInput>,
    pub acknowledgement: Option<&'a molten_core::coordination_delivery::DeliveryRequest>,
    pub commit: Option<&'a crate::addressable_actor::delivery::Commit>,
    pub logical_tick: u64,
}

impl<C: crate::fabric_time::TimerClockAdapter> crate::addressable_actor::ActorEffectPort for Executor<'_, C> {
    fn observe_admission(
        &mut self,
        effect: &ActorEffectIntent,
    ) -> crate::addressable_actor::PortResult<crate::addressable_actor::EffectAdmissionObservation> {
        let observed = super::binding::observe(self.owner, self.signing, self.placement, super::binding::Input {
            configuration: self.configuration,
            input_ref: &effect.effect_ref,
            logical_tick: self.logical_tick,
        })
        .map_err(|error| super::port_error(error, false))?;
        let host = observed.host(self.configuration);
        self.prepared_incarnation = Some(observed.instance.lifecycle.incarnation.get());
        Ok(crate::addressable_actor::EffectAdmissionObservation {
            admission_ref: observed.authority_ref,
            actor_key_ref: host.actor_key_ref,
            profile_ref: host.profile_ref,
            system_extension_manifest_ref: host.system_extension_manifest_ref,
            placement_ref: host.placement_ref,
            extension_generation: host.extension_generation,
            policy_current: host.policy_current,
            capability_current: host.capability_current,
            placement_current: host.placement_current,
            generation_current: observed.instance.lifecycle.generation.get() == effect.extension_generation,
            resources_admitted: host.resources_admitted,
            adapter_admitted: host.adapter_admitted,
        })
    }

    fn execute(
        &mut self,
        effect: &ActorEffectIntent,
        admission: &crate::addressable_actor::EffectAdmissionObservation,
    ) -> crate::addressable_actor::PortResult<crate::addressable_actor::EffectObservation> {
        if !admission.admits(effect) {
            return Err(super::port_error("actor effect admission binding changed", false));
        }
        let current = self
            .placement
            .current(&self.configuration.assignment_id)
            .map_err(|error| super::port_error(error, false))?
            .ok_or_else(|| super::port_error("actor placement was revoked", false))?;
        if current.assignment_ref != effect.placement_ref
            || current.assignment.state != crate::fabric_membership::AssignmentState::Active
        {
            return Err(super::port_error("actor placement became stale before effect", false));
        }
        let member = identify_actor_key(&self.configuration.actor_key);
        super::prepare(
            self.signing,
            self.owner,
            crate::node_service_hosting::native::actor::Input::Observe(&member),
            self.logical_tick,
        )
        .map_err(|error| super::port_error(error, false))?;
        let native = self
            .owner
            .actor_instance(&member, self.logical_tick)
            .map_err(|error| super::port_error(error, false))?;
        if self.prepared_incarnation != Some(native.lifecycle.incarnation.get())
            || native.lifecycle.generation.get() != effect.extension_generation
            || native.manifest_ref != effect.system_extension_manifest_ref
        {
            return Err(super::port_error("actor native generation changed before effect", false));
        }
        // Preparation/read failures are definite until a mutating owner call is attempted.
        // Keep the flag set across nested calls and post-effect validation failures.
        let mut is_attempted = false;
        let outcome = self
            .run(effect, &member, &mut is_attempted)
            .map_err(|error| super::port_error(error, is_attempted))?;
        Ok(crate::addressable_actor::EffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref.clone(),
            disposition: crate::addressable_actor::EffectDisposition::Succeeded,
            outcome_ref: Some(outcome),
        })
    }
}

impl<C: crate::fabric_time::TimerClockAdapter> Executor<'_, C> {
    fn prepare(&mut self, input: crate::node_service_hosting::native::actor::Input<'_>) -> crate::error::Result<()> {
        super::prepare(self.signing, self.owner, input, self.logical_tick)
    }

    fn run(&mut self, effect: &ActorEffectIntent, member: &str, attempted: &mut bool) -> crate::error::Result<String> {
        let observed = match effect.kind {
            ActorEffectIntentKind::RestoreCheckpoint => {
                self.prepare(crate::node_service_hosting::native::actor::Input::CheckpointValue(member))?;
                let checkpoint = self.owner.actor_checkpoint_value(member, self.logical_tick)?;
                if effect.subject_ref.as_deref() != Some(checkpoint.value_ref.as_str()) {
                    return Err(super::invalid("actor restore differs from actual selected native checkpoint"));
                }
                let state = crate::addressable_actor::counter::State::decode(&checkpoint.bytes)?;
                if state.actor_key_ref != member {
                    return Err(super::invalid("native checkpoint targets another actor key"));
                }
                self.prepare(crate::node_service_hosting::native::actor::Input::Rewake(member))?;
                *attempted = true;
                self.owner.actor_rewake(member, self.logical_tick)?
            }
            ActorEffectIntentKind::StartRuntime => {
                self.prepare(crate::node_service_hosting::native::actor::Input::Rewake(member))?;
                *attempted = true;
                self.owner.actor_rewake(member, self.logical_tick)?
            }
            ActorEffectIntentKind::PersistCheckpoint => {
                self.prepare(crate::node_service_hosting::native::actor::Input::Checkpoint(member))?;
                *attempted = true;
                let observed = self.owner.actor_checkpoint(member, self.logical_tick)?;
                if observed.checkpoint_ref != effect.subject_ref {
                    return Err(super::invalid("actual native checkpoint differs from actor sleep plan"));
                }
                observed
            }
            ActorEffectIntentKind::StopRuntime => {
                self.prepare(crate::node_service_hosting::native::actor::Input::Drain(member))?;
                *attempted = true;
                self.owner.actor_drain(member, self.logical_tick)?
            }
            ActorEffectIntentKind::DeliverMessage => {
                return self.deliver_message(effect, member, attempted);
            }
            ActorEffectIntentKind::AcknowledgeDelivery => {
                return self.acknowledge(effect, member, attempted);
            }
            ActorEffectIntentKind::InvokeTimer
            | ActorEffectIntentKind::AcceptConnection
            | ActorEffectIntentKind::NotifyOperator
            | ActorEffectIntentKind::PublishStatus => {
                return Err(super::invalid("counter profile does not admit this wake/effect class"));
            }
        };
        Ok(crate::system_extension::canonical_instance_record(&observed)?.record_ref)
    }

    fn deliver_message(
        &mut self,
        effect: &ActorEffectIntent,
        member: &str,
        attempted: &mut bool,
    ) -> crate::error::Result<String> {
        let work = self.work.ok_or_else(|| super::invalid("actor delivery has no exact retained native work"))?;
        if effect.subject_ref.as_deref() != Some(work.presentation.work_ref.as_str()) {
            return Err(super::invalid("actor delivery item differs from retained native work"));
        }
        let readmission = crate::node_service_host::operation::Invocation::Readmit {
            member: member.to_string(),
            work_ref: work.presentation.work_ref.clone(),
        };
        self.signing.prepare(&self.owner.preview(&readmission, self.logical_tick)?)?;
        *attempted = true;
        let claim = self.owner.actor_readmit(member, &work.presentation.work_ref, self.logical_tick)?;
        let admitted_work = crate::node_service_hosting::DeliveryInput {
            member: work.member.clone(),
            presentation: work.presentation.clone(),
            claim_ref: Some(claim.claim_ref),
            ingress: work.ingress.clone(),
            logical_tick: self.logical_tick,
        };
        self.signing.prepare(&self.owner.actor_ingress_preview(&admitted_work)?)?;
        *attempted = true;
        let result = self.owner.actor_ingress(&admitted_work)?;
        if !matches!(result.dispatch, crate::system_extension::DispatchResult::Executed { .. })
            || result.effects != crate::node_service_hosting::NativeEffectDisposition::NotRequested
        {
            return Err(super::invalid("counter callback did not produce a terminal effect-free semantic result"));
        }
        self.signing.prepare(&self.owner.delivery_native_commit_preview(
            member,
            &work.ingress.request_ref,
            self.logical_tick,
        )?)?;
        *attempted = true;
        let proof = self.owner.delivery_native_commit(member, &work.ingress.request_ref, self.logical_tick)?;
        let input = crate::addressable_actor::counter::Add::decode(&work.ingress.payload.bytes)?;
        let state = crate::addressable_actor::counter::State::decode(&proof.state().bytes)?;
        if state.actor_key_ref != member
            || !state
                .completed
                .iter()
                .any(|done| done.request_id == input.request_id && done.increment == input.increment)
        {
            return Err(super::invalid("native durable state has no exact counter logical completion"));
        }
        Ok(proof.commit_ref().to_string())
    }

    fn acknowledge(
        &mut self,
        effect: &ActorEffectIntent,
        member: &str,
        attempted: &mut bool,
    ) -> crate::error::Result<String> {
        let request = self.acknowledgement.ok_or_else(|| super::invalid("actor completion has no acknowledgement"))?;
        let proof = self.commit.ok_or_else(|| super::invalid("actor completion has no native durable proof"))?;
        let molten_core::coordination_delivery::DeliveryOperation::Acknowledge { token } = &request.operation else {
            return Err(super::invalid("actor completion does not name an acknowledgement"));
        };
        if effect.subject_ref.as_deref() != Some(token.token_ref.as_str()) {
            return Err(super::invalid("actor acknowledgement token differs from effect"));
        }
        let delivery_member = &self.configuration.delivery_member;
        self.signing
            .prepare(&self.owner.delivery_preview(delivery_member, self.delivery, request, Some(proof))?)?;
        *attempted = true;
        match self.owner.delivery_acknowledge(delivery_member, member, self.delivery, request, proof)? {
            crate::addressable_actor::delivery::Acknowledgement::Committed { receipt_ref, status }
                if status.commit_confirmed() =>
            {
                Ok(receipt_ref)
            }
            crate::addressable_actor::delivery::Acknowledgement::AlreadyAcknowledged { state_ref, .. } => Ok(state_ref),
            crate::addressable_actor::delivery::Acknowledgement::Committed { .. } => {
                Err(super::invalid("actor delivery acknowledgement remains unknown"))
            }
        }
    }
}
