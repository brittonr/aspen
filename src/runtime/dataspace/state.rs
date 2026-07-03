use super::*;

const RANDOM_XORSHIFT_RIGHT_A: u32 = 12;
const RANDOM_XORSHIFT_LEFT_B: u32 = 25;
const RANDOM_XORSHIFT_RIGHT_C: u32 = 27;
const RANDOM_XORSHIFT_MULTIPLIER: u64 = 0x2545_F491_4F6C_DD1D;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRecordedEffectTransition {
    pub after: RuntimeSnapshot,
    pub response: Event,
}

fn effect_response(effect: Effect, actor: String, sequence: u64, upper: Option<u64>, value: u64) -> Event {
    Event::EffectResponse {
        effect,
        actor,
        sequence,
        upper,
        value,
    }
}

fn deterministic_random_step(rng_state: u64, upper: u64) -> (u64, u64) {
    let mut next_state = rng_state;
    next_state ^= next_state >> RANDOM_XORSHIFT_RIGHT_A;
    next_state ^= next_state << RANDOM_XORSHIFT_LEFT_B;
    next_state ^= next_state >> RANDOM_XORSHIFT_RIGHT_C;
    let mixed = next_state.wrapping_mul(RANDOM_XORSHIFT_MULTIPLIER);
    let value = if upper == 0 { 0 } else { mixed % upper };
    (next_state, value)
}

// r[impl molten.runtime_state_machine_proof.turn_commit_delta]
pub fn recorded_effect_response_transition(
    before: &RuntimeSnapshot,
    request: &Event,
    value: u64,
) -> Result<RuntimeRecordedEffectTransition> {
    match request {
        Event::EffectRequest {
            effect: Effect::Clock,
            actor,
            sequence,
            upper,
        } => {
            let mut after = before.clone();
            after.logical_time = value + 1;
            Ok(RuntimeRecordedEffectTransition {
                after,
                response: effect_response(Effect::Clock, actor.clone(), *sequence, *upper, value),
            })
        }
        Event::EffectRequest {
            effect: Effect::Random,
            actor,
            sequence,
            upper: Some(upper),
        } => {
            let mut after = before.clone();
            let (next_state, _ignored_local_value) = deterministic_random_step(after.rng_state, *upper);
            after.rng_state = next_state;
            Ok(RuntimeRecordedEffectTransition {
                after,
                response: effect_response(Effect::Random, actor.clone(), *sequence, Some(*upper), value),
            })
        }
        Event::EffectRequest {
            effect: Effect::Random, ..
        } => Err(MoltenError::invalid_harness("recorded random effect request missing upper bound")),
        _ => Err(MoltenError::invalid_harness("recorded effect response requires an effect request")),
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
        match self.apply_step_with_predicate_receipt(step) {
            Ok((events, _receipt)) => events,
            Err(error) => self.rollback_turn(
                PendingTurn::new(),
                step.primary_actor().to_string(),
                format!("runtime predicate denied step: {error}"),
            ),
        }
    }

    pub fn apply_step_with_predicate_receipt(&mut self, step: &Step) -> Result<(Vec<Event>, Option<PredicateReceipt>)> {
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
                Ok((vec![request, response], None))
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
                Ok((vec![request, response], None))
            }
            Step::Send { .. } | Step::Observe { .. } | Step::Assert { .. } | Step::Retract { .. } => {
                let turn = self.begin_turn(step);
                let (events, receipt) = self.commit_turn_with_predicate_receipt(turn)?;
                Ok((events, Some(receipt)))
            }
        }
    }

    pub fn begin_turn(&self, step: &Step) -> PendingTurn {
        let mut turn = PendingTurn::new();
        self.stage_step(&mut turn, step);
        turn
    }

    pub(crate) fn commit_turn(&mut self, turn: PendingTurn) -> Vec<Event> {
        let after = committed_turn_snapshot(&self.snapshot(), &turn);
        self.overwrite_from_snapshot(after);
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
        let transition = recorded_effect_response_transition(&self.snapshot(), request, value)?;
        self.overwrite_from_snapshot(transition.after);
        Ok(transition.response)
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

    fn overwrite_from_snapshot(&mut self, snapshot: RuntimeSnapshot) {
        self.logical_time = snapshot.logical_time;
        self.rng_state = snapshot.rng_state;
        self.effect_sequence = snapshot.effect_sequence;
        self.messages = snapshot.messages;
        self.assertions = snapshot.assertions;
        self.observers = snapshot.observers;
    }

    fn next_effect_sequence(&mut self) -> u64 {
        let sequence = self.effect_sequence;
        self.effect_sequence += 1;
        sequence
    }

    fn next_random(&mut self, upper: u64) -> u64 {
        // Deterministic xorshift64* profile; not cryptographic and not ambient entropy.
        let (next_state, value) = deterministic_random_step(self.rng_state, upper);
        self.rng_state = next_state;
        value
    }
}
