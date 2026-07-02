use super::*;

fn effect_response(effect: Effect, actor: String, sequence: u64, upper: Option<u64>, value: u64) -> Event {
    Event::EffectResponse {
        effect,
        actor,
        sequence,
        upper,
        value,
    }
}

impl RuntimeState {
    pub fn new(seed: u64) -> Self {
        Self {
            logical_time: 0,
            rng_state: seed.max(1),
            effect_sequence: 0,
            messages: OrderedSet::new(),
            assertions: OrderedSet::new(),
            observers: OrderedSet::new(),
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            logical_time: self.logical_time,
            rng_state: self.rng_state,
            effect_sequence: self.effect_sequence,
            messages: self.messages.clone(),
            assertions: self.assertions.clone(),
            observers: self.observers.clone(),
        }
    }

    pub fn apply_step(&mut self, step: &Step) -> Vec<Event> {
        match step {
            Step::Clock { actor } => {
                let sequence = self.next_effect_sequence();
                let request = Event::EffectRequest {
                    effect: Effect::Clock,
                    actor: actor.clone(),
                    sequence,
                    upper: None,
                };
                let value = self.local_clock_response_value();
                let response = effect_response(Effect::Clock, actor.clone(), sequence, None, value);
                vec![request, response]
            }
            Step::Random { actor, upper } => {
                let sequence = self.next_effect_sequence();
                let request = Event::EffectRequest {
                    effect: Effect::Random,
                    actor: actor.clone(),
                    sequence,
                    upper: Some(*upper),
                };
                let value = self.next_random(*upper);
                let response = effect_response(Effect::Random, actor.clone(), sequence, Some(*upper), value);
                vec![request, response]
            }
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => {
                let turn = self.begin_turn(step);
                self.commit_turn(turn)
            }
        }
    }

    pub fn begin_turn(&self, step: &Step) -> PendingTurn {
        let mut turn = PendingTurn::new();
        self.stage_step(&mut turn, step);
        turn
    }

    pub fn commit_turn(&mut self, turn: PendingTurn) -> Vec<Event> {
        for action in turn.actions {
            match action {
                TurnAction::Send(message) => {
                    self.messages.insert(message);
                }
                TurnAction::Observe(observer) => {
                    self.observers.insert(observer);
                }
                TurnAction::Assert(assertion) => {
                    self.assertions.insert(assertion);
                }
                TurnAction::Retract(assertion) => {
                    self.assertions.remove(&assertion);
                }
            }
        }
        turn.events
    }

    pub fn commit_turn_with_predicate_receipt(&mut self, turn: PendingTurn) -> Result<(Vec<Event>, PredicateReceipt)> {
        let before = self.snapshot();
        let mut preview = self.clone();
        let events = preview.commit_turn(turn.clone());
        let after = preview.snapshot();
        let receipt = evaluate_turn_transition(&before, &turn, &after, TurnOutcome::Committed)?;
        if receipt.decision == PredicateDecision::Pass {
            *self = preview;
            Ok((events, receipt))
        } else {
            Err(MoltenError::invalid_harness("runtime turn predicate denied commit"))
        }
    }

    pub fn rollback_turn(&self, _turn: PendingTurn, actor: impl Into<String>, reason: impl Into<String>) -> Vec<Event> {
        vec![Event::TurnRolledBack {
            actor: actor.into(),
            reason: reason.into(),
        }]
    }

    pub fn rollback_turn_with_predicate_receipt(
        &self,
        turn: PendingTurn,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<(Vec<Event>, PredicateReceipt)> {
        let before = self.snapshot();
        let receipt = evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Denied)?;
        let events = self.rollback_turn(turn, actor, reason);
        Ok((events, receipt))
    }

    pub fn cleanup_actor_scope(&mut self, actor: &str) -> Result<RuntimeScopeCleanup> {
        let mut assertion_refs = Vec::with_capacity(self.assertions.len());
        for assertion in self.assertions.iter().filter(|assertion| assertion.actor == actor) {
            assertion_refs.push(assertion.assertion_ref()?);
        }
        let mut observer_refs = Vec::with_capacity(self.observers.len());
        for observer in self.observers.iter().filter(|observer| observer.actor == actor) {
            observer_refs.push(observer.observer_ref()?);
        }
        let mut message_refs = Vec::with_capacity(self.messages.len());
        for message in self.messages.iter().filter(|message| message.from == actor || message.to == actor) {
            message_refs.push(message.message_ref()?);
        }
        assertion_refs.sort();
        observer_refs.sort();
        message_refs.sort();
        self.assertions.retain(|assertion| assertion.actor != actor);
        self.observers.retain(|observer| observer.actor != actor);
        self.messages.retain(|message| message.from != actor && message.to != actor);
        Ok(RuntimeScopeCleanup {
            actor: actor.to_owned(),
            assertion_refs,
            observer_refs,
            message_refs,
        })
    }

