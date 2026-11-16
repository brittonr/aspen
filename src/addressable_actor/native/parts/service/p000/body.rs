impl<'a, C: crate::fabric_time::TimerClockAdapter> super::Consumer<'a, C> {
    pub fn open(input: super::Open<'a, '_, C>) -> crate::error::Result<Self> {
        let super::Open { storage, engine_epoch, configuration, owner, placement, delivery, signing } = input;
        if !validate_actor_key(&configuration.actor_key).is_empty()
            || !validate_addressable_actor_profile(&configuration.profile).is_empty()
        {
            return Err(super::invalid("native actor configuration is invalid"));
        }
        if configuration.profile.delivery_profile_ref != delivery.delivery_manifest_ref()
            || configuration.profile.time_profile_ref != delivery.time_profile_ref()
        {
            return Err(super::invalid("native actor delivery/time profile was silently substituted"));
        }
        let store = crate::addressable_actor::Local::open(storage, engine_epoch).map_err(problem)?;
        Ok(Self {
            configuration,
            owner,
            placement,
            delivery,
            signing,
            store,
            engine_epoch,
        })
    }

    /// Bootstrap actor metadata only from the actual admitted drained native checkpoint.
    /// This does not install a manifest, choose placement or start a callback.
    pub fn initialize(&mut self, logical_tick: u64) -> crate::error::Result<crate::addressable_actor::PublishedState> {
        let key = identify_actor_key(&self.configuration.actor_key);
        if self.store.load(&key).map_err(problem)?.is_some() {
            return Err(super::invalid("actor is already initialized"));
        }
        let input_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("native-actor-initialize-v1", vec![
                crate::preserves_rail::string(&key),
                crate::preserves_rail::string(identify_addressable_actor_profile(&self.configuration.profile)),
            ]))?;
        let observed = super::binding::observe(
            self.owner,
            self.signing,
            self.placement,
            super::binding::Input {
                configuration: self.configuration,
                input_ref: &input_ref,
                logical_tick,
            },
        )?;
        if observed.instance.lifecycle.phase != crate::system_extension::LifecyclePhase::Drained {
            return Err(super::invalid("native actor must be checkpointed and drained before metadata initialization"));
        }
        super::prepare(
            self.signing,
            self.owner,
            crate::node_service_hosting::native::actor::Input::CheckpointValue(&key),
            logical_tick,
        )?;
        let checkpoint = self.owner.actor_checkpoint_value(&key, logical_tick)?;
        let counter = crate::addressable_actor::counter::State::decode(&checkpoint.bytes)?;
        if counter.actor_key_ref != key
            || observed.instance.checkpoint_ref.as_deref() != Some(checkpoint.value_ref.as_str())
        {
            return Err(super::invalid("native checkpoint does not bind the selected actor key"));
        }
        let mut state = ActorState::dormant(
            &key,
            identify_addressable_actor_profile(&self.configuration.profile),
            &observed.instance.manifest_ref,
            &observed.assignment.assignment_ref,
            observed.instance.lifecycle.generation.get(),
        );
        state.checkpoint_ref = Some(checkpoint.value_ref.clone());
        state.durable_state_ref = Some(checkpoint.value_ref);
        let next = crate::addressable_actor::PublishedState::from_state(state);
        let result = self
            .store
            .compare_and_commit(&crate::addressable_actor::CommitRequest {
                actor_key_ref: key,
                expected: crate::addressable_actor::ExpectedState {
                    state_ref: None,
                    revision: 0,
                },
                next: next.clone(),
                requested_engine_epoch: self.engine_epoch,
            })
            .map_err(problem)?;
        if result.disposition != crate::addressable_actor::CommitDisposition::Applied
            || !result.currentness.is_current()
            || result.durability != crate::addressable_actor::DurabilityOutcome::Durable
        {
            return Err(super::invalid("native actor initialization was not confirmed current and durable"));
        }
        Ok(next)
    }

    pub fn fence(&mut self, logical_tick: u64) -> crate::error::Result<super::Fence> {
        let key = identify_actor_key(&self.configuration.actor_key);
        let current =
            self.store.load(&key).map_err(problem)?.ok_or_else(|| super::invalid("actor is not initialized"))?;
        let observed = super::binding::observe(
            self.owner,
            self.signing,
            self.placement,
            super::binding::Input {
                configuration: self.configuration,
                input_ref: &current.state_ref,
                logical_tick,
            },
        )?;
        Ok(super::Fence {
            actor_key_ref: key,
            placement_ref: current.state.placement_ref,
            generation: current.state.extension_generation,
            incarnation: observed.instance.lifecycle.incarnation.get(),
            lifecycle_sequence: current.state.lifecycle_sequence,
            expected: crate::addressable_actor::ExpectedState {
                state_ref: Some(current.state_ref),
                revision: current.revision,
            },
        })
    }

    pub fn apply(&mut self, input: super::Apply<'_>) -> crate::error::Result<crate::addressable_actor::Outcome> {
        let super::Apply { fence, operation_id, mut operation, logical_tick, work, acknowledgement } = input;
        let input_ref = operation_input(fence, operation_id, &operation)?;
        let observed = super::binding::observe(
            self.owner,
            self.signing,
            self.placement,
            super::binding::Input {
                configuration: self.configuration,
                input_ref: &input_ref,
                logical_tick,
            },
        )?;
        if observed.instance.lifecycle.incarnation.get() != fence.incarnation {
            return Err(super::invalid("stale native actor incarnation"));
        }
        if let ActorOperation::IdleSleep {
            pending_mailbox_items,
            unresolved_effects,
            ..
        } = &mut operation
        {
            *pending_mailbox_items = self.delivery.pending_items(&self.configuration.queue_id)?;
            *unresolved_effects = u32::try_from(observed.instance.unresolved.len())
                .map_err(|_| super::invalid("native unresolved operation count exceeds actor bound"))?;
        }
        let host = observed.host(self.configuration);
        self.validate_survival(&operation, logical_tick)?;
        let proof = self.validate_operation(&operation, work, acknowledgement)?;
        let request = ActorRequest {
            schema: ACTOR_REQUEST_SCHEMA.to_string(),
            operation_id: operation_id.to_string(),
            actor_key_ref: fence.actor_key_ref.clone(),
            placement_ref: fence.placement_ref.clone(),
            extension_generation: fence.generation,
            expected_lifecycle_sequence: fence.lifecycle_sequence,
            logical_tick,
            admission: admission(self.configuration, &host, fence, observed),
            operation,
        };
        let mut effects = super::effects::Executor {
            configuration: self.configuration,
            owner: self.owner,
            placement: self.placement,
            delivery: self.delivery,
            signing: self.signing,
            prepared_incarnation: None,
            work,
            acknowledgement,
            commit: proof.as_ref(),
            logical_tick,
        };
        let outcome = crate::addressable_actor::apply_request(&mut self.store, &mut effects, &mut Status, &crate::addressable_actor::Request {
            profile: &self.configuration.profile,
            actor_key: &self.configuration.actor_key,
            host_binding: &host,
            expected: fence.expected.clone(),
            request: &request,
            requested_engine_epoch: self.engine_epoch,
        })
        .map_err(problem)?;
        if crate::addressable_actor::public_outcome(&outcome) == crate::addressable_actor::PublicOutcome::Applied {
            let work = work.ok_or_else(|| super::invalid("successful completion lost original native ingress"))?;
            let acknowledgement = acknowledgement
                .ok_or_else(|| super::invalid("successful completion lost original acknowledgement"))?;
            self.store.retain_completion(&crate::addressable_actor::store::completion::Evidence {
                ingress_ref: work.ingress.request_ref.clone(),
                actor_key_ref: request.actor_key_ref.clone(),
                operation: request.operation,
                acknowledgement: acknowledgement.clone(),
                receipt_bytes: outcome.receipt.bytes.clone(),
                receipt_ref: outcome.receipt.receipt_ref.clone(),
            }).map_err(problem)?;
        }
        Ok(outcome)
    }
}
