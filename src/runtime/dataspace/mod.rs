use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::ActorId;
use super::Envelope;
use super::EnvelopeBoundary;
use super::PendingTurn;
use super::PredicateDecision;
use super::RuntimeAssertion;
use super::RuntimeEffect;
use super::RuntimeEvent;
use super::RuntimeMessage;
use super::RuntimeObserver;
use super::RuntimePredicateReceipt;
use super::RuntimeStep;
use super::RuntimeValue;
use super::TurnAction;
use super::TurnOutcome;
use super::evaluate_turn_transition;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn canonical_hash(value: &preserves::IOValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn u64_value(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScopeCleanup {
    pub actor: String,
    pub assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub message_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEnvelopeDelivery {
    pub actor: ActorId,
    pub boundary: EnvelopeBoundary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalDataspaceAdapter {
    subscriptions: BTreeMap<String, BTreeSet<String>>,
}

impl LocalDataspaceAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_actor(&mut self, actor: ActorId) {
        self.subscriptions.entry(actor.into_string()).or_default();
    }

    pub fn observe_subject(&mut self, actor: ActorId, subject: &RuntimeValue) {
        self.subscriptions.entry(actor.into_string()).or_default().insert(subject.value_ref().to_string());
    }

    pub fn route_envelope(&self, envelope: &Envelope) -> Result<Vec<LocalEnvelopeDelivery>> {
        let subject_ref = envelope.subject.value_ref();
        let boundary = envelope.boundary()?;
        let mut deliveries = Vec::with_capacity(self.subscriptions.len());
        for (actor, subjects) in &self.subscriptions {
            if subjects.contains(subject_ref) {
                deliveries.push(LocalEnvelopeDelivery {
                    actor: ActorId::parse(actor.clone())?,
                    boundary: boundary.clone(),
                });
            }
        }
        tracing::event!(
            tracing::Level::DEBUG,
            adapter = "local-dataspace",
            decision = "route",
            subject_ref = subject_ref,
            deliveries = deliveries.len(),
            "runtime adapter decision"
        );
        Ok(deliveries)
    }
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

    pub fn commit_turn_with_predicate_receipt(
        &mut self,
        turn: PendingTurn,
    ) -> Result<(Vec<RuntimeEvent>, RuntimePredicateReceipt)> {
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

    pub fn rollback_turn_with_predicate_receipt(
        &self,
        turn: PendingTurn,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<(Vec<RuntimeEvent>, RuntimePredicateReceipt)> {
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
    use super::*;
    use crate::runtime::Capability;
    use crate::runtime::ContentRef;
    use crate::runtime::EnvelopeInput;
    use crate::runtime::EvidenceRef;

    #[test]
    fn local_dataspace_routes_matching_envelope_subject() {
        let subject = RuntimeValue::string("service.ready").expect("subject");
        let envelope = Envelope::new(EnvelopeInput {
            sender: ActorId::parse("actor:producer").expect("sender"),
            subject: subject.clone(),
            body: RuntimeValue::string("ready").expect("body"),
            blob_refs: vec![
                ContentRef::parse(crate::preserves_rail::content_ref_from_bytes(b"payload")).expect("blob"),
            ],
            capabilities: vec![Capability::parse("send:service.ready").expect("capability")],
            evidence_refs: vec![
                EvidenceRef::parse(crate::preserves_rail::content_ref_from_bytes(b"route-evidence")).expect("evidence"),
            ],
        })
        .expect("envelope");
        let mut adapter = LocalDataspaceAdapter::new();
        adapter.register_actor(ActorId::parse("actor:ignored").expect("ignored actor"));
        adapter.observe_subject(ActorId::parse("actor:consumer").expect("consumer"), &subject);

        let deliveries = adapter.route_envelope(&envelope).expect("deliveries");
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].actor.as_str(), "actor:consumer");
        assert_eq!(deliveries[0].boundary.subject_ref, subject.value_ref());
        assert_eq!(deliveries[0].boundary.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
    }

    #[test]
    fn runtime_values_and_events_expose_stable_content_refs() {
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        crate::preserves_rail::validate_content_ref(value.value_ref()).expect("value ref shape");
        assert_eq!(
            value.value_ref(),
            crate::preserves_rail::canonical_hash(value.as_iovalue()).expect("canonical value ref")
        );

        let message = RuntimeMessage {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value.clone(),
        };
        crate::preserves_rail::validate_content_ref(&message.message_ref().expect("message ref"))
            .expect("message ref shape");
        let mut state = RuntimeState::new(7);
        state.apply_step(&RuntimeStep::Send {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value.clone(),
        });
        let snapshot_ref = state.snapshot().snapshot_ref().expect("snapshot ref");
        crate::preserves_rail::validate_content_ref(&snapshot_ref).expect("snapshot ref shape");

        let event = RuntimeEvent::MessageDelivered {
            from: "producer".to_string(),
            to: "consumer".to_string(),
            body: value,
        };
        let event_ref = event.event_ref().expect("event ref");
        crate::preserves_rail::validate_content_ref(&event_ref).expect("event ref shape");
        assert_eq!(event_ref, event.event_ref().expect("event ref stable"));
    }

    #[test]
    fn snapshot_model_covers_handler_state_and_dataspace_indexes() {
        let mut state = RuntimeState::new(7);
        let message = RuntimeValue::string("hello").expect("runtime test value");
        state.apply_step(&RuntimeStep::Send {
            from: "producer".into(),
            to: "consumer".into(),
            body: message,
        });
        state.apply_step(&RuntimeStep::Observe {
            actor: "consumer".into(),
            pattern: RuntimeValue::string("service.ready").expect("runtime test value"),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "producer".into(),
            value: RuntimeValue::string("service.ready").expect("runtime test value"),
        });
        state.apply_step(&RuntimeStep::Clock {
            actor: "producer".into(),
        });
        state.apply_step(&RuntimeStep::Random {
            actor: "producer".into(),
            upper: 100,
        });
        let snapshot = state.snapshot();
        assert_eq!(snapshot.logical_time, 1);
        assert_ne!(snapshot.rng_state, 7);
        assert_eq!(snapshot.effect_sequence, 2);
        assert_eq!(snapshot.messages.len(), 1);
        assert_eq!(snapshot.assertions.len(), 1);
        assert_eq!(snapshot.observers.len(), 1);
        crate::preserves_rail::validate_content_ref(&snapshot.snapshot_ref().expect("snapshot ref"))
            .expect("snapshot ref shape");
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
            RuntimeEvent::EffectRequest { sequence: 0, .. },
            RuntimeEvent::EffectResponse {
                sequence: 0,
                value: 0,
                ..
            }
        ]));
        let random = state.apply_step(&RuntimeStep::Random {
            actor: "a".into(),
            upper: 10,
        });
        assert!(matches!(random.as_slice(), [
            RuntimeEvent::EffectRequest {
                sequence: 1,
                upper: Some(10),
                ..
            },
            RuntimeEvent::EffectResponse {
                sequence: 1,
                upper: Some(10),
                ..
            }
        ]));

        let mut replay = RuntimeState::new(7);
        assert_eq!(clock, replay.apply_step(&RuntimeStep::Clock { actor: "a".into() }));
        assert_eq!(
            random,
            replay.apply_step(&RuntimeStep::Random {
                actor: "a".into(),
                upper: 10
            })
        );
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
        assert_eq!(state.snapshot(), before);
        let events = state.rollback_turn(turn, step.primary_actor(), "policy denied");
        assert_eq!(state.snapshot(), before);
        assert!(matches!(events.as_slice(), [RuntimeEvent::TurnRolledBack { .. }]));

        let committed = state.apply_step(&step);
        assert!(matches!(committed.as_slice(), [RuntimeEvent::AssertionCommitted { .. }]));
        assert_ne!(state.snapshot(), before);
    }
}