    pub fn begin_effect_for_step(&mut self, step: &Step) -> Option<Event> {
        match step {
            Step::Clock { actor } => Some(Event::EffectRequest {
                effect: Effect::Clock,
                actor: actor.clone(),
                sequence: self.next_effect_sequence(),
                upper: None,
            }),
            Step::Random { actor, upper } => Some(Event::EffectRequest {
                effect: Effect::Random,
                actor: actor.clone(),
                sequence: self.next_effect_sequence(),
                upper: Some(*upper),
            }),
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => None,
        }
    }

    pub fn apply_recorded_effect_response(&mut self, request: &Event, value: u64) -> Result<Event> {
        match request {
            Event::EffectRequest {
                effect: Effect::Clock,
                actor,
                sequence,
                upper,
            } => {
                self.logical_time = value + 1;
                Ok(effect_response(Effect::Clock, actor.clone(), *sequence, *upper, value))
            }
            Event::EffectRequest {
                effect: Effect::Random,
                actor,
                sequence,
                upper: Some(upper),
            } => {
                let _ignored_local_value = self.next_random(*upper);
                Ok(effect_response(Effect::Random, actor.clone(), *sequence, Some(*upper), value))
            }
            Event::EffectRequest {
                effect: Effect::Random, ..
            } => Err(MoltenError::invalid_harness("recorded random effect request missing upper bound")),
            _ => Err(MoltenError::invalid_harness("recorded effect response requires an effect request")),
        }
    }

    fn stage_step(&self, turn: &mut PendingTurn, step: &Step) {
        match step {
            Step::Send { from, to, body } => {
                let message = Message {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                };
                turn.events.push(Event::MessageDelivered {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                });
                turn.actions.push(TurnAction::Send(message));
            }
            Step::Observe { actor, pattern } => {
                let observer = Observer {
                    actor: actor.clone(),
                    pattern: pattern.clone(),
                };
                turn.events.push(Event::ObserveRegistered {
                    actor: actor.clone(),
                    pattern: pattern.clone(),
                });
                for assertion in self.assertions.iter().filter(|assertion| assertion.value == *pattern) {
                    turn.events.push(Event::AssertionObserved {
                        observer: actor.clone(),
                        owner: assertion.actor.clone(),
                        value: assertion.value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Observe(observer));
            }
            Step::Assert { actor, value } => {
                let assertion = Assertion {
                    actor: actor.clone(),
                    value: value.clone(),
                };
                turn.events.push(Event::AssertionCommitted {
                    actor: actor.clone(),
                    value: value.clone(),
                });
                for observer in self.observers.iter().filter(|observer| observer.pattern == *value) {
                    turn.events.push(Event::AssertionObserved {
                        observer: observer.actor.clone(),
                        owner: actor.clone(),
                        value: value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Assert(assertion));
            }
            Step::Retract { actor, value } => {
                let assertion = Assertion {
                    actor: actor.clone(),
                    value: value.clone(),
                };
                turn.events.push(Event::AssertionRetracted {
                    actor: actor.clone(),
                    value: value.clone(),
                });
                for observer in self.observers.iter().filter(|observer| observer.pattern == *value) {
                    turn.events.push(Event::AssertionRetractionObserved {
                        observer: observer.actor.clone(),
                        owner: actor.clone(),
                        value: value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Retract(assertion));
            }
            Step::Clock { .. } | Step::Random { .. } => {}
        }
    }

    fn local_clock_response_value(&mut self) -> u64 {
        let logical_time = self.logical_time;
        self.logical_time += 1;
        logical_time
    }

    fn next_effect_sequence(&mut self) -> u64 {
        let sequence = self.effect_sequence;
        self.effect_sequence += 1;
        sequence
    }

    fn next_random(&mut self, upper: u64) -> u64 {
        // Deterministic xorshift64* profile; not cryptographic and not ambient entropy.
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        let value = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        if upper == 0 { 0 } else { value % upper }
    }
}
