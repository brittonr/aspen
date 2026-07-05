use ::syndicate::bag::BTreeBag;
use ::syndicate::syndicate_package_version;

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
    assertions: BTreeBag<Assertion>,
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

// r[impl molten.syndicate_dataspace.reference_harness]
// r[impl molten.syndicate_dataspace.parity_receipts]
// r[impl molten.syndicate_dataspace.cap_attenuation]
// r[impl molten.syndicate_dataspace.flow_control_receipts]
// r[impl molten.syndicate_dataspace.trace_evidence]
pub fn run_reference_harness(
    steps: &[Step],
    capabilities: &CapabilityContext,
    budget: ResourceBudget,
) -> Result<ReferenceRun> {
    let mut molten_state = RuntimeState::new(DEFAULT_SYNDICATE_HARNESS_SEED);
    let mut syndicate_state = ReferenceHarness::new();
    let mut molten_events = Vec::new();
    let mut syndicate_events = Vec::new();
    let mut flow_control = Vec::with_capacity(steps.len());

    for step in steps {
        let request = AdmissionRequest::from_step(step);
        let authorization = capabilities.authorize(&request);
        if !authorization.authorized {
            let rollback =
                denied_event(step, "missing Molten capability admission; Syndicate cap evidence is diagnostic-only");
            molten_events.push_bounded(rollback.clone())?;
            syndicate_events.push_bounded(rollback)?;
            continue;
        }

        let fanout = syndicate_state.preview_fanout(step)?;
        let fanout_usize = usize::try_from(fanout)
            .map_err(|_| crate::error::MoltenError::invalid_harness("syndicate fanout count overflow"))?;
        let flow = flow_control_receipt(step, fanout_usize, budget)?;
        let is_flow_passed = flow.decision == SYNDICATE_DECISION_PASS;
        flow_control.push(flow);
        if !is_flow_passed {
            let rollback = denied_event(step, "Syndicate account fanout exceeds Molten resource budget");
            molten_events.push_bounded(rollback.clone())?;
            syndicate_events.push_bounded(rollback)?;
            continue;
        }

        let molten_new = molten_state.apply_step(step);
        for event in molten_new {
            molten_events.push_bounded(event)?;
        }
        let syndicate_new = syndicate_state.apply_step(step)?;
        for event in syndicate_new {
            syndicate_events.push_bounded(event)?;
        }
    }

    let parity = parity_receipt(&molten_events, &syndicate_events)?;
    let trace = trace_evidence(&syndicate_events)?;
    Ok(ReferenceRun {
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

fn flow_control_receipt(step: &Step, fanout: usize, budget: ResourceBudget) -> Result<FlowControlReceipt> {
    let step_ref = step.step_ref()?;
    let account_value = crate::preserves_rail::record("syndicate-account-observation-v1", vec![
        crate::preserves_rail::record("surface", vec![crate::preserves_rail::string(SYNDICATE_REFERENCE_SURFACE)]),
        crate::preserves_rail::record("syndicate-version", vec![crate::preserves_rail::string(
            syndicate_package_version(),
        )]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(&step_ref)]),
        crate::preserves_rail::record("fanout", vec![crate::preserves_rail::u64_value(usize_to_u64(
            fanout, "fanout",
        )?)]),
        crate::preserves_rail::record("max-fanout", vec![crate::preserves_rail::u64_value(usize_to_u64(
            budget.max_fanout,
            "max fanout",
        )?)]),
    ]);
    let account_observation_ref = crate::preserves_rail::canonical_hash(&account_value)?;
    let (decision, diagnostics) = if fanout <= budget.max_fanout {
        (SYNDICATE_DECISION_PASS.to_string(), Vec::new())
    } else {
        (SYNDICATE_DECISION_DENY.to_string(), vec!["syndicate-account-fanout-budget-exceeded".to_string()])
    };
    let value = crate::preserves_rail::record("syndicate-flow-control-receipt-v1", vec![
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(&step_ref)]),
        crate::preserves_rail::record("account-observation-ref", vec![crate::preserves_rail::string(
            &account_observation_ref,
        )]),
        crate::preserves_rail::record("fanout", vec![crate::preserves_rail::u64_value(usize_to_u64(
            fanout, "fanout",
        )?)]),
        crate::preserves_rail::record("max-fanout", vec![crate::preserves_rail::u64_value(usize_to_u64(
            budget.max_fanout,
            "max fanout",
        )?)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(FlowControlReceipt {
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

fn parity_receipt(molten_events: &[Event], syndicate_events: &[Event]) -> Result<ParityReceipt> {
    let molten_event_refs = event_refs(molten_events)?;
    let syndicate_event_refs = event_refs(syndicate_events)?;
    let diagnostics = parity_diagnostics(&molten_event_refs, &syndicate_event_refs)?;
    let decision = if diagnostics.is_empty() {
        SYNDICATE_DECISION_PASS.to_string()
    } else {
        SYNDICATE_DECISION_DENY.to_string()
    };
    let value = crate::preserves_rail::record("syndicate-parity-receipt-v1", vec![
        crate::preserves_rail::record("surface", vec![crate::preserves_rail::string(SYNDICATE_REFERENCE_SURFACE)]),
        crate::preserves_rail::record("syndicate-version", vec![crate::preserves_rail::string(
            syndicate_package_version(),
        )]),
        crate::preserves_rail::record("molten-events", vec![refs_value(&molten_event_refs)]),
        crate::preserves_rail::record("syndicate-events", vec![refs_value(&syndicate_event_refs)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ParityReceipt {
        decision,
        molten_event_refs,
        syndicate_event_refs,
        diagnostics,
        receipt_ref,
        value,
    })
}

fn trace_evidence(events: &[Event]) -> Result<TraceEvidence> {
    let event_refs = event_refs(events)?;
    let (replayability_status, diagnostics) = if event_refs.is_empty() {
        (SYNDICATE_DIAGNOSTIC_ONLY.to_string(), vec![
            "syndicate-trace-has-no-committed-action-refs".to_string(),
        ])
    } else {
        (SYNDICATE_REPLAY_RECORDED.to_string(), Vec::new())
    };
    let value = crate::preserves_rail::record("syndicate-trace-evidence-v1", vec![
        crate::preserves_rail::record("surface", vec![crate::preserves_rail::string(SYNDICATE_REFERENCE_SURFACE)]),
        crate::preserves_rail::record("syndicate-version", vec![crate::preserves_rail::string(
            syndicate_package_version(),
        )]),
        crate::preserves_rail::record("event-refs", vec![refs_value(&event_refs)]),
        crate::preserves_rail::record("replayability", vec![crate::preserves_rail::string(&replayability_status)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ]);
    let trace_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(TraceEvidence {
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

fn parity_diagnostics(left: &[String], right: &[String]) -> Result<Vec<String>> {
    if left == right {
        return Ok(Vec::new());
    }
    let mut diagnostics = Vec::new();
    let shared = left.len().min(right.len());
    for index in usize::default()..shared {
        if left[index] != right[index] {
            diagnostics.push_bounded(format!("first-divergent-event-ref-index-{index}"))?;
            return Ok(diagnostics);
        }
    }
    diagnostics.push_bounded("event-ref-count-mismatch".to_string())?;
    Ok(diagnostics)
}

fn refs_value(refs: &[String]) -> IoValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn usize_to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} cannot convert from usize to u64: {error}")))
}
