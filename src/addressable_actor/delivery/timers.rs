use molten_core::coordination_delivery::*;

struct Scheduled {
    intent: DeliveryTimerIntent,
    timer: crate::fabric_time::TimerState,
}

/// Actual bounded fabric timers, driven by the selected clock. These are not
/// a second durable journal: Local claims remain the recovery source.
pub(super) struct Scheduler<C: crate::fabric_time::TimerClockAdapter> {
    clock: C,
    profile: crate::fabric_time::AdmittedTimeProfile,
    context: Option<crate::fabric_time::ExtensionTimeContext>,
    service_id: Option<String>,
    next_sequence: u64,
    logical_anchor: u64,
    clock_anchor: u64,
    scheduled: std::collections::BTreeMap<String, Scheduled>,
}

impl<C: crate::fabric_time::TimerClockAdapter> Scheduler<C> {
    pub(super) fn new(clock: C, profile: crate::fabric_time::AdmittedTimeProfile) -> Self {
        Self {
            clock,
            profile,
            context: None,
            service_id: None,
            next_sequence: 1,
            logical_anchor: 0,
            clock_anchor: 0,
            scheduled: std::collections::BTreeMap::new(),
        }
    }

    pub(super) fn bind<E: crate::system_extension::SystemExtensionExecutor>(
        &mut self,
        host: &crate::system_extension::Dispatcher<E>,
        logical_tick: u64,
    ) -> crate::error::Result<()> {
        self.clock_anchor = self.clock.now_ticks().map_err(super::problem)?;
        self.logical_anchor = logical_tick;
        self.context = Some(crate::fabric_time::ExtensionTimeContext::from_host(host));
        self.service_id = Some(host.manifest().manifest().service_id.clone());
        Ok(())
    }

    pub(super) fn restore(&mut self, state: &DeliveryState) -> crate::error::Result<()> {
        for active in state.in_flight.values() {
            let token = &active.token;
            self.apply(&DeliveryTimerIntent {
                kind: DeliveryTimerIntentKind::ScheduleLeaseExpiry,
                timer_id: identify_delivery_timer(TimerIdentityInput {
                    kind: DeliveryTimerIntentKind::ScheduleLeaseExpiry,
                    item_ref: &token.item_ref,
                    delivery_id: Some(&token.delivery_id),
                    deadline_tick: token.visibility_deadline_tick,
                    service_generation: token.service_generation,
                    consistency_epoch: token.consistency_epoch,
                }),
                item_ref: token.item_ref.clone(),
                delivery_id: Some(token.delivery_id.clone()),
                deadline_tick: token.visibility_deadline_tick,
                service_generation: token.service_generation,
                consistency_epoch: token.consistency_epoch,
            })?;
        }
        Ok(())
    }

    pub(super) fn poll_due(&mut self) -> crate::error::Result<Vec<DeliveryTimerIntent>> {
        let now = self.clock.now_ticks().map_err(super::problem)?;
        let mut due = Vec::with_capacity(self.scheduled.len());
        for scheduled in self.scheduled.values_mut() {
            let transition =
                crate::fabric_time::poll_timer(&scheduled.timer, scheduled.intent.service_generation, now, 1)
                    .map_err(super::problem)?;
            scheduled.timer = transition.next;
            if transition.delivery_count == 1 {
                if due.len() >= super::MAX_REQUESTS {
                    return Err(super::invalid("actor due timer bound exhausted"));
                }
                due.push(scheduled.intent.clone());
            }
        }
        self.scheduled
            .retain(|_, scheduled| scheduled.timer.phase == crate::fabric_time::TimerPhase::Scheduled);
        due.sort_by(|left, right| (left.deadline_tick, &left.timer_id).cmp(&(right.deadline_tick, &right.timer_id)));
        Ok(due)
    }

    pub(super) fn await_next(&mut self) -> crate::error::Result<Vec<DeliveryTimerIntent>> {
        let Some(deadline) = self.scheduled.values().map(|entry| entry.timer.next_deadline_ticks).min() else {
            return Ok(Vec::new());
        };
        let now = self.clock.now_ticks().map_err(super::problem)?;
        if now < deadline {
            self.clock.await_ticks(deadline).map_err(super::problem)?;
        }
        self.poll_due()
    }

