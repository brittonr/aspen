impl<A: AuthorityPort, R: RuntimePort> crate::node_service_hosting::Host<A, R> {
    /// Project the exact current native instance and Host state without granting authority.
    pub fn actor_preview(
        &self,
        input: Input<'_>,
        logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
        let (operation, member) = input.parts();
        let instance = self.actor_current(member, logical_tick)?;
        let input_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-native-actor-input-v1", vec![
                crate::preserves_rail::string(operation),
                crate::system_extension::canonical_instance_record(&instance)?.value,
            ]))?;
        self.request(operation, member, &input_ref, logical_tick)
    }

    /// Observe the journal-backed instance only after exact current authorization.
    pub fn actor_instance(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.actor_admit(Input::Observe(member), logical_tick)?;
        self.actor_current(member, logical_tick)
    }

    /// Checkpoint a running actor through the durable Host lifecycle intent driver.
    pub fn actor_checkpoint(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.actor_require_running(member, logical_tick)?;
        self.actor_transition(Input::Checkpoint(member), &[crate::node_service_hosting::RuntimeAction::Checkpoint], logical_tick)
    }

    /// Persist checkpoint then drain. No Stop callback or terminal actor state is introduced.
    pub fn actor_sleep(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.actor_require_running(member, logical_tick)?;
        self.actor_transition(Input::Sleep(member), &[crate::node_service_hosting::RuntimeAction::Checkpoint, crate::node_service_hosting::RuntimeAction::Drain], logical_tick)
    }

    /// Drain an already checkpointed actor without repeating its checkpoint callback.
    pub fn actor_drain(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        self.actor_require_running(member, logical_tick)?;
        if self.actor_current(member, logical_tick)?.checkpoint_ref.is_none() {
            return Err(crate::node_service_hosting::invalid("actor drain requires its selected durable checkpoint"));
        }
        self.actor_transition(Input::Drain(member), &[crate::node_service_hosting::RuntimeAction::Drain], logical_tick)
    }

    /// Recover a drained actor; an already recovered running instance is observed, not activated
    /// twice.
    pub fn actor_rewake(
        &mut self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let current = self.actor_current(member, logical_tick)?;
        if current.checkpoint_ref.is_none() || current.state_ref.is_none() || current.evidence_refs.is_empty() {
            return Err(crate::node_service_hosting::invalid("native actor has no durable checkpoint/state evidence"));
        }
        match current.lifecycle.phase {
            crate::system_extension::LifecyclePhase::Running if current.is_accepting_ingress => {
                self.actor_admit(Input::Rewake(member), logical_tick)?;
                Ok(current)
            }
            crate::system_extension::LifecyclePhase::Drained if !current.is_accepting_ingress => {
                self.actor_transition(Input::Rewake(member), &[crate::node_service_hosting::RuntimeAction::Recover], logical_tick)
            }
            crate::system_extension::LifecyclePhase::Absent
            | crate::system_extension::LifecyclePhase::Installed
            | crate::system_extension::LifecyclePhase::Admitted
            | crate::system_extension::LifecyclePhase::Initializing
            | crate::system_extension::LifecyclePhase::Initialized
            | crate::system_extension::LifecyclePhase::Starting
            | crate::system_extension::LifecyclePhase::Running
            | crate::system_extension::LifecyclePhase::Checkpointing
            | crate::system_extension::LifecyclePhase::Recovering
            | crate::system_extension::LifecyclePhase::Draining
            | crate::system_extension::LifecyclePhase::Drained
            | crate::system_extension::LifecyclePhase::Failed
            | crate::system_extension::LifecyclePhase::Restarting
            | crate::system_extension::LifecyclePhase::Upgrading
            | crate::system_extension::LifecyclePhase::RollingBack
            | crate::system_extension::LifecyclePhase::ShuttingDown
            | crate::system_extension::LifecyclePhase::Quarantined
            | crate::system_extension::LifecyclePhase::Stopped
            | crate::system_extension::LifecyclePhase::Removed => {
                Err(crate::node_service_hosting::invalid("native actor cannot reawake from its current phase"))
            }
        }
    }

    /// Deliver only previously retained and admitted Host work, preserving callback/effect
    /// uncertainty. Submit/readmit and signing previews remain the existing Host work APIs.
    pub fn actor_ingress(
        &mut self,
        input: &crate::node_service_hosting::DeliveryInput,
    ) -> crate::error::Result<crate::node_service_hosting::RuntimeDelivery> {
        self.actor_require_running(&input.member, input.logical_tick)?;
        self.deliver(input)
    }

    fn actor_current(
        &self,
        member: &str,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let group = self.member_group(member)?;
        self.available_group(&group, logical_tick)?;
        let current = self.runtime.instance(member)?;
        if self.state.members.get(member) != Some(&current) || !current.unresolved.is_empty() {
            return Err(crate::node_service_hosting::invalid("native actor is stale or has unresolved operations"));
        }
        Ok(current)
    }

    fn actor_require_running(&self, member: &str, logical_tick: u64) -> crate::error::Result<()> {
        let current = self.actor_current(member, logical_tick)?;
        if current.lifecycle.phase != crate::system_extension::LifecyclePhase::Running || !current.is_accepting_ingress
        {
            return Err(crate::node_service_hosting::invalid("native actor is not accepting running ingress"));
        }
        Ok(())
    }

    fn actor_admit(&mut self, input: Input<'_>, logical_tick: u64) -> crate::error::Result<()> {
        let request = self.actor_preview(input, logical_tick)?;
        self.admit(&request.operation, &request.subject, &request.input_ref, logical_tick)?;
        Ok(())
    }

    fn actor_transition(
        &mut self,
        input: Input<'_>,
        actions: &[crate::node_service_hosting::RuntimeAction],
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let (operation, member) = input.parts();
        let request = self.actor_preview(input, logical_tick)?;
        let admission = self.admit(operation, member, &request.input_ref, logical_tick)?;
        let group_id = self.member_group(member)?;
        let pending = actions
            .iter()
            .map(|action| self.action(member, action.clone()))
            .collect::<crate::error::Result<Vec<_>>>()?;
        let mut next = self.state.clone();
        crate::node_service_hosting::groups::fence(&mut next, member);
        let group = next
            .groups
            .get_mut(&group_id)
            .ok_or_else(|| crate::node_service_hosting::invalid("unknown actor group"))?;
        group.pending = pending;
        group.last_tick = logical_tick;
        group.admission_ref = admission;
        self.commit(next)?;
        self.drive(&group_id, logical_tick)?;
        self.actor_current(member, logical_tick)
    }
}

