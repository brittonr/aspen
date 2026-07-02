type AdmissionDecision = super::AdmissionDecision;
type AdmissionRequest = super::AdmissionRequest;
type Formatter<'a> = std::fmt::Formatter<'a>;
type FmtResult = std::fmt::Result;
type IoValue = preserves::IOValue;
type Ordering = std::cmp::Ordering;
type Result<T> = crate::error::Result<T>;

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

#[derive(Clone)]
pub struct RuntimeValue {
    value: IoValue,
    canonical: Vec<u8>,
    value_ref: String,
}

impl RuntimeValue {
    pub fn new(value: IoValue) -> Result<Self> {
        let canonical = canonical_bytes(&value)?;
        let value_ref = content_ref_from_bytes(&canonical);
        Ok(Self {
            value,
            canonical,
            value_ref,
        })
    }

    pub fn string(value: impl AsRef<str>) -> Result<Self> {
        Self::new(IoValue::new(value.as_ref().to_owned()))
    }

    pub fn as_iovalue(&self) -> &IoValue {
        &self.value
    }

    pub fn into_iovalue(self) -> IoValue {
        self.value
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }

    pub fn value_ref(&self) -> &str {
        &self.value_ref
    }
}

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("RuntimeValue")
            .field("value", &self.value)
            .field("canonical_len", &self.canonical.len())
            .field("value_ref", &self.value_ref)
            .finish()
    }
}

impl PartialEq for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for RuntimeValue {}

impl PartialOrd for RuntimeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RuntimeValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical.cmp(&other.canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMessage {
    pub from: String,
    pub to: String,
    pub body: RuntimeValue,
}

impl RuntimeMessage {
    pub fn to_value(&self) -> IoValue {
        record("runtime-message-v1", vec![
            string(&self.from),
            string(&self.to),
            self.body.as_iovalue().clone(),
            record("body-ref", vec![string(self.body.value_ref())]),
        ])
    }

    pub fn message_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAssertion {
    pub actor: String,
    pub value: RuntimeValue,
}

impl RuntimeAssertion {
    pub fn to_value(&self) -> IoValue {
        record("runtime-assertion-v1", vec![
            string(&self.actor),
            self.value.as_iovalue().clone(),
            record("value-ref", vec![string(self.value.value_ref())]),
        ])
    }

    pub fn assertion_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeObserver {
    pub actor: String,
    pub pattern: RuntimeValue,
}

impl RuntimeObserver {
    pub fn to_value(&self) -> IoValue {
        record("runtime-observer-v1", vec![
            string(&self.actor),
            self.pattern.as_iovalue().clone(),
            record("pattern-ref", vec![string(self.pattern.value_ref())]),
        ])
    }

    pub fn observer_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStep {
    Send {
        from: String,
        to: String,
        body: RuntimeValue,
    },
    Observe {
        actor: String,
        pattern: RuntimeValue,
    },
    Assert {
        actor: String,
        value: RuntimeValue,
    },
    Retract {
        actor: String,
        value: RuntimeValue,
    },
    Clock {
        actor: String,
    },
    Random {
        actor: String,
        upper: u64,
    },
}

impl RuntimeStep {
    pub fn to_value(&self) -> IoValue {
        match self {
            RuntimeStep::Send { from, to, body } => record("runtime-step-send-v1", vec![
                string(from),
                string(to),
                body.as_iovalue().clone(),
                record("body-ref", vec![string(body.value_ref())]),
            ]),
            RuntimeStep::Observe { actor, pattern } => record("runtime-step-observe-v1", vec![
                string(actor),
                pattern.as_iovalue().clone(),
                record("pattern-ref", vec![string(pattern.value_ref())]),
            ]),
            RuntimeStep::Assert { actor, value } => record("runtime-step-assert-v1", vec![
                string(actor),
                value.as_iovalue().clone(),
                record("value-ref", vec![string(value.value_ref())]),
            ]),
            RuntimeStep::Retract { actor, value } => record("runtime-step-retract-v1", vec![
                string(actor),
                value.as_iovalue().clone(),
                record("value-ref", vec![string(value.value_ref())]),
            ]),
            RuntimeStep::Clock { actor } => record("runtime-step-clock-v1", vec![string(actor)]),
            RuntimeStep::Random { actor, upper } => {
                record("runtime-step-random-v1", vec![string(actor), u64_value(*upper)])
            }
        }
    }

    pub fn step_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    pub fn actor_ids(&self) -> Vec<&str> {
        match self {
            RuntimeStep::Send { from, to, .. } => vec![from.as_str(), to.as_str()],
            RuntimeStep::Observe { actor, .. }
            | RuntimeStep::Assert { actor, .. }
            | RuntimeStep::Retract { actor, .. }
            | RuntimeStep::Clock { actor }
            | RuntimeStep::Random { actor, .. } => vec![actor.as_str()],
        }
    }

    pub fn primary_actor(&self) -> &str {
        match self {
            RuntimeStep::Send { from, .. } => from,
            RuntimeStep::Observe { actor, .. }
            | RuntimeStep::Assert { actor, .. }
            | RuntimeStep::Retract { actor, .. }
            | RuntimeStep::Clock { actor }
            | RuntimeStep::Random { actor, .. } => actor,
        }
    }
}

mod chronicle;
pub use chronicle::RuntimeEffect;
pub use chronicle::RuntimeEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTurn {
    pub(crate) actions: Vec<TurnAction>,
    pub(crate) events: Vec<RuntimeEvent>,
}

impl PendingTurn {
    pub(crate) fn new() -> Self {
        Self {
            actions: Vec::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TurnAction {
    Send(RuntimeMessage),
    Observe(RuntimeObserver),
    Assert(RuntimeAssertion),
    Retract(RuntimeAssertion),
}
