impl<C: TimerClockAdapter> Bridge<C> {
    pub fn new(
        local: crate::coordination_delivery::Local,
        manifest: DeliveryManifest,
        policy: DeliveryPolicy,
        time_profile: crate::fabric_time::AdmittedTimeProfile,
        clock: C,
    ) -> crate::error::Result<Self> {
        validate_delivery_manifest(&manifest, &policy).map_err(problem)?;
        let time_admission_ref = admit_time_profile(&time_profile)?;
        if time_profile.profile_ref != manifest.time_profile_ref
            || clock.profile_ref() != time_profile.profile_ref
            || !time_profile.supported_domains.contains(&clock.timer_domain())
        {
            return Err(invalid("actor delivery clock does not match admitted time profile"));
        }
        let timers = timers::Scheduler::new(clock, time_profile.clone());
        Ok(Self {
            local,
            manifest,
            policy,
            time_profile,
            time_admission_ref,
            timers,
            status: ObservedStatus::default(),
        })
    }

    /// Exact current request for independently provisioned Current/AuthorityPort signing.
    /// A preview is not permission and is invalidated by any intervening delivery mutation.
    pub fn preview<E: SystemExtensionExecutor>(
        &self,
        native: &crate::system_extension::Dispatcher<E>,
        request: &DeliveryRequest,
        commit: Option<&Commit>,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        self.binding(native)?;
        let observed = self.local.load(&request.queue_id).map_err(problem)?;
        let state_ref = observed.as_ref().map_or_else(
            || {
                identify_delivery_state(&DeliveryState::empty(
                    &request.queue_id,
                    &self.manifest.policy_ref,
                    request.service_generation,
                    request.consistency_epoch,
                ))
            },
            |published| published.state_ref.clone(),
        );
        let input = crate::preserves_rail::record("actor-coordination-delivery-input-v1", vec![
            crate::preserves_rail::string(identify_delivery_manifest(&self.manifest)),
            crate::preserves_rail::string(identify_delivery_policy(&self.policy)),
            crate::preserves_rail::string(&self.time_profile.profile_ref),
            crate::preserves_rail::string(&self.time_admission_ref),
            crate::preserves_rail::string(native.manifest().manifest_ref()),
            crate::preserves_rail::string(identify_delivery_request(request)),
            commit.map_or_else(|| crate::preserves_rail::record("none", vec![]), |commit| commit.binding_value()),
        ]);
        Ok(crate::node_service_hosting::OperationRequest {
            operation: format!("actor-delivery-{}", request.operation.kind()),
            subject: request.queue_id.clone(),
            logical_tick: request.logical_tick,
            state_ref,
            input_ref: crate::preserves_rail::canonical_hash(&input)?,
        })
    }

    pub fn delivery_manifest_ref(&self) -> String {
        identify_delivery_manifest(&self.manifest)
    }

    pub fn time_profile_ref(&self) -> &str {
        &self.time_profile.profile_ref
    }

    pub fn active_token(&self, queue_id: &str, item_ref: &str) -> crate::error::Result<Option<DeliveryToken>> {
        Ok(self
            .local
            .load(queue_id)
            .map_err(problem)?
            .and_then(|published| published.state.in_flight.get(item_ref).map(|active| active.token.clone())))
    }

    /// Observe exact ready work, not admission or permission to claim it. The current
    /// fenced claim still passes through native delivery authorization and Local CAS.
    pub(crate) fn retained_enqueue(&self, request: &DeliveryRequest) -> crate::error::Result<bool> {
        let DeliveryOperation::Enqueue { item_ref, content_ref, metadata_ref, metadata_bytes } = &request.operation else {
            return Err(invalid("retained enqueue observation requires an enqueue request"));
        };
        let Some(published) = self.local.load(&request.queue_id).map_err(problem)? else {
            return Ok(false);
        };
        let state = &published.state;
        if state.in_flight.iter().any(|(key, row)| key == item_ref || row.item.item_ref == *item_ref)
            || state.completed.iter().any(|(key, row)| key == item_ref || row.item.item_ref == *item_ref)
            || state.dead_letter.iter().any(|(key, row)| key == item_ref || row.item.item_ref == *item_ref)
        {
            return Err(invalid("retained counter delivery is not exclusively queued work"));
        }
        let mut rows = state.ready.iter().filter(|(key, row)| *key == item_ref || row.item.item_ref == *item_ref);
        let Some((key, ready)) = rows.next() else {
            return Ok(false);
        };
        let item = &ready.item;
        if rows.next().is_some()
            || key != item_ref
            || item.item_ref != *item_ref
            || item.content_ref != *content_ref
            || item.metadata_ref != *metadata_ref
            || item.metadata_bytes != *metadata_bytes
            || item.policy_ref != self.manifest.policy_ref
            || state.policy_ref != self.manifest.policy_ref
            || state.service_generation != request.service_generation
            || state.consistency_epoch != request.consistency_epoch
        {
            return Err(invalid("retained counter delivery differs from the exact queued request"));
        }
        Ok(true)
    }

    /// Pure claim preview. Subsequent authorization still binds the exact queue state.
    pub fn claimable_item(&self, request: &DeliveryRequest) -> crate::error::Result<Option<String>> {
        if !matches!(request.operation, DeliveryOperation::Claim) {
            return Err(invalid("claim preview requires a claim request"));
        }
        let Some(published) = self.local.load(&request.queue_id).map_err(problem)? else {
            return Ok(None);
        };
        let transition = plan_delivery_transition(&DeliveryTransitionInput {
            manifest: &self.manifest,
            policy: &self.policy,
            time_profile: &self.time_profile,
            state: &published.state,
            request,
        });
        if transition.decision == DeliveryDecisionKind::Denied {
            return Err(invalid(format!("exact actor claim preview refused: {:?}", transition.issue)));
        }
        Ok(transition.token.map(|token| token.item_ref))
    }

    /// Runnable mailbox work: ready plus currently claimed items. Dead letters
    /// remain quarantined under delivery policy and do not wake a dormant actor.
    pub fn pending_items(&self, queue_id: &str) -> crate::error::Result<u32> {
        self.local
            .load(queue_id)
            .map_err(problem)?
            .as_ref()
            .map_or(Ok(0), |published| pending_items(&published.state))
    }

    pub fn queue_state_ref(&self, queue_id: &str) -> crate::error::Result<Option<String>> {
        Ok(self.local.load(queue_id).map_err(problem)?.map(|published| published.state_ref))
    }

    /// Read the existing durable terminal row; this observation grants no authority.
    pub fn completed_delivery(
        &self,
        queue_id: &str,
        item_ref: &str,
    ) -> crate::error::Result<Option<CompletedDelivery>> {
        Ok(self
            .local
            .load(queue_id)
            .map_err(problem)?
            .and_then(|published| published.state.completed.get(item_ref).cloned()))
    }

    /// Observe the exact original acknowledgement without applying it again.
    pub(crate) fn completed_acknowledgement(
        &self,
        request: &DeliveryRequest,
    ) -> crate::error::Result<Option<CompletedDelivery>> {
        let DeliveryOperation::Acknowledge { token } = &request.operation else {
            return Ok(None);
        };
        let Some(published) = self.local.load(&request.queue_id).map_err(problem)? else {
            return Ok(None);
        };
        if token.queue_id != request.queue_id || !exact_ack_replay(&published.state, request, token) {
            return Ok(None);
        }
        Ok(published.state.completed.get(&token.item_ref).cloned())
    }
}