    fn apply(&mut self, intent: &DeliveryTimerIntent) -> crate::error::Result<()> {
        use DeliveryTimerIntentKind::*;
        if matches!(intent.kind, CancelLeaseExpiry | CancelDeadLetterRetention) {
            let scheduled_kind = if intent.kind == CancelLeaseExpiry {
                ScheduleLeaseExpiry
            } else {
                ScheduleDeadLetterRetention
            };
            let id = self.scheduled.iter().find_map(|(id, scheduled)| {
                let prior = &scheduled.intent;
                (prior.kind == scheduled_kind
                    && prior.item_ref == intent.item_ref
                    && prior.delivery_id == intent.delivery_id
                    && prior.deadline_tick == intent.deadline_tick
                    && prior.service_generation == intent.service_generation
                    && prior.consistency_epoch == intent.consistency_epoch)
                    .then(|| id.clone())
            });
            // An already fired timer is absent; cancellation does not invent a schedule.
            if let Some(id) = id {
                let scheduled = self.scheduled.get(&id).ok_or_else(|| super::invalid("actor timer disappeared"))?;
                crate::fabric_time::cancel_timer(&scheduled.timer, intent.service_generation)
                    .map_err(super::problem)?;
                self.scheduled.remove(&id);
            }
            return Ok(());
        }
        if let Some(existing) = self.scheduled.get(&intent.timer_id) {
            if existing.intent == *intent {
                return Ok(());
            }
            return Err(super::invalid("actor timer identity was rebound"));
        }
        if self.scheduled.len() >= super::MAX_REQUESTS {
            return Err(super::invalid("actor timer bound exhausted"));
        }
        let context = self.context.as_ref().ok_or_else(|| super::invalid("actor timer has no native host binding"))?;
        let service_id = self
            .service_id
            .as_ref()
            .ok_or_else(|| super::invalid("actor timer host service is absent"))?
            .clone();
        let sequence = self.next_sequence;
        let next_sequence = sequence.checked_add(1).ok_or_else(|| super::invalid("actor timer sequence overflow"))?;
        // Logical positions are not monotonic/virtual clock instants. Only the
        // explicitly admitted remaining duration crosses the domain boundary.
        let remaining = intent.deadline_tick.saturating_sub(self.logical_anchor);
        let deadline_ticks = self
            .clock_anchor
            .checked_add(remaining)
            .ok_or_else(|| super::invalid("actor timer clock deadline overflow"))?;
        let timer = context.schedule_timer(
            &self.profile,
            self.scheduled.len() as u64,
            &crate::fabric_time::TimerScheduleRequest {
                profile_ref: self.profile.profile_ref.clone(),
                key: crate::fabric_time::TimerKey {
                    service_id,
                    generation: intent.service_generation,
                    sequence,
                },
                domain: self.clock.timer_domain(),
                deadline_ticks,
                kind: crate::fabric_time::TimerKind::OneShot,
                ordering_key: sequence,
                coalescing: crate::fabric_time::TimerCoalescingPolicy::DeliverEach { max_catch_up: 1 },
                lateness: crate::fabric_time::TimerLatenessPolicy::DeliverRegardless,
                overload: crate::fabric_time::TimerOverloadPolicy::RejectAndRetain,
                resource_charge: crate::fabric_time::TimerResourceCharge::single_slot(),
            },
        )?;
        self.scheduled.insert(intent.timer_id.clone(), Scheduled {
            intent: intent.clone(),
            timer,
        });
        self.next_sequence = next_sequence;
        Ok(())
    }
}

impl<C: crate::fabric_time::TimerClockAdapter> crate::coordination_delivery::DeliveryTimerPort for Scheduler<C> {
    fn apply_timer_intents(
        &mut self,
        intents: &[DeliveryTimerIntent],
    ) -> crate::coordination_delivery::PortResult<crate::coordination_delivery::TimerObservation> {
        let mut observation = crate::coordination_delivery::TimerObservation::empty();
        for intent in intents {
            match self.apply(intent) {
                Ok(()) => observation.accepted_timer_refs.push(intent.timer_id.clone()),
                Err(_) => observation.failed_timer_refs.push(intent.timer_id.clone()),
            }
        }
        Ok(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination_delivery::DeliveryTimerPort;

    fn scheduler() -> Scheduler<crate::fabric_time::VirtualClockAdapter> {
        let profile = crate::addressable_actor::delivery::tests::time_profile();
        let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("virtual clock");
        let mut timers = Scheduler::new(clock, profile.clone());
        timers.context =
            Some(crate::fabric_time::ExtensionTimeContext::from_test_snapshot("actor-timer", 1, &profile, vec![]));
        timers.service_id = Some("actor-timer".to_string());
        timers
    }

    fn intent(index: u64) -> DeliveryTimerIntent {
        DeliveryTimerIntent {
            kind: DeliveryTimerIntentKind::ScheduleLeaseExpiry,
            timer_id: crate::preserves_rail::content_ref_from_bytes(&index.to_le_bytes()),
            item_ref: crate::preserves_rail::content_ref_from_bytes(b"request"),
            delivery_id: Some(format!("delivery-{index}")),
            deadline_tick: 5,
            service_generation: 1,
            consistency_epoch: 1,
        }
    }

    #[test]
    fn accepted_timer_really_fires_once_on_the_selected_clock() {
        let mut timers = scheduler();
        let scheduled = intent(1);
        let observed = timers.apply_timer_intents(std::slice::from_ref(&scheduled)).expect("schedule");
        assert_eq!(observed.accepted_timer_refs, vec![scheduled.timer_id.clone()]);
        assert!(timers.poll_due().expect("not due").is_empty());
        assert_eq!(timers.await_next().expect("actual clock deadline"), vec![scheduled]);
        assert!(timers.poll_due().expect("already fired").is_empty());
    }

    #[test]
    fn cancellation_matches_the_actual_schedule_not_the_cancel_identifier() {
        let mut timers = scheduler();
        let scheduled = intent(1);
        timers.apply_timer_intents(std::slice::from_ref(&scheduled)).expect("schedule");
        let mut cancel = scheduled;
        cancel.kind = DeliveryTimerIntentKind::CancelLeaseExpiry;
        cancel.timer_id = crate::preserves_rail::content_ref_from_bytes(b"cancel-id");
        assert!(timers.apply_timer_intents(&[cancel]).expect("cancel").failed_timer_refs.is_empty());
        assert!(timers.await_next().expect("cancelled queue").is_empty());
    }

    #[test]
    fn exhausted_or_unbound_schedules_are_not_accepted() {
        let mut timers = scheduler();
        timers.profile.max_timers = 1;
        assert_eq!(
            timers
                .apply_timer_intents(&[intent(1), intent(2)])
                .expect("bounded schedule")
                .failed_timer_refs
                .len(),
            1
        );
        let profile = timers.profile.clone();
        let clock = crate::fabric_time::VirtualClockAdapter::new(&profile, 0, 0).expect("clock");
        let mut unbound = Scheduler::new(clock, profile);
        let outcome = unbound.apply_timer_intents(&[intent(3)]).expect("reported denial");
        assert!(outcome.accepted_timer_refs.is_empty());
        assert_eq!(outcome.failed_timer_refs.len(), 1);
    }
}
