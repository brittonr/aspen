impl<C: TimerClockAdapter> Bridge<C> {
    /// Enqueue/claim and fenced expiry only. Acknowledge cannot bypass Commit.
    pub fn apply<E: SystemExtensionExecutor>(
        &mut self,
        authority: &mut impl AuthorityPort,
        native: &crate::system_extension::Dispatcher<E>,
        request: &DeliveryRequest,
    ) -> crate::error::Result<crate::coordination_delivery::Outcome> {
        if !matches!(
            request.operation,
            DeliveryOperation::Enqueue { .. } | DeliveryOperation::Claim | DeliveryOperation::ExpireLease { .. }
        ) {
            return Err(invalid(
                "actor delivery operation requires native completion or is outside the consumer profile",
            ));
        }
        self.apply_admitted(authority, native, request, None)
    }

    pub fn acknowledge<E: SystemExtensionExecutor, P: crate::fabric_execution::ExecutionFabricPort>(
        &mut self,
        authority: &mut impl AuthorityPort,
        delivery_host: &crate::system_extension::Dispatcher<E>,
        actor_host: &crate::system_extension::Dispatcher<crate::system_extension::Process<P, crate::system_extension::Durable>>,
        request: &DeliveryRequest,
        commit: &Commit,
    ) -> crate::error::Result<Acknowledgement> {
        let DeliveryOperation::Acknowledge { token } = &request.operation else {
            return Err(invalid("native completion requires an acknowledge request"));
        };
        self.binding(delivery_host)?;
        let published = self
            .local
            .load(&request.queue_id)
            .map_err(problem)?
            .ok_or_else(|| invalid("actor delivery queue is absent"))?;
        if exact_ack_replay(&published.state, request, token) {
            return Ok(Acknowledgement::AlreadyAcknowledged {
                state_ref: published.state_ref,
                delivery_id: token.delivery_id.clone(),
            });
        }
        let active = published
            .state
            .in_flight
            .get(&token.item_ref)
            .ok_or_else(|| invalid("actor delivery has no active token"))?;
        if active.token != *token || token.queue_id != request.queue_id {
            return Err(invalid("actor delivery token is not the exact current claim"));
        }
        commit.validate(actor_host, &active.item.content_ref, actor_host.state().generation.get())?;
        if !request.evidence_refs.iter().any(|reference| reference == commit.event_ref())
            || !request.evidence_refs.iter().any(|reference| reference == commit.commit_ref())
            || !request.evidence_refs.contains(&commit.state().value_ref)
        {
            return Err(invalid("actor acknowledgement must bind native event, state and durable journal commit"));
        }
        self.apply_admitted(authority, delivery_host, request, Some(commit)).map(|outcome| {
            Acknowledgement::Committed {
                receipt_ref: outcome.receipt.receipt_ref,
                status: outcome.receipt.status,
            }
        })
    }

    /// Poll actual scheduled fabric timers. The caller handles returned expiry intents
    /// through an independently authorized ExpireLease request; polling grants no authority.
    pub fn poll_due(&mut self) -> crate::error::Result<Vec<DeliveryTimerIntent>> {
        self.timers.poll_due()
    }

    pub fn await_next(&mut self) -> crate::error::Result<Vec<DeliveryTimerIntent>> {
        self.timers.await_next()
    }

    /// Canonical observation only: no publication durability is claimed.
    pub fn observed_status_ref(&self) -> Option<&str> {
        self.status.observed_ref.as_deref()
    }

    fn binding<E: SystemExtensionExecutor>(
        &self,
        native: &crate::system_extension::Dispatcher<E>,
    ) -> crate::error::Result<crate::coordination_delivery::BindingFacts> {
        crate::coordination_delivery::project_binding(
            native.manifest(),
            native.state().generation.get(),
            native.state().phase == crate::system_extension::LifecyclePhase::Running,
            &self.manifest,
        )
        .map_err(problem)
    }

    fn apply_admitted<E: SystemExtensionExecutor>(
        &mut self,
        authority: &mut impl AuthorityPort,
        native: &crate::system_extension::Dispatcher<E>,
        request: &DeliveryRequest,
        commit: Option<&Commit>,
    ) -> crate::error::Result<crate::coordination_delivery::Outcome> {
        let binding = self.binding(native)?;
        let observed = self.local.load(&request.queue_id).map_err(problem)?;
        if let DeliveryOperation::Enqueue { item_ref, .. } = &request.operation
            && let Some(published) = &observed
        {
            let state = &published.state;
            let count = state.ready.len() + state.in_flight.len() + state.dead_letter.len() + state.completed.len();
            if count >= MAX_REQUESTS
                && !state.ready.contains_key(item_ref)
                && !state.in_flight.contains_key(item_ref)
                && !state.dead_letter.contains_key(item_ref)
                && !state.completed.contains_key(item_ref)
            {
                return Err(invalid("actor consumer request bound exhausted"));
            }
        }
        let expected = observed.as_ref().map_or(
            crate::coordination_delivery::ExpectedState {
                state_ref: None,
                revision: 0,
            },
            |published| crate::coordination_delivery::ExpectedState {
                state_ref: Some(published.state_ref.clone()),
                revision: published.revision,
            },
        );
        let preview = self.preview(native, request, commit)?;
        let admission = authority.admit(&preview)?;
        if !super::ports::valid_reference(&admission) {
            return Err(invalid("actor delivery current authority returned an invalid admission reference"));
        }
        self.timers.bind(native, request.logical_tick)?;
        if let Some(published) = &observed {
            self.timers.restore(&published.state)?;
        }
        crate::coordination_delivery::apply_request(&mut self.local, &mut self.timers, &mut self.status, &crate::coordination_delivery::Request {
            manifest: &self.manifest,
            policy: &self.policy,
            time_profile: &self.time_profile,
            host_binding: &binding,
            expected,
            request,
        })
        .map_err(problem)
    }
}
