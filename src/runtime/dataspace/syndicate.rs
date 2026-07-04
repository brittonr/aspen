use ::syndicate::bag::BTreeBag;
use ::syndicate::syndicate_package_version;

use super::*;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const DEFAULT_SYNDICATE_HARNESS_SEED: u64 = 1;
const DEFAULT_MAX_FANOUT: usize = 8;
const SYNDICATE_BAG_INSERT_DELTA: i32 = 1;
const SYNDICATE_BAG_RETRACT_DELTA: i32 = -1;
const SYNDICATE_REPLAY_RECORDED: &str = "recorded";
const SYNDICATE_DIAGNOSTIC_ONLY: &str = "diagnostic-only";
const SYNDICATE_DECISION_PASS: &str = "pass";
const SYNDICATE_DECISION_DENY: &str = "deny";
const SYNDICATE_REFERENCE_SURFACE: &str = "syndicate-reference-harness-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyndicateResourceBudget {
    pub max_fanout: usize,
}

impl Default for SyndicateResourceBudget {
    fn default() -> Self {
        Self {
            max_fanout: DEFAULT_MAX_FANOUT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyndicateParityReceipt {
    pub decision: String,
    pub molten_event_refs: Vec<String>,
    pub syndicate_event_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyndicateTraceEvidence {
    pub replayability_status: String,
    pub event_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub trace_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyndicateFlowControlReceipt {
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
pub struct SyndicateReferenceRun {
    pub molten_events: Vec<Event>,
    pub syndicate_events: Vec<Event>,
    pub parity: SyndicateParityReceipt,
    pub trace: SyndicateTraceEvidence,
    pub flow_control: Vec<SyndicateFlowControlReceipt>,
}

#[derive(Debug)]
pub struct SyndicateReferenceHarness {
    assertions: BTreeBag<Assertion>,
    observers: OrderedSet<Observer>,
    messages: OrderedSet<Message>,
}

impl Default for SyndicateReferenceHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl SyndicateReferenceHarness {
    pub fn new() -> Self {
        Self {
            assertions: BTreeBag::new(),
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

    pub fn preview_fanout(&self, step: &Step) -> Result<usize> {
        match step {
            Step::Observe { pattern, .. } => self.matching_assertion_count(pattern),
            Step::Assert { value, .. } | Step::Retract { value, .. } => self.matching_observer_count(value),
            Step::Send { .. } | Step::Clock { .. } | Step::Random { .. } => Ok(usize::default()),
        }
    }

    // r[impl molten.syndicate_dataspace.facet_cleanup]
    pub fn cleanup_actor_scope(&mut self, actor: &str) -> Result<RuntimeScopeCleanup> {
        let mut assertion_refs = Vec::with_capacity(self.assertions.len());
        let mut removed_assertions = Vec::new();
        for assertion in self.assertions.keys().filter(|assertion| assertion.actor == actor) {
            assertion_refs.push(assertion.assertion_ref()?);
            removed_assertions.push(assertion.clone());
        }
        for assertion in removed_assertions {
            self.assertions.change_clamped(assertion, SYNDICATE_BAG_RETRACT_DELTA);
        }

        let mut observer_refs = Vec::with_capacity(self.observers.len());
        for observer in self.observers.iter().filter(|observer| observer.actor == actor) {
            observer_refs.push(observer.observer_ref()?);
        }
        self.observers.retain(|observer| observer.actor != actor);

        let mut message_refs = Vec::with_capacity(self.messages.len());
        for message in self.messages.iter().filter(|message| message.from == actor || message.to == actor) {
            message_refs.push(message.message_ref()?);
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

// r[impl molten.syndicate_dataspace.reference_harness]
// r[impl molten.syndicate_dataspace.parity_receipts]
// r[impl molten.syndicate_dataspace.cap_attenuation]
// r[impl molten.syndicate_dataspace.flow_control_receipts]
// r[impl molten.syndicate_dataspace.trace_evidence]
pub fn run_syndicate_reference_harness(
    steps: &[Step],
    capabilities: &CapabilityContext,
    budget: SyndicateResourceBudget,
) -> Result<SyndicateReferenceRun> {
    let mut molten_state = RuntimeState::new(DEFAULT_SYNDICATE_HARNESS_SEED);
    let mut syndicate_state = SyndicateReferenceHarness::new();
    let mut molten_events = Vec::new();
    let mut syndicate_events = Vec::new();
    let mut flow_control = Vec::with_capacity(steps.len());

    for step in steps {
        let request = AdmissionRequest::from_step(step);
        let authorization = capabilities.authorize(&request);
        if !authorization.authorized {
            let rollback =
                denied_event(step, "missing Molten capability admission; Syndicate cap evidence is diagnostic-only");
            molten_events.push(rollback.clone());
            syndicate_events.push(rollback);
            continue;
        }

        let fanout = syndicate_state.preview_fanout(step)?;
        let flow = flow_control_receipt(step, fanout, budget)?;
        let flow_passed = flow.decision == SYNDICATE_DECISION_PASS;
        flow_control.push(flow);
        if !flow_passed {
            let rollback = denied_event(step, "Syndicate account fanout exceeds Molten resource budget");
            molten_events.push(rollback.clone());
            syndicate_events.push(rollback);
            continue;
        }

        molten_events.extend(molten_state.apply_step(step));
        syndicate_events.extend(syndicate_state.apply_step(step)?);
    }

    let parity = parity_receipt(&molten_events, &syndicate_events)?;
    let trace = trace_evidence(&syndicate_events)?;
    Ok(SyndicateReferenceRun {
        molten_events,
        syndicate_events,
        parity,
        trace,
        flow_control,
    })
}

fn denied_event(step: &Step, reason: &str) -> Event {
    Event::TurnRolledBack {
        actor: step.primary_actor().to_string(),
        reason: reason.to_string(),
    }
}

fn flow_control_receipt(
    step: &Step,
    fanout: usize,
    budget: SyndicateResourceBudget,
) -> Result<SyndicateFlowControlReceipt> {
    let step_ref = step.step_ref()?;
    let account_value = record("syndicate-account-observation-v1", vec![
        record("surface", vec![string(SYNDICATE_REFERENCE_SURFACE)]),
        record("syndicate-version", vec![string(syndicate_package_version())]),
        record("step-ref", vec![string(&step_ref)]),
        record("fanout", vec![u64_value(usize_to_u64(fanout, "fanout")?)]),
        record("max-fanout", vec![u64_value(usize_to_u64(budget.max_fanout, "max fanout")?)]),
    ]);
    let account_observation_ref = canonical_hash(&account_value)?;
    let (decision, diagnostics) = if fanout <= budget.max_fanout {
        (SYNDICATE_DECISION_PASS.to_string(), Vec::new())
    } else {
        (SYNDICATE_DECISION_DENY.to_string(), vec!["syndicate-account-fanout-budget-exceeded".to_string()])
    };
    let value = record("syndicate-flow-control-receipt-v1", vec![
        record("step-ref", vec![string(&step_ref)]),
        record("account-observation-ref", vec![string(&account_observation_ref)]),
        record("fanout", vec![u64_value(usize_to_u64(fanout, "fanout")?)]),
        record("max-fanout", vec![u64_value(usize_to_u64(budget.max_fanout, "max fanout")?)]),
        record("decision", vec![string(&decision)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(SyndicateFlowControlReceipt {
        decision,
        step_ref,
        fanout,
        max_fanout: budget.max_fanout,
        account_observation_ref,
        diagnostics,
        receipt_ref,
        value,
    })
}

fn parity_receipt(molten_events: &[Event], syndicate_events: &[Event]) -> Result<SyndicateParityReceipt> {
    let molten_event_refs = event_refs(molten_events)?;
    let syndicate_event_refs = event_refs(syndicate_events)?;
    let diagnostics = parity_diagnostics(&molten_event_refs, &syndicate_event_refs);
    let decision = if diagnostics.is_empty() {
        SYNDICATE_DECISION_PASS.to_string()
    } else {
        SYNDICATE_DECISION_DENY.to_string()
    };
    let value = record("syndicate-parity-receipt-v1", vec![
        record("surface", vec![string(SYNDICATE_REFERENCE_SURFACE)]),
        record("syndicate-version", vec![string(syndicate_package_version())]),
        record("molten-events", vec![refs_value(&molten_event_refs)]),
        record("syndicate-events", vec![refs_value(&syndicate_event_refs)]),
        record("decision", vec![string(&decision)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(SyndicateParityReceipt {
        decision,
        molten_event_refs,
        syndicate_event_refs,
        diagnostics,
        receipt_ref,
        value,
    })
}

fn trace_evidence(events: &[Event]) -> Result<SyndicateTraceEvidence> {
    let event_refs = event_refs(events)?;
    let (replayability_status, diagnostics) = if event_refs.is_empty() {
        (SYNDICATE_DIAGNOSTIC_ONLY.to_string(), vec![
            "syndicate-trace-has-no-committed-action-refs".to_string(),
        ])
    } else {
        (SYNDICATE_REPLAY_RECORDED.to_string(), Vec::new())
    };
    let value = record("syndicate-trace-evidence-v1", vec![
        record("surface", vec![string(SYNDICATE_REFERENCE_SURFACE)]),
        record("syndicate-version", vec![string(syndicate_package_version())]),
        record("event-refs", vec![refs_value(&event_refs)]),
        record("replayability", vec![string(&replayability_status)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
    ]);
    let trace_ref = canonical_hash(&value)?;
    Ok(SyndicateTraceEvidence {
        replayability_status,
        event_refs,
        diagnostics,
        trace_ref,
        value,
    })
}

fn event_refs(events: &[Event]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(events.len());
    for event in events {
        refs.push(event.event_ref()?);
    }
    Ok(refs)
}

fn parity_diagnostics(left: &[String], right: &[String]) -> Vec<String> {
    if left == right {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    let shared = left.len().min(right.len());
    for index in usize::default()..shared {
        if left[index] != right[index] {
            diagnostics.push(format!("first-divergent-event-ref-index-{index}"));
            return diagnostics;
        }
    }
    diagnostics.push("event-ref-count-mismatch".to_string());
    diagnostics
}

fn refs_value(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn usize_to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} cannot convert from usize to u64: {error}")))
}
