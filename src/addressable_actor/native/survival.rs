impl<C: crate::fabric_time::TimerClockAdapter> super::Consumer<'_, C> {
    pub(super) fn validate_survival(
        &mut self,
        operation: &molten_core::addressable_actor::ActorOperation,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        let molten_core::addressable_actor::ActorOperation::RecoverySucceeded {
            checkpoint_ref,
            restored_classes,
            durable_state_ref,
        } = operation
        else {
            return Ok(());
        };
        // Validate unsupported/runtime-only classes before protected materialization or callbacks.
        if restored_classes.iter().any(|class| {
            self.configuration.profile.survival.disposition(*class)
                != Some(molten_core::addressable_actor::SurvivalDisposition::Durable)
        }) {
            return Err(super::invalid("native actor cannot restore a runtime-only or unsupported survival class"));
        }
        let key = molten_core::addressable_actor::identify_actor_key(&self.configuration.actor_key);
        super::prepare(
            self.signing,
            self.owner,
            crate::node_service_hosting::native::actor::Input::Observe(&key),
            logical_tick,
        )?;
        let native = self.owner.actor_instance(&key, logical_tick)?;
        if native.lifecycle.phase != crate::system_extension::LifecyclePhase::Running
            || native.checkpoint_ref.as_ref() != Some(checkpoint_ref)
            || native.state_ref.as_ref() != Some(durable_state_ref)
        {
            return Err(super::invalid("native recovery claim differs from current durable checkpoint/state"));
        }
        super::prepare(
            self.signing,
            self.owner,
            crate::node_service_hosting::native::actor::Input::State(&key),
            logical_tick,
        )?;
        let materialized = self.owner.actor_state(&key, logical_tick)?;
        let state = crate::addressable_actor::counter::State::decode(&materialized.bytes)?;
        if state.actor_key_ref != key || materialized.value_ref != *durable_state_ref {
            return Err(super::invalid("native restored state does not bind the exact actor key"));
        }
        for class in restored_classes {
            match class {
                molten_core::addressable_actor::SurvivalClass::DurableState
                | molten_core::addressable_actor::SurvivalClass::CompletedSemanticEvents => {}
                molten_core::addressable_actor::SurvivalClass::Checkpoints => {
                    self.validate_checkpoint(&key, checkpoint_ref, &materialized, logical_tick)?;
                }
                molten_core::addressable_actor::SurvivalClass::MailboxEntries => {
                    if self.delivery.queue_state_ref(&self.configuration.queue_id)?.is_none() {
                        return Err(super::invalid("mailbox survival has no actual persisted delivery head"));
                    }
                }
                molten_core::addressable_actor::SurvivalClass::Processes
                | molten_core::addressable_actor::SurvivalClass::Streams
                | molten_core::addressable_actor::SurvivalClass::Sessions
                | molten_core::addressable_actor::SurvivalClass::PartialCallbacks
                | molten_core::addressable_actor::SurvivalClass::InFlightDeltas => {
                    return Err(super::invalid("unsupported native actor survival claim"));
                }
            }
        }
        Ok(())
    }

    fn validate_checkpoint(
        &mut self,
        key: &str,
        checkpoint_ref: &str,
        materialized: &crate::system_extension::NativeCallbackValue,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        super::prepare(
            self.signing,
            self.owner,
            crate::node_service_hosting::native::actor::Input::CheckpointValue(key),
            logical_tick,
        )?;
        let checkpoint = self.owner.actor_checkpoint_value(key, logical_tick)?;
        if checkpoint.value_ref != checkpoint_ref || checkpoint.bytes != materialized.bytes {
            return Err(super::invalid("native restored checkpoint bytes differ from exact semantic state"));
        }
        Ok(())
    }
}
