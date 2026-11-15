impl<'a, C: crate::fabric_time::TimerClockAdapter> super::Consumer<'a, C> {
    fn validate_operation(
        &mut self,
        operation: &ActorOperation,
        work: Option<&crate::node_service_hosting::DeliveryInput>,
        acknowledgement: Option<&molten_core::coordination_delivery::DeliveryRequest>,
    ) -> crate::error::Result<Option<crate::addressable_actor::delivery::Commit>> {
        match operation {
            ActorOperation::Wake { reason: WakeReason::Message { delivery_item_ref, delivery_token_ref } } => {
                self.validate_message_claim(delivery_item_ref, delivery_token_ref, work, acknowledgement)?;
                Ok(None)
            }
            ActorOperation::CompleteDelivery { .. } => self.validate_completion(operation, work, acknowledgement).map(Some),
            ActorOperation::StartSucceeded { .. }
            | ActorOperation::IdleSleep { .. }
            | ActorOperation::BeginRecovery { .. }
            | ActorOperation::RecoverySucceeded { .. } => Ok(None),
            ActorOperation::Wake { reason: WakeReason::Timer { .. } | WakeReason::Connection { .. } | WakeReason::Operator { .. } }
            | ActorOperation::BeginDrain
            | ActorOperation::DrainSucceeded { .. }
            | ActorOperation::Stop
            | ActorOperation::Degrade { .. }
            | ActorOperation::RecoveryFailed { .. }
            | ActorOperation::RecordUnknownEffect { .. }
            | ActorOperation::ResolveUnknownEffect { .. } => Err(super::invalid("operation is outside the bounded native counter profile")),
        }
    }

    fn validate_message_claim(
        &self,
        delivery_item_ref: &str,
        delivery_token_ref: &str,
        work: Option<&crate::node_service_hosting::DeliveryInput>,
        acknowledgement: Option<&molten_core::coordination_delivery::DeliveryRequest>,
    ) -> crate::error::Result<()> {
        let key = identify_actor_key(&self.configuration.actor_key);
        let work = work.ok_or_else(|| super::invalid("counter wake requires exact native retained work"))?;
        let ack =
            acknowledgement.ok_or_else(|| super::invalid("counter wake requires its coordination claim"))?;
        let molten_core::coordination_delivery::DeliveryOperation::Acknowledge { token } = &ack.operation
        else {
            return Err(super::invalid("counter wake claim has no exact token"));
        };
        if work.member != key
            || work.presentation.work_ref != delivery_item_ref
            || ack.queue_id != self.configuration.queue_id
            || token.item_ref != delivery_item_ref
            || token.token_ref != delivery_token_ref
            || self.delivery.active_token(&ack.queue_id, delivery_item_ref)?.as_ref() != Some(token)
        {
            return Err(super::invalid("counter wake differs from actual current delivery/native claim"));
        }
        let input = crate::addressable_actor::counter::Add::decode(&work.ingress.payload.bytes)?;
        if input.actor_key_ref != key {
            return Err(super::invalid("counter payload targets wrong key"));
        }
        Ok(())
    }

    fn validate_completion(
        &mut self,
        operation: &ActorOperation,
        work: Option<&crate::node_service_hosting::DeliveryInput>,
        acknowledgement: Option<&molten_core::coordination_delivery::DeliveryRequest>,
    ) -> crate::error::Result<crate::addressable_actor::delivery::Commit> {
        let ActorOperation::CompleteDelivery { delivery_item_ref, delivery_token_ref, semantic_event_ref, semantic_commit_ref } = operation else {
            return Err(super::invalid("operation is not a native delivery completion"));
        };
        let key = identify_actor_key(&self.configuration.actor_key);
        let work = work.ok_or_else(|| super::invalid("counter completion requires original native ingress"))?;
        let ack =
            acknowledgement.ok_or_else(|| super::invalid("counter completion requires exact ack request"))?;
        let molten_core::coordination_delivery::DeliveryOperation::Acknowledge { token } = &ack.operation
        else {
            return Err(super::invalid("counter completion has no acknowledgement token"));
        };
        self.signing.prepare(&self.owner.delivery_native_commit_preview(
            &key,
            &work.ingress.request_ref,
            ack.logical_tick,
        )?)?;
        let proof = self.owner.delivery_native_commit(&key, &work.ingress.request_ref, ack.logical_tick)?;
        let input = crate::addressable_actor::counter::Add::decode(&work.ingress.payload.bytes)?;
        let state = crate::addressable_actor::counter::State::decode(&proof.state().bytes)?;
        if work.member != key
            || input.actor_key_ref != key
            || ack.queue_id != self.configuration.queue_id
            || token.item_ref != *delivery_item_ref
            || work.presentation.work_ref != *delivery_item_ref
            || token.queue_id != ack.queue_id
            || delivery_item_ref != &work.ingress.payload.value_ref
            || proof.content_ref() != work.ingress.payload.value_ref
            || token.token_ref != *delivery_token_ref
            || proof.event_ref() != semantic_event_ref
            || proof.commit_ref() != semantic_commit_ref
            || state.actor_key_ref != key
            || !state
                .completed
                .iter()
                .any(|done| done.request_id == input.request_id && done.increment == input.increment)
        {
            return Err(super::invalid("counter semantic completion lacks exact durable native proof"));
        }
        Ok(proof)
    }
}
