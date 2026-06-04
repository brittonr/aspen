use std::cmp::Ordering;
use std::fmt;

use preserves::IOValue;

use super::AdmissionDecision;
use super::AdmissionRequest;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;

#[derive(Clone)]
pub struct RuntimeValue {
    value: IOValue,
    canonical: Vec<u8>,
}

impl RuntimeValue {
    pub fn new(value: IOValue) -> Result<Self> {
        let canonical = canonical_bytes(&value)?;
        Ok(Self { value, canonical })
    }

    pub fn string(value: impl AsRef<str>) -> Result<Self> {
        Self::new(IOValue::new(value.as_ref().to_owned()))
    }

    pub fn as_iovalue(&self) -> &IOValue {
        &self.value
    }

    pub fn into_iovalue(self) -> IOValue {
        self.value
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
}

impl fmt::Debug for RuntimeValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeValue")
            .field("value", &self.value)
            .field("canonical_len", &self.canonical.len())
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAssertion {
    pub actor: String,
    pub value: RuntimeValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeObserver {
    pub actor: String,
    pub pattern: RuntimeValue,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    MessageDelivered {
        from: String,
        to: String,
        body: RuntimeValue,
    },
    ObserveRegistered {
        actor: String,
        pattern: RuntimeValue,
    },
    AssertionObserved {
        observer: String,
        owner: String,
        value: RuntimeValue,
    },
    AssertionCommitted {
        actor: String,
        value: RuntimeValue,
    },
    AssertionRetracted {
        actor: String,
        value: RuntimeValue,
    },
    AssertionRetractionObserved {
        observer: String,
        owner: String,
        value: RuntimeValue,
    },
    EffectRequest {
        effect: RuntimeEffect,
        actor: String,
        sequence: u64,
        upper: Option<u64>,
    },
    EffectResponse {
        effect: RuntimeEffect,
        actor: String,
        sequence: u64,
        upper: Option<u64>,
        value: u64,
    },
    AdmissionDecision {
        request: AdmissionRequest,
        decision: AdmissionDecision,
    },
    TurnRolledBack {
        actor: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEffect {
    Clock,
    Random,
}

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
