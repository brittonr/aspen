use super::*;

const DEFAULT_SYNDICATE_HARNESS_SEED: u64 = 1;
const DEFAULT_MAX_FANOUT: usize = 8;
const SYNDICATE_BAG_INSERT_DELTA: i32 = 1;
const SYNDICATE_BAG_RETRACT_DELTA: i32 = -1;
const SYNDICATE_REPLAY_RECORDED: &str = "recorded";
const SYNDICATE_DIAGNOSTIC_ONLY: &str = "diagnostic-only";
const SYNDICATE_DECISION_PASS: &str = "pass";
const SYNDICATE_DECISION_DENY: &str = "deny";
const SYNDICATE_REFERENCE_SURFACE: &str = "syndicate-reference-harness-v1";
const MAX_SYNDICATE_EVENTS: usize = 1024;
const MAX_SYNDICATE_DIAGNOSTICS: usize = 256;

trait BoundedPush<T> {
    fn push_bounded(&mut self, value: T) -> Result<()>;
}

impl BoundedPush<Event> for Vec<Event> {
    fn push_bounded(&mut self, event: Event) -> Result<()> {
        let next = self
            .len()
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("syndicate event count overflow"))?;
        if next > MAX_SYNDICATE_EVENTS {
            return Err(crate::error::MoltenError::invalid_harness("syndicate events exceeded bound"));
        }
        self.push(event);
        Ok(())
    }
}

