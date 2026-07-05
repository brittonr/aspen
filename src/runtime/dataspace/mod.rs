type ActorId = super::ActorId;
type Envelope = super::Envelope;
type EnvelopeBoundary = super::EnvelopeBoundary;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OrderedMap<Key, Value> = std::collections::BTreeMap<Key, Value>;
type OrderedSet<Value> = std::collections::BTreeSet<Value>;
type PendingTurn = super::PendingTurn;
type AdmissionRequest = super::AdmissionRequest;
type CapabilityContext = super::CapabilityContext;
type PredicateDecision = super::PredicateDecision;
type Result<T> = crate::error::Result<T>;
type RuntimePattern = super::RuntimePattern;
type Assertion = super::RuntimeAssertion;
type Effect = super::RuntimeEffect;
type Event = super::RuntimeEvent;
type Message = super::RuntimeMessage;
type Observer = super::RuntimeObserver;
type PredicateReceipt = super::RuntimePredicateReceipt;
type Step = super::RuntimeStep;
type Value = super::RuntimeValue;
type TurnAction = super::TurnAction;
type TurnOutcome = super::TurnOutcome;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn committed_turn_snapshot(before: &RuntimeSnapshot, turn: &PendingTurn) -> RuntimeSnapshot {
    super::committed_turn_snapshot(before, turn)
}

fn evaluate_turn_transition(
    before: &RuntimeSnapshot,
    turn: &PendingTurn,
    after: &RuntimeSnapshot,
    outcome: TurnOutcome,
) -> Result<PredicateReceipt> {
    super::evaluate_turn_transition(before, turn, after, outcome)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub logical_time: u64,
    pub rng_state: u64,
    pub effect_sequence: u64,
    pub messages: OrderedSet<Message>,
    pub assertions: OrderedSet<Assertion>,
    pub observers: OrderedSet<Observer>,
}

impl RuntimeSnapshot {
    pub fn to_value(&self) -> IoValue {
        record("runtime-snapshot-v1", vec![
            u64_value(self.logical_time),
            u64_value(self.rng_state),
            u64_value(self.effect_sequence),
            sequence(self.messages.iter().map(Message::to_value).collect()),
            sequence(self.assertions.iter().map(Assertion::to_value).collect()),
            sequence(self.observers.iter().map(Observer::to_value).collect()),
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
    messages: OrderedSet<Message>,
    assertions: OrderedSet<Assertion>,
    observers: OrderedSet<Observer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScopeCleanup {
    pub actor: String,
    pub assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub message_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDelivery {
    pub actor: ActorId,
    pub boundary: EnvelopeBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LocalSubscription {
    pattern_ref: String,
    pattern: RuntimePattern,
}

impl LocalSubscription {
    fn new(pattern: RuntimePattern) -> Result<Self> {
        pattern.validate()?;
        Ok(Self {
            pattern_ref: pattern.pattern_ref()?,
            pattern,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalAdapter {
    subscriptions: OrderedMap<String, OrderedSet<LocalSubscription>>,
}

impl LocalAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_actor(&mut self, actor: ActorId) {
        self.subscriptions.entry(actor.into_string()).or_default();
    }

    pub fn observe_subject(&mut self, actor: ActorId, subject: &Value) {
        if let Ok(subscription) = LocalSubscription::new(RuntimePattern::exact(subject.clone())) {
            self.subscriptions.entry(actor.into_string()).or_default().insert(subscription);
        }
    }

    pub fn observe_pattern(&mut self, actor: ActorId, pattern: RuntimePattern) -> Result<()> {
        let subscription = LocalSubscription::new(pattern)?;
        self.subscriptions.entry(actor.into_string()).or_default().insert(subscription);
        Ok(())
    }

    pub fn observe_pattern_value(&mut self, actor: ActorId, pattern: &Value) -> Result<()> {
        self.observe_pattern(actor, RuntimePattern::from_observe_value(pattern)?)
    }

    pub fn route_envelope(&self, envelope: &Envelope) -> Result<Vec<LocalDelivery>> {
        let subject_ref = envelope.subject.value_ref();
        let boundary = envelope.boundary()?;
        let mut deliveries = Vec::with_capacity(self.subscriptions.len());
        for (actor, subscriptions) in &self.subscriptions {
            for subscription in subscriptions {
                // r[impl molten.preserves_boundary_codegen.pattern_routing]
                if subscription.pattern.matches_value(&envelope.subject)?.0 {
                    tracing::event!(
                        tracing::Level::DEBUG,
                        adapter = "local-dataspace",
                        actor = actor.as_str(),
                        pattern_ref = subscription.pattern_ref.as_str(),
                        subject_ref = subject_ref,
                        "runtime adapter pattern matched"
                    );
                    deliveries.push(LocalDelivery {
                        actor: ActorId::parse(actor.clone())?,
                        boundary: boundary.clone(),
                    });
                    break;
                }
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

mod state;
mod syndicate;
pub use state::RuntimeRecordedEffectTransition;
pub use state::recorded_effect_response_transition;
pub use syndicate::FlowControlReceipt;
pub use syndicate::ParityReceipt;
pub use syndicate::ReferenceHarness;
pub use syndicate::ReferenceRun;
pub use syndicate::ResourceBudget;
pub use syndicate::TraceEvidence;
pub use syndicate::run_reference_harness;

#[cfg(test)]
mod tests;
