use std::collections::BTreeSet;

use super::PendingTurn;
use super::RuntimeAssertion;
use super::RuntimeEffect;
use super::RuntimeEvent;
use super::RuntimeMessage;
use super::RuntimeObserver;
use super::RuntimeStep;
use super::TurnAction;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::u64_value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub logical_time: u64,
    pub rng_state: u64,
    pub effect_sequence: u64,
    pub messages: BTreeSet<RuntimeMessage>,
    pub assertions: BTreeSet<RuntimeAssertion>,
    pub observers: BTreeSet<RuntimeObserver>,
}

impl RuntimeSnapshot {
    pub fn to_value(&self) -> preserves::IOValue {
        record("runtime-snapshot-v1", vec![
            u64_value(self.logical_time),
            u64_value(self.rng_state),
            u64_value(self.effect_sequence),
            sequence(self.messages.iter().map(RuntimeMessage::to_value).collect()),
            sequence(self.assertions.iter().map(RuntimeAssertion::to_value).collect()),
            sequence(self.observers.iter().map(RuntimeObserver::to_value).collect()),
        ])
    }

    pub fn snapshot_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeState {
    logical_time: u64,
    rng_state: u64,
    effect_sequence: u64,
    messages: BTreeSet<RuntimeMessage>,
    assertions: BTreeSet<RuntimeAssertion>,
    observers: BTreeSet<RuntimeObserver>,
}

fn effect_response(
    effect: RuntimeEffect,
    actor: String,
    sequence: u64,
    upper: Option<u64>,
    value: u64,
) -> RuntimeEvent {
    RuntimeEvent::EffectResponse {
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
            messages: BTreeSet::new(),
            assertions: BTreeSet::new(),
            observers: BTreeSet::new(),
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

    pub fn apply_step(&mut self, step: &RuntimeStep) -> Vec<RuntimeEvent> {
        match step {
            RuntimeStep::Clock { actor } => {
                let sequence = self.next_effect_sequence();
                let request = RuntimeEvent::EffectRequest {
                    effect: RuntimeEffect::Clock,
                    actor: actor.clone(),
                    sequence,
                    upper: None,
                };
                let value = self.local_clock_response_value();
                let response = effect_response(RuntimeEffect::Clock, actor.clone(), sequence, None, value);
                vec![request, response]
            }
            RuntimeStep::Random { actor, upper } => {
                let sequence = self.next_effect_sequence();
                let request = RuntimeEvent::EffectRequest {
                    effect: RuntimeEffect::Random,
                    actor: actor.clone(),
                    sequence,
                    upper: Some(*upper),
                };
                let value = self.next_random(*upper);
                let response = effect_response(RuntimeEffect::Random, actor.clone(), sequence, Some(*upper), value);
                vec![request, response]
            }
            RuntimeStep::Send { .. }
            | RuntimeStep::Observe { .. }
            | RuntimeStep::Assert { .. }
            | RuntimeStep::Retract { .. } => {
                let turn = self.begin_turn(step);
                self.commit_turn(turn)
            }
        }
    }

    pub fn begin_turn(&self, step: &RuntimeStep) -> PendingTurn {
        let mut turn = PendingTurn::new();
        self.stage_step(&mut turn, step);
        turn
    }

    pub fn commit_turn(&mut self, turn: PendingTurn) -> Vec<RuntimeEvent> {
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

    pub fn rollback_turn(
        &self,
        _turn: PendingTurn,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Vec<RuntimeEvent> {
        vec![RuntimeEvent::TurnRolledBack {
            actor: actor.into(),
            reason: reason.into(),
        }]
    }

    pub fn begin_effect_for_step(&mut self, step: &RuntimeStep) -> Option<RuntimeEvent> {
        match step {
            RuntimeStep::Clock { actor } => Some(RuntimeEvent::EffectRequest {
                effect: RuntimeEffect::Clock,
                actor: actor.clone(),
                sequence: self.next_effect_sequence(),
                upper: None,
            }),
            RuntimeStep::Random { actor, upper } => Some(RuntimeEvent::EffectRequest {
                effect: RuntimeEffect::Random,
                actor: actor.clone(),
                sequence: self.next_effect_sequence(),
                upper: Some(*upper),
            }),
            RuntimeStep::Send { .. }
            | RuntimeStep::Observe { .. }
            | RuntimeStep::Assert { .. }
            | RuntimeStep::Retract { .. } => None,
        }
    }

    pub fn apply_recorded_effect_response(&mut self, request: &RuntimeEvent, value: u64) -> Result<RuntimeEvent> {
        match request {
            RuntimeEvent::EffectRequest {
                effect: RuntimeEffect::Clock,
                actor,
                sequence,
                upper,
            } => {
                self.logical_time = value + 1;
                Ok(effect_response(RuntimeEffect::Clock, actor.clone(), *sequence, *upper, value))
            }
            RuntimeEvent::EffectRequest {
                effect: RuntimeEffect::Random,
                actor,
                sequence,
                upper: Some(upper),
            } => {
                let _ignored_local_value = self.next_random(*upper);
                Ok(effect_response(RuntimeEffect::Random, actor.clone(), *sequence, Some(*upper), value))
            }
            RuntimeEvent::EffectRequest {
                effect: RuntimeEffect::Random,
                ..
            } => Err(MoltenError::invalid_harness("recorded random effect request missing upper bound")),
            _ => Err(MoltenError::invalid_harness("recorded effect response requires an effect request")),
        }
    }

    fn stage_step(&self, turn: &mut PendingTurn, step: &RuntimeStep) {
        match step {
            RuntimeStep::Send { from, to, body } => {
                let message = RuntimeMessage {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                };
                turn.events.push(RuntimeEvent::MessageDelivered {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                });
                turn.actions.push(TurnAction::Send(message));
            }
            RuntimeStep::Observe { actor, pattern } => {
                let observer = RuntimeObserver {
                    actor: actor.clone(),
                    pattern: pattern.clone(),
                };
                turn.events.push(RuntimeEvent::ObserveRegistered {
                    actor: actor.clone(),
                    pattern: pattern.clone(),
                });
                for assertion in self.assertions.iter().filter(|assertion| assertion.value == *pattern) {
                    turn.events.push(RuntimeEvent::AssertionObserved {
                        observer: actor.clone(),
                        owner: assertion.actor.clone(),
                        value: assertion.value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Observe(observer));
            }
            RuntimeStep::Assert { actor, value } => {
                let assertion = RuntimeAssertion {
                    actor: actor.clone(),
                    value: value.clone(),
                };
                turn.events.push(RuntimeEvent::AssertionCommitted {
                    actor: actor.clone(),
                    value: value.clone(),
                });
                for observer in self.observers.iter().filter(|observer| observer.pattern == *value) {
                    turn.events.push(RuntimeEvent::AssertionObserved {
                        observer: observer.actor.clone(),
                        owner: actor.clone(),
                        value: value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Assert(assertion));
            }
            RuntimeStep::Retract { actor, value } => {
                let assertion = RuntimeAssertion {
                    actor: actor.clone(),
                    value: value.clone(),
                };
                turn.events.push(RuntimeEvent::AssertionRetracted {
                    actor: actor.clone(),
                    value: value.clone(),
                });
                for observer in self.observers.iter().filter(|observer| observer.pattern == *value) {
                    turn.events.push(RuntimeEvent::AssertionRetractionObserved {
                        observer: observer.actor.clone(),
                        owner: actor.clone(),
                        value: value.clone(),
                    });
                }
                turn.actions.push(TurnAction::Retract(assertion));
            }
            RuntimeStep::Clock { .. } | RuntimeStep::Random { .. } => {}
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

#[cfg(test)]
mod tests {
    use super::RuntimeState;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::validate_content_ref;
    use crate::runtime::RuntimeEvent;
    use crate::runtime::RuntimeMessage;
    use crate::runtime::RuntimeStep;
    use crate::runtime::RuntimeValue;

    #[test]
    fn runtime_values_and_events_expose_stable_content_refs() {
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        validate_content_ref(value.value_ref()).expect("value ref shape");
        assert_eq!(value.value_ref(), canonical_hash(value.as_iovalue()).expect("canonical value ref"));

        let message = RuntimeMessage {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value.clone(),
        };
        validate_content_ref(&message.message_ref().expect("message ref")).expect("message ref shape");
        let mut state = RuntimeState::new(7);
        state.apply_step(&RuntimeStep::Send {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value.clone(),
        });
        let snapshot_ref = state.snapshot().snapshot_ref().expect("snapshot ref");
        validate_content_ref(&snapshot_ref).expect("snapshot ref shape");

        let event = RuntimeEvent::MessageDelivered {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value,
        };
        let event_ref = event.event_ref().expect("event ref");
        validate_content_ref(&event_ref).expect("event ref shape");
        assert_eq!(event_ref, event.event_ref().expect("event ref stable"));
    }

    #[test]
    fn transition_is_deterministic_from_explicit_seed() {
        let steps = [
            RuntimeStep::Observe {
                actor: "consumer".into(),
                pattern: RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            RuntimeStep::Assert {
                actor: "producer".into(),
                value: RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            RuntimeStep::Clock {
                actor: "producer".into(),
            },
            RuntimeStep::Random {
                actor: "producer".into(),
                upper: 100,
            },
        ];
        let mut left = RuntimeState::new(7);
        let mut right = RuntimeState::new(7);
        for step in &steps {
            assert_eq!(left.apply_step(step), right.apply_step(step));
            assert_eq!(left.snapshot(), right.snapshot());
        }
    }

    #[test]
    fn clock_and_random_emit_request_response_pairs() {
        let mut state = RuntimeState::new(7);
        let clock = state.apply_step(&RuntimeStep::Clock { actor: "a".into() });
        assert!(matches!(clock.as_slice(), [
            RuntimeEvent::EffectRequest { .. },
            RuntimeEvent::EffectResponse { .. }
        ]));
        let random = state.apply_step(&RuntimeStep::Random {
            actor: "a".into(),
            upper: 10,
        });
        assert!(matches!(random.as_slice(), [
            RuntimeEvent::EffectRequest { .. },
            RuntimeEvent::EffectResponse { .. }
        ]));
    }

    #[test]
    fn rollback_leaves_staged_dataspace_actions_uncommitted() {
        let mut state = RuntimeState::new(1);
        let before = state.snapshot();
        let step = RuntimeStep::Assert {
            actor: "producer".into(),
            value: RuntimeValue::string("service.ready").expect("runtime test value"),
        };
        let turn = state.begin_turn(&step);
        let events = state.rollback_turn(turn, step.primary_actor(), "policy denied");
        assert_eq!(state.snapshot(), before);
        assert!(matches!(events.as_slice(), [RuntimeEvent::TurnRolledBack { .. }]));

        let committed = state.apply_step(&step);
        assert!(matches!(committed.as_slice(), [RuntimeEvent::AssertionCommitted { .. }]));
        assert_ne!(state.snapshot(), before);
    }
}
