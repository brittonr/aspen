type AdmissionDecision = super::AdmissionDecision;
type AdmissionRequest = super::AdmissionRequest;
type IoValue = super::IoValue;
type Result<T> = super::Result<T>;
type RuntimeValue = super::RuntimeValue;

fn canonical_hash(value: &IoValue) -> Result<String> {
    super::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    super::record(label, fields)
}

fn string(value: impl AsRef<str>) -> IoValue {
    super::string(value)
}

fn u64_value(value: u64) -> IoValue {
    super::u64_value(value)
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

impl RuntimeEffect {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeEffect::Clock => "clock",
            RuntimeEffect::Random => "random",
        }
    }
}

// r[impl molten.runtime_spine.canonical_content_refs.runtime_values]
impl RuntimeEvent {
    pub fn to_value(&self) -> IoValue {
        match self {
            RuntimeEvent::MessageDelivered { from, to, body } => record("runtime-event-message-delivered-v1", vec![
                string(from),
                string(to),
                body.as_iovalue().clone(),
                record("body-ref", vec![string(body.value_ref())]),
            ]),
            RuntimeEvent::ObserveRegistered { actor, pattern } => record("runtime-event-observe-registered-v1", vec![
                string(actor),
                pattern.as_iovalue().clone(),
                record("pattern-ref", vec![string(pattern.value_ref())]),
            ]),
            RuntimeEvent::AssertionObserved { observer, owner, value } => {
                record("runtime-event-assertion-observed-v1", vec![
                    string(observer),
                    string(owner),
                    value.as_iovalue().clone(),
                    record("value-ref", vec![string(value.value_ref())]),
                ])
            }
            RuntimeEvent::AssertionCommitted { actor, value } => record("runtime-event-assertion-committed-v1", vec![
                string(actor),
                value.as_iovalue().clone(),
                record("value-ref", vec![string(value.value_ref())]),
            ]),
            RuntimeEvent::AssertionRetracted { actor, value } => record("runtime-event-assertion-retracted-v1", vec![
                string(actor),
                value.as_iovalue().clone(),
                record("value-ref", vec![string(value.value_ref())]),
            ]),
            RuntimeEvent::AssertionRetractionObserved { observer, owner, value } => {
                record("runtime-event-assertion-retraction-observed-v1", vec![
                    string(observer),
                    string(owner),
                    value.as_iovalue().clone(),
                    record("value-ref", vec![string(value.value_ref())]),
                ])
            }
            RuntimeEvent::EffectRequest {
                effect,
                actor,
                sequence,
                upper,
            } => optional_upper_event_value(&OptionalUpperEventValueInput {
                name: "runtime-event-effect-request-v1",
                effect,
                actor,
                sequence: *sequence,
                upper: *upper,
                value: None,
            }),
            RuntimeEvent::EffectResponse {
                effect,
                actor,
                sequence,
                upper,
                value,
            } => optional_upper_event_value(&OptionalUpperEventValueInput {
                name: "runtime-event-effect-response-v1",
                effect,
                actor,
                sequence: *sequence,
                upper: *upper,
                value: Some(*value),
            }),
            RuntimeEvent::AdmissionDecision { request, decision } => {
                record("runtime-event-admission-decision-v1", vec![
                    admission_request_ref_value(request),
                    record("decision", vec![string(decision.status()), string(decision.reason())]),
                ])
            }
            RuntimeEvent::TurnRolledBack { actor, reason } => {
                record("runtime-event-turn-rolled-back-v1", vec![string(actor), string(reason)])
            }
        }
    }

    pub fn event_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }
}

struct OptionalUpperEventValueInput<'a> {
    name: &'static str,
    effect: &'a RuntimeEffect,
    actor: &'a str,
    sequence: u64,
    upper: Option<u64>,
    value: Option<u64>,
}

fn optional_upper_event_value(input: &OptionalUpperEventValueInput<'_>) -> IoValue {
    let mut fields = vec![
        string(input.effect.as_str()),
        string(input.actor),
        u64_value(input.sequence),
    ];
    if let Some(upper) = input.upper {
        fields.push(u64_value(upper));
    }
    if let Some(value) = input.value {
        fields.push(u64_value(value));
    }
    record(input.name, fields)
}

fn admission_request_ref_value(request: &AdmissionRequest) -> IoValue {
    let mut fields = vec![string(&request.actor), string(request.action.as_str())];
    if let Some(target) = request.target.as_ref() {
        fields.push(record("target", vec![string(target)]));
    }
    if let Some(value) = request.value.as_ref() {
        fields.push(record("value-ref", vec![string(value.value_ref())]));
    }
    if let Some(upper) = request.upper {
        fields.push(record("upper", vec![u64_value(upper)]));
    }
    record("runtime-admission-request-ref-v1", fields)
}
