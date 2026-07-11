use super::AdmissionDecision;
use super::AdmittedSystemExtensionManifest;
use super::CallbackKind;
use super::HealthState;
use super::LifecyclePhase;
use super::LifecycleState;
use super::MAX_SYSTEM_EXTENSION_ITEMS;
use super::ResourceIssue;
use super::ResourceUsage;
use super::duplicates;
use super::plan_resource_admission;
use super::valid_ref;
use super::valid_token;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricPortRequirement;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackEvent {
    pub callback: CallbackKind,
    pub generation: u64,
    pub event_ref: String,
    pub payload_ref: Option<String>,
    pub accounted_bytes: u64,
    pub logical_tick: u64,
    pub deadline_tick: Option<u64>,
    pub cancellation_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackInvocation {
    pub callback: CallbackKind,
    pub generation: u64,
    pub sequence: u64,
    pub event_ref: String,
    pub payload_ref: Option<String>,
    pub logical_tick: u64,
    pub deadline_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackDispatchPlan {
    pub decision: AdmissionDecision,
    pub invocation: Option<CallbackInvocation>,
    pub next_usage: ResourceUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchIssue {
    CallbackNotDeclared(CallbackKind),
    CallbackIllegalInPhase {
        callback: CallbackKind,
        phase: LifecyclePhase,
    },
    StaleGeneration {
        actual: u64,
        active: u64,
    },
    MalformedEventRef(String),
    MalformedPayloadRef(String),
    MissingDeadline,
    DeadlineExpired {
        deadline: u64,
        logical_tick: u64,
    },
    DeadlineExceedsEnvelope {
        deadline: u64,
        maximum: u64,
    },
    DeadlineOverflow,
    CancellationRequested,
    SequenceOverflow,
    Resource(ResourceIssue),
}

// r[impl molten.system_extension.callbacks]
// r[impl molten.system_extension.lifecycle]
pub fn plan_callback_dispatch(
    manifest: &AdmittedSystemExtensionManifest,
    state: &LifecycleState,
    usage: ResourceUsage,
    event: &CallbackEvent,
    invocation_sequence: u64,
) -> Result<CallbackDispatchPlan, Vec<DispatchIssue>> {
    let issues = validate_callback_event(manifest, state, event);
    if !issues.is_empty() {
        return Err(issues);
    }
    let resource_plan = plan_resource_admission(&manifest.resources, usage, event.accounted_bytes)
        .map_err(|resource_issues| resource_issues.into_iter().map(DispatchIssue::Resource).collect::<Vec<_>>())?;
    if resource_plan.decision != AdmissionDecision::Schedule {
        return Ok(CallbackDispatchPlan {
            decision: resource_plan.decision,
            invocation: None,
            next_usage: resource_plan.next_usage,
        });
    }
    let sequence = invocation_sequence.checked_add(1).ok_or_else(|| vec![DispatchIssue::SequenceOverflow])?;
    let deadline_tick = event.deadline_tick.ok_or_else(|| vec![DispatchIssue::MissingDeadline])?;
    Ok(CallbackDispatchPlan {
        decision: AdmissionDecision::Schedule,
        invocation: Some(CallbackInvocation {
            callback: event.callback,
            generation: event.generation,
            sequence,
            event_ref: event.event_ref.clone(),
            payload_ref: event.payload_ref.clone(),
            logical_tick: event.logical_tick,
            deadline_tick,
        }),
        next_usage: resource_plan.next_usage,
    })
}

fn validate_callback_event(
    manifest: &AdmittedSystemExtensionManifest,
    state: &LifecycleState,
    event: &CallbackEvent,
) -> Vec<DispatchIssue> {
    let mut issues = Vec::new();
    if !manifest.declares_callback(event.callback) {
        issues.push(DispatchIssue::CallbackNotDeclared(event.callback));
    }
    if !callback_allowed_in_phase(event.callback, state.phase) {
        issues.push(DispatchIssue::CallbackIllegalInPhase {
            callback: event.callback,
            phase: state.phase,
        });
    }
    if event.generation != state.generation {
        issues.push(DispatchIssue::StaleGeneration {
            actual: event.generation,
            active: state.generation,
        });
    }
    if !valid_ref(&event.event_ref) {
        issues.push(DispatchIssue::MalformedEventRef(event.event_ref.clone()));
    }
    if let Some(payload_ref) = &event.payload_ref
        && !valid_ref(payload_ref)
    {
        issues.push(DispatchIssue::MalformedPayloadRef(payload_ref.clone()));
    }
    match event.deadline_tick {
        None => issues.push(DispatchIssue::MissingDeadline),
        Some(deadline) if deadline < event.logical_tick => issues.push(DispatchIssue::DeadlineExpired {
            deadline,
            logical_tick: event.logical_tick,
        }),
        Some(deadline) => match event.logical_tick.checked_add(manifest.resources.callback_deadline_ticks) {
            None => issues.push(DispatchIssue::DeadlineOverflow),
            Some(maximum) if deadline > maximum => {
                issues.push(DispatchIssue::DeadlineExceedsEnvelope { deadline, maximum });
            }
            Some(_) => {}
        },
    }
    if event.cancellation_requested {
        issues.push(DispatchIssue::CancellationRequested);
    }
    issues
}

fn callback_allowed_in_phase(callback: CallbackKind, phase: LifecyclePhase) -> bool {
    match callback {
        CallbackKind::Initialize => phase == LifecyclePhase::Initializing,
        CallbackKind::Start => phase == LifecyclePhase::Starting,
        CallbackKind::Request
        | CallbackKind::Message
        | CallbackKind::StreamOpen
        | CallbackKind::StreamEvent
        | CallbackKind::Timer
        | CallbackKind::Health => phase == LifecyclePhase::Running,
        CallbackKind::Checkpoint => phase == LifecyclePhase::Checkpointing,
        CallbackKind::Recover => {
            matches!(phase, LifecyclePhase::Recovering | LifecyclePhase::Upgrading | LifecyclePhase::RollingBack)
        }
        CallbackKind::Drain => phase == LifecyclePhase::Draining,
        CallbackKind::Shutdown => phase == LifecyclePhase::ShuttingDown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientEffect {
    Filesystem,
    Network,
    Clock,
    Randomness,
    Process,
    Environment,
}

impl AmbientEffect {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Network => "network",
            Self::Clock => "clock",
            Self::Randomness => "randomness",
            Self::Process => "process",
            Self::Environment => "environment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectTarget {
    FabricPort(FabricPortKey),
    Ambient(AmbientEffect),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedEffectRequest {
    pub target: EffectTarget,
    pub operation: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub request_ref: String,
    pub generation: u64,
    pub accounted_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectIssue {
    TooManyEffects { actual: usize, maximum: usize },
    DuplicateRequestRef(String),
    StaleGeneration { actual: u64, active: u64 },
    AmbientEffectDenied(AmbientEffect),
    PortNotBound(FabricPortKey),
    OperationNotAdmitted { key: FabricPortKey, operation: String },
    InputSchemaNotAdmitted { key: FabricPortKey, schema_ref: String },
    OutputSchemaNotAdmitted { key: FabricPortKey, schema_ref: String },
    MalformedOperation(String),
    MalformedSchemaRef { field: &'static str, value: String },
    MalformedRef { field: &'static str, value: String },
    BytesExceedEnvelope { actual: u64, maximum: u64 },
}

// r[impl molten.system_extension.typed_effects]
pub fn validate_typed_effects(
    manifest: &AdmittedSystemExtensionManifest,
    active_generation: u64,
    effects: &[TypedEffectRequest],
) -> Vec<EffectIssue> {
    let mut issues = Vec::new();
    if effects.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(EffectIssue::TooManyEffects {
            actual: effects.len(),
            maximum: MAX_SYSTEM_EXTENSION_ITEMS,
        });
    }
    let request_refs: Vec<_> = effects.iter().map(|effect| effect.request_ref.clone()).collect();
    if duplicates(&request_refs) {
        for request_ref in request_refs {
            if effects.iter().filter(|effect| effect.request_ref == request_ref).count() > 1 {
                issues.push(EffectIssue::DuplicateRequestRef(request_ref));
                break;
            }
        }
    }
    for effect in effects {
        validate_effect(manifest, active_generation, effect, &mut issues);
    }
    issues
}

fn validate_effect(
    manifest: &AdmittedSystemExtensionManifest,
    active_generation: u64,
    effect: &TypedEffectRequest,
    issues: &mut Vec<EffectIssue>,
) {
    if effect.generation != active_generation {
        issues.push(EffectIssue::StaleGeneration {
            actual: effect.generation,
            active: active_generation,
        });
    }
    if !valid_token(&effect.operation) {
        issues.push(EffectIssue::MalformedOperation(effect.operation.clone()));
    }
    for (field, schema_ref) in [
        ("input-schema-ref", effect.input_schema_ref.as_str()),
        ("output-schema-ref", effect.output_schema_ref.as_str()),
    ] {
        if !valid_token(schema_ref) {
            issues.push(EffectIssue::MalformedSchemaRef {
                field,
                value: schema_ref.to_string(),
            });
        }
    }
    if !valid_ref(&effect.request_ref) {
        issues.push(EffectIssue::MalformedRef {
            field: "request-ref",
            value: effect.request_ref.clone(),
        });
    }
    if effect.accounted_bytes > manifest.resources.max_inflight_bytes {
        issues.push(EffectIssue::BytesExceedEnvelope {
            actual: effect.accounted_bytes,
            maximum: manifest.resources.max_inflight_bytes,
        });
    }

    let key = match &effect.target {
        EffectTarget::Ambient(ambient) => {
            issues.push(EffectIssue::AmbientEffectDenied(*ambient));
            return;
        }
        EffectTarget::FabricPort(key) => key,
    };
    let is_bound = manifest
        .required_port_bindings
        .iter()
        .chain(manifest.optional_port_bindings.iter())
        .any(|binding| binding.key == *key);
    if !is_bound {
        issues.push(EffectIssue::PortNotBound(key.clone()));
        return;
    }
    let Some(requirement) = find_requirement(manifest, key) else {
        issues.push(EffectIssue::PortNotBound(key.clone()));
        return;
    };
    if !requirement.operation_classes.contains(&effect.operation) {
        issues.push(EffectIssue::OperationNotAdmitted {
            key: key.clone(),
            operation: effect.operation.clone(),
        });
    }
    if !requirement.input_schema_refs.contains(&effect.input_schema_ref) {
        issues.push(EffectIssue::InputSchemaNotAdmitted {
            key: key.clone(),
            schema_ref: effect.input_schema_ref.clone(),
        });
    }
    if !requirement.output_schema_refs.contains(&effect.output_schema_ref) {
        issues.push(EffectIssue::OutputSchemaNotAdmitted {
            key: key.clone(),
            schema_ref: effect.output_schema_ref.clone(),
        });
    }
}

fn find_requirement<'a>(
    manifest: &'a AdmittedSystemExtensionManifest,
    key: &FabricPortKey,
) -> Option<&'a FabricPortRequirement> {
    manifest
        .all_port_requirements()
        .find(|requirement| requirement.port_id == key.port_id && requirement.version == key.version)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackOutcome {
    pub output_refs: Vec<String>,
    pub effects: Vec<TypedEffectRequest>,
    pub state_ref: Option<String>,
    pub checkpoint_ref: Option<String>,
    pub health: HealthState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackOutcomeIssue {
    TooManyItems { field: &'static str, actual: usize },
    DuplicateRef { field: &'static str, value: String },
    MalformedRef { field: &'static str, value: String },
    MissingCheckpointRef,
    MissingStateRef,
    Effect(EffectIssue),
}

pub fn validate_callback_outcome(
    manifest: &AdmittedSystemExtensionManifest,
    invocation: &CallbackInvocation,
    outcome: &CallbackOutcome,
) -> Vec<CallbackOutcomeIssue> {
    let mut issues = Vec::new();
    validate_outcome_refs("output-refs", &outcome.output_refs, &mut issues);
    for (field, reference) in [
        ("state-ref", outcome.state_ref.as_ref()),
        ("checkpoint-ref", outcome.checkpoint_ref.as_ref()),
    ] {
        if let Some(reference) = reference
            && !valid_ref(reference)
        {
            issues.push(CallbackOutcomeIssue::MalformedRef {
                field,
                value: reference.clone(),
            });
        }
    }
    if invocation.callback == CallbackKind::Checkpoint && outcome.checkpoint_ref.is_none() {
        issues.push(CallbackOutcomeIssue::MissingCheckpointRef);
    }
    if invocation.callback == CallbackKind::Recover && outcome.state_ref.is_none() {
        issues.push(CallbackOutcomeIssue::MissingStateRef);
    }
    for issue in validate_typed_effects(manifest, invocation.generation, &outcome.effects) {
        issues.push(CallbackOutcomeIssue::Effect(issue));
    }
    issues
}

fn validate_outcome_refs(field: &'static str, refs: &[String], issues: &mut Vec<CallbackOutcomeIssue>) {
    if refs.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(CallbackOutcomeIssue::TooManyItems {
            field,
            actual: refs.len(),
        });
    }
    if duplicates(refs) {
        for reference in refs {
            if refs.iter().filter(|item| *item == reference).count() > 1 {
                issues.push(CallbackOutcomeIssue::DuplicateRef {
                    field,
                    value: reference.clone(),
                });
                break;
            }
        }
    }
    for reference in refs {
        if !valid_ref(reference) {
            issues.push(CallbackOutcomeIssue::MalformedRef {
                field,
                value: reference.clone(),
            });
        }
    }
}