impl BoundedPush<String> for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()> {
        let next = self
            .len()
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("syndicate diagnostic count overflow"))?;
        if next > MAX_SYNDICATE_DIAGNOSTICS {
            return Err(crate::error::MoltenError::invalid_harness("syndicate diagnostics exceeded bound"));
        }
        self.push(diagnostic);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_fanout: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_fanout: DEFAULT_MAX_FANOUT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityReceipt {
    pub decision: String,
    pub molten_event_refs: Vec<String>,
    pub syndicate_event_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvidence {
    pub replayability_status: String,
    pub event_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub trace_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowControlReceipt {
    pub decision: String,
    pub step_ref: String,
    pub fanout: usize,
    pub max_fanout: usize,
    pub account_observation_ref: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRun {
    pub molten_events: Vec<Event>,
    pub syndicate_events: Vec<Event>,
    pub parity: ParityReceipt,
    pub trace: TraceEvidence,
    pub flow_control: Vec<FlowControlReceipt>,
}

#[derive(Debug)]
pub struct ReferenceHarness {
    assertions: ::syndicate::bag::BTreeBag<Assertion>,
    observers: OrderedSet<Observer>,
    messages: OrderedSet<Message>,
}

impl Default for ReferenceHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceHarness {
    pub fn new() -> Self {
        Self {
            assertions: ::syndicate::bag::BTreeBag::new(),
            observers: OrderedSet::new(),
            messages: OrderedSet::new(),
        }
    }

    pub fn apply_step(&mut self, step: &Step) -> Result<Vec<Event>> {
        match step {
            Step::Send { from, to, body } => {
                let message = Message {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                };
                self.messages.insert(message);
                Ok(vec![Event::MessageDelivered {
                    from: from.clone(),
                    to: to.clone(),
                    body: body.clone(),
                }])
            }
            Step::Observe { actor, pattern } => self.apply_observe(actor, pattern),
            Step::Assert { actor, value } => self.apply_assert(actor, value),
            Step::Retract { actor, value } => self.apply_retract(actor, value),
            Step::Clock { .. } | Step::Random { .. } => Ok(Vec::new()),
        }
    }

    pub fn preview_fanout(&self, step: &Step) -> Result<u64> {
        let count = match step {
            Step::Observe { pattern, .. } => self.matching_assertion_count(pattern)?,
            Step::Assert { value, .. } | Step::Retract { value, .. } => self.matching_observer_count(value)?,
            Step::Send { .. } | Step::Clock { .. } | Step::Random { .. } => usize::default(),
        };
        u64::try_from(count).map_err(|_| crate::error::MoltenError::invalid_harness("syndicate fanout count overflow"))
    }

    // r[impl molten.syndicate_dataspace.facet_cleanup]
    pub fn cleanup_actor_scope(&mut self, actor: &str) -> Result<RuntimeScopeCleanup> {
        let mut assertion_refs = Vec::with_capacity(self.assertions.len());
        let actor_assertions: Vec<_> =
            self.assertions.keys().filter(|assertion| assertion.actor == actor).collect::<Vec<_>>();
        for assertion in &actor_assertions {
            assertion_refs.push_bounded(assertion.assertion_ref()?)?;
        }
        let removed_assertions: Vec<_> = actor_assertions.into_iter().cloned().collect();
        for assertion in removed_assertions {
            self.assertions.change_clamped(assertion, SYNDICATE_BAG_RETRACT_DELTA);
        }

        let mut observer_refs = Vec::with_capacity(self.observers.len());
        for observer in self.observers.iter().filter(|observer| observer.actor == actor) {
            observer_refs.push_bounded(observer.observer_ref()?)?;
        }
        self.observers.retain(|observer| observer.actor != actor);

        let mut message_refs = Vec::with_capacity(self.messages.len());
        for message in self.messages.iter().filter(|message| message.from == actor || message.to == actor) {
            message_refs.push_bounded(message.message_ref()?)?;
        }
        self.messages.retain(|message| message.from != actor && message.to != actor);

        assertion_refs.sort();
        observer_refs.sort();
        message_refs.sort();
        Ok(RuntimeScopeCleanup {
            actor: actor.to_owned(),
            assertion_refs,
            observer_refs,
            message_refs,
        })
    }

    fn apply_observe(&mut self, actor: &str, pattern: &Value) -> Result<Vec<Event>> {
        let mut events = vec![Event::ObserveRegistered {
            actor: actor.to_string(),
            pattern: pattern.clone(),
        }];
        for assertion in self.matching_assertions(pattern)? {
            events.push(Event::AssertionObserved {
                observer: actor.to_string(),
                owner: assertion.actor.clone(),
                value: assertion.value.clone(),
            });
        }
        self.observers.insert(Observer {
            actor: actor.to_string(),
            pattern: pattern.clone(),
        });
        Ok(events)
    }

    fn apply_assert(&mut self, actor: &str, value: &Value) -> Result<Vec<Event>> {
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
        };
        let mut events = vec![Event::AssertionCommitted {
            actor: actor.to_string(),
            value: value.clone(),
        }];
        for observer in self.matching_observers(value)? {
            events.push(Event::AssertionObserved {
                observer: observer.actor,
                owner: actor.to_string(),
                value: value.clone(),
            });
        }
        self.assertions.change(assertion, SYNDICATE_BAG_INSERT_DELTA);
        Ok(events)
    }

    fn apply_retract(&mut self, actor: &str, value: &Value) -> Result<Vec<Event>> {
        let assertion = Assertion {
            actor: actor.to_string(),
            value: value.clone(),
        };
        let mut events = vec![Event::AssertionRetracted {
            actor: actor.to_string(),
            value: value.clone(),
        }];
        for observer in self.matching_observers(value)? {
            events.push(Event::AssertionRetractionObserved {
                observer: observer.actor,
                owner: actor.to_string(),
                value: value.clone(),
            });
        }
        self.assertions.change_clamped(assertion, SYNDICATE_BAG_RETRACT_DELTA);
        Ok(events)
    }

    fn matching_assertion_count(&self, pattern: &Value) -> Result<usize> {
        Ok(self.matching_assertions(pattern)?.len())
    }

    fn matching_observer_count(&self, value: &Value) -> Result<usize> {
        Ok(self.matching_observers(value)?.len())
    }

    fn matching_assertions(&self, pattern_value: &Value) -> Result<Vec<Assertion>> {
        let pattern = RuntimePattern::from_observe_value(pattern_value)?;
        let mut assertions = Vec::with_capacity(self.assertions.len());
        for assertion in self.assertions.keys() {
            if pattern.matches_value(&assertion.value)?.0 {
                assertions.push(assertion.clone());
            }
        }
        Ok(assertions)
    }

    fn matching_observers(&self, value: &Value) -> Result<Vec<Observer>> {
        let mut observers = Vec::with_capacity(self.observers.len());
        for observer in &self.observers {
            let pattern = RuntimePattern::from_observe_value(&observer.pattern)?;
            if pattern.matches_value(value)?.0 {
                observers.push(observer.clone());
            }
        }
        Ok(observers)
    }
}
