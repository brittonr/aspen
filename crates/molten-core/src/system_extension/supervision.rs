use super::CallbackKind;
use super::ExecutionProfile;
use super::MAX_SYSTEM_EXTENSION_ITEMS;
use super::duplicates;
use super::valid_ref;

const MAX_CALLBACK_CONCURRENCY: u64 = 1_024;
const MAX_QUEUED_EVENTS: u64 = 65_536;
const MAX_INFLIGHT_BYTES: u64 = 1_073_741_824;
const MAX_OPEN_STREAMS: u64 = 4_096;
const MAX_TIMERS: u64 = 65_536;
const MAX_EFFECT_REQUESTS: u64 = 65_536;
const MAX_LOGICAL_TICKS: u64 = 1_000_000_000;
const MAX_RESTART_ATTEMPTS: u64 = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverloadPolicy {
    Reject,
    Delay,
    UpstreamBackpressure,
}

impl OverloadPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::Delay => "delay",
            Self::UpstreamBackpressure => "upstream-backpressure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEnvelope {
    pub max_concurrent_callbacks: u64,
    pub max_queued_events: u64,
    pub max_inflight_bytes: u64,
    pub max_open_streams: u64,
    pub max_timers: u64,
    pub max_effect_requests: u64,
    pub callback_deadline_ticks: u64,
    pub shutdown_grace_ticks: u64,
    pub max_restart_attempts: u64,
    pub overload_policy: OverloadPolicy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceUsage {
    pub concurrent_callbacks: u64,
    pub queued_events: u64,
    pub inflight_bytes: u64,
    pub open_streams: u64,
    pub timers: u64,
    pub effect_requests: u64,
}

impl ResourceUsage {
    pub const fn is_idle(self) -> bool {
        self.concurrent_callbacks == 0
            && self.queued_events == 0
            && self.inflight_bytes == 0
            && self.open_streams == 0
            && self.timers == 0
            && self.effect_requests == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceIssue {
    ZeroLimit(&'static str),
    HardLimitExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    UsageExceedsEnvelope {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    CounterOverflow(&'static str),
    CounterUnderflow(&'static str),
}

pub fn validate_resource_envelope(envelope: &ResourceEnvelope) -> Vec<ResourceIssue> {
    let mut issues = Vec::new();
    validate_positive_limit(
        "max-concurrent-callbacks",
        envelope.max_concurrent_callbacks,
        MAX_CALLBACK_CONCURRENCY,
        &mut issues,
    );
    validate_limit("max-queued-events", envelope.max_queued_events, MAX_QUEUED_EVENTS, &mut issues);
    validate_positive_limit("max-inflight-bytes", envelope.max_inflight_bytes, MAX_INFLIGHT_BYTES, &mut issues);
    validate_limit("max-open-streams", envelope.max_open_streams, MAX_OPEN_STREAMS, &mut issues);
    validate_limit("max-timers", envelope.max_timers, MAX_TIMERS, &mut issues);
    validate_positive_limit("max-effect-requests", envelope.max_effect_requests, MAX_EFFECT_REQUESTS, &mut issues);
    validate_positive_limit(
        "callback-deadline-ticks",
        envelope.callback_deadline_ticks,
        MAX_LOGICAL_TICKS,
        &mut issues,
    );
    validate_positive_limit("shutdown-grace-ticks", envelope.shutdown_grace_ticks, MAX_LOGICAL_TICKS, &mut issues);
    validate_limit("max-restart-attempts", envelope.max_restart_attempts, MAX_RESTART_ATTEMPTS, &mut issues);
    issues
}

pub fn validate_resource_usage(envelope: &ResourceEnvelope, usage: ResourceUsage) -> Vec<ResourceIssue> {
    let mut issues = Vec::new();
    validate_usage("concurrent-callbacks", usage.concurrent_callbacks, envelope.max_concurrent_callbacks, &mut issues);
    validate_usage("queued-events", usage.queued_events, envelope.max_queued_events, &mut issues);
    validate_usage("inflight-bytes", usage.inflight_bytes, envelope.max_inflight_bytes, &mut issues);
    validate_usage("open-streams", usage.open_streams, envelope.max_open_streams, &mut issues);
    validate_usage("timers", usage.timers, envelope.max_timers, &mut issues);
    validate_usage("effect-requests", usage.effect_requests, envelope.max_effect_requests, &mut issues);
    issues
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDecision {
    Schedule,
    Queue,
    Reject,
    Backpressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceAdmissionPlan {
    pub decision: AdmissionDecision,
    pub next_usage: ResourceUsage,
}

// r[impl molten.system_extension.backpressure]
pub fn plan_resource_admission(
    envelope: &ResourceEnvelope,
    usage: ResourceUsage,
    accounted_bytes: u64,
) -> Result<ResourceAdmissionPlan, Vec<ResourceIssue>> {
    let mut issues = validate_resource_usage(envelope, usage);
    let next_bytes = match usage.inflight_bytes.checked_add(accounted_bytes) {
        Some(value) => value,
        None => {
            issues.push(ResourceIssue::CounterOverflow("inflight-bytes"));
            usage.inflight_bytes
        }
    };
    if next_bytes > envelope.max_inflight_bytes {
        issues.push(ResourceIssue::UsageExceedsEnvelope {
            field: "inflight-bytes",
            actual: next_bytes,
            maximum: envelope.max_inflight_bytes,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    if usage.concurrent_callbacks < envelope.max_concurrent_callbacks {
        let next_callbacks = usage
            .concurrent_callbacks
            .checked_add(1)
            .ok_or_else(|| vec![ResourceIssue::CounterOverflow("concurrent-callbacks")])?;
        return Ok(ResourceAdmissionPlan {
            decision: AdmissionDecision::Schedule,
            next_usage: ResourceUsage {
                concurrent_callbacks: next_callbacks,
                inflight_bytes: next_bytes,
                ..usage
            },
        });
    }

    match envelope.overload_policy {
        OverloadPolicy::Reject => Ok(ResourceAdmissionPlan {
            decision: AdmissionDecision::Reject,
            next_usage: usage,
        }),
        OverloadPolicy::UpstreamBackpressure => Ok(ResourceAdmissionPlan {
            decision: AdmissionDecision::Backpressure,
            next_usage: usage,
        }),
        OverloadPolicy::Delay if usage.queued_events < envelope.max_queued_events => {
            let next_queue = usage
                .queued_events
                .checked_add(1)
                .ok_or_else(|| vec![ResourceIssue::CounterOverflow("queued-events")])?;
            Ok(ResourceAdmissionPlan {
                decision: AdmissionDecision::Queue,
                next_usage: ResourceUsage {
                    queued_events: next_queue,
                    inflight_bytes: next_bytes,
                    ..usage
                },
            })
        }
        OverloadPolicy::Delay => Ok(ResourceAdmissionPlan {
            decision: AdmissionDecision::Reject,
            next_usage: usage,
        }),
    }
}

pub fn reserve_effect_requests(
    envelope: &ResourceEnvelope,
    usage: ResourceUsage,
    effect_count: usize,
) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let effect_count =
        u64::try_from(effect_count).map_err(|_| vec![ResourceIssue::CounterOverflow("effect-requests")])?;
    let next_effects = usage
        .effect_requests
        .checked_add(effect_count)
        .ok_or_else(|| vec![ResourceIssue::CounterOverflow("effect-requests")])?;
    if next_effects > envelope.max_effect_requests {
        return Err(vec![ResourceIssue::UsageExceedsEnvelope {
            field: "effect-requests",
            actual: next_effects,
            maximum: envelope.max_effect_requests,
        }]);
    }
    Ok(ResourceUsage {
        effect_requests: next_effects,
        ..usage
    })
}

pub fn release_effect_requests(usage: ResourceUsage, effect_count: usize) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let effect_count =
        u64::try_from(effect_count).map_err(|_| vec![ResourceIssue::CounterOverflow("effect-requests")])?;
    let effect_requests = usage
        .effect_requests
        .checked_sub(effect_count)
        .ok_or_else(|| vec![ResourceIssue::CounterUnderflow("effect-requests")])?;
    Ok(ResourceUsage {
        effect_requests,
        ..usage
    })
}

pub fn release_callback_resources(
    usage: ResourceUsage,
    accounted_bytes: u64,
    effect_count: usize,
) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let effect_count =
        u64::try_from(effect_count).map_err(|_| vec![ResourceIssue::CounterOverflow("effect-requests")])?;
    let concurrent_callbacks = usage
        .concurrent_callbacks
        .checked_sub(1)
        .ok_or_else(|| vec![ResourceIssue::CounterUnderflow("concurrent-callbacks")])?;
    let inflight_bytes = usage
        .inflight_bytes
        .checked_sub(accounted_bytes)
        .ok_or_else(|| vec![ResourceIssue::CounterUnderflow("inflight-bytes")])?;
    let effect_requests = usage
        .effect_requests
        .checked_sub(effect_count)
        .ok_or_else(|| vec![ResourceIssue::CounterUnderflow("effect-requests")])?;
    Ok(ResourceUsage {
        concurrent_callbacks,
        inflight_bytes,
        effect_requests,
        ..usage
    })
}

pub fn reserve_stream(envelope: &ResourceEnvelope, usage: ResourceUsage) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let open_streams = reserve_counter("open-streams", usage.open_streams, envelope.max_open_streams)?;
    Ok(ResourceUsage { open_streams, ..usage })
}

pub fn release_stream(usage: ResourceUsage) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let open_streams = release_counter("open-streams", usage.open_streams)?;
    Ok(ResourceUsage { open_streams, ..usage })
}

pub fn reserve_timer(envelope: &ResourceEnvelope, usage: ResourceUsage) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let timers = reserve_counter("timers", usage.timers, envelope.max_timers)?;
    Ok(ResourceUsage { timers, ..usage })
}

pub fn release_timer(usage: ResourceUsage) -> Result<ResourceUsage, Vec<ResourceIssue>> {
    let timers = release_counter("timers", usage.timers)?;
    Ok(ResourceUsage { timers, ..usage })
}

fn reserve_counter(field: &'static str, actual: u64, maximum: u64) -> Result<u64, Vec<ResourceIssue>> {
    let next = actual.checked_add(1).ok_or_else(|| vec![ResourceIssue::CounterOverflow(field)])?;
    if next > maximum {
        return Err(vec![ResourceIssue::UsageExceedsEnvelope {
            field,
            actual: next,
            maximum,
        }]);
    }
    Ok(next)
}

fn release_counter(field: &'static str, actual: u64) -> Result<u64, Vec<ResourceIssue>> {
    actual.checked_sub(1).ok_or_else(|| vec![ResourceIssue::CounterUnderflow(field)])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Unknown,
    Starting,
    Healthy,
    Degraded,
    Failed,
    Quarantined,
    Stopped,
}

impl HealthState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Starting => "starting",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
            Self::Quarantined => "quarantined",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    Fatal,
    PolicyViolation,
    ResourceViolation,
    GenerationViolation,
}

impl FailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Retryable => "retryable",
            Self::Fatal => "fatal",
            Self::PolicyViolation => "policy-violation",
            Self::ResourceViolation => "resource-violation",
            Self::GenerationViolation => "generation-violation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisionDecision {
    Restart,
    Quarantine,
}

pub fn plan_supervision(
    failure: FailureClass,
    restart_attempts: u64,
    max_restart_attempts: u64,
) -> SupervisionDecision {
    let is_retryable = failure == FailureClass::Retryable;
    if is_retryable && restart_attempts < max_restart_attempts {
        SupervisionDecision::Restart
    } else {
        SupervisionDecision::Quarantine
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackObservation {
    pub callback: CallbackKind,
    pub invocation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableConformanceInput {
    pub execution_profile: ExecutionProfile,
    pub required_callbacks: Vec<CallbackKind>,
    pub observations: Vec<CallbackObservation>,
    pub execution_binding_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableConformanceIssue {
    EmptyRequiredCallbacks,
    TooManyItems { field: &'static str, actual: usize },
    DuplicateRequiredCallback(CallbackKind),
    DuplicateObservation(CallbackKind),
    MissingInvocation(CallbackKind),
    MissingExecutionBinding,
    MalformedExecutionBindingRef(String),
}

// r[impl molten.system_extension.final_validation]
pub fn validate_executable_conformance(input: &ExecutableConformanceInput) -> Vec<ExecutableConformanceIssue> {
    let mut issues = Vec::new();
    if input.required_callbacks.is_empty() {
        issues.push(ExecutableConformanceIssue::EmptyRequiredCallbacks);
    }
    if input.required_callbacks.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(ExecutableConformanceIssue::TooManyItems {
            field: "required-callbacks",
            actual: input.required_callbacks.len(),
        });
    }
    if input.observations.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(ExecutableConformanceIssue::TooManyItems {
            field: "observations",
            actual: input.observations.len(),
        });
    }
    if input.execution_binding_refs.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(ExecutableConformanceIssue::TooManyItems {
            field: "execution-binding-refs",
            actual: input.execution_binding_refs.len(),
        });
    }
    if duplicates(&input.required_callbacks) {
        for callback in &input.required_callbacks {
            if input.required_callbacks.iter().filter(|item| *item == callback).count() > 1 {
                issues.push(ExecutableConformanceIssue::DuplicateRequiredCallback(*callback));
                break;
            }
        }
    }
    let observed_callbacks: Vec<_> = input.observations.iter().map(|item| item.callback).collect();
    if duplicates(&observed_callbacks) {
        for callback in observed_callbacks {
            if input.observations.iter().filter(|item| item.callback == callback).count() > 1 {
                issues.push(ExecutableConformanceIssue::DuplicateObservation(callback));
                break;
            }
        }
    }
    for callback in &input.required_callbacks {
        let invoked = input
            .observations
            .iter()
            .find(|item| item.callback == *callback)
            .is_some_and(|item| item.invocation_count > 0);
        if !invoked {
            issues.push(ExecutableConformanceIssue::MissingInvocation(*callback));
        }
    }
    if input.execution_binding_refs.is_empty() {
        issues.push(ExecutableConformanceIssue::MissingExecutionBinding);
    }
    for reference in &input.execution_binding_refs {
        if !valid_ref(reference) {
            issues.push(ExecutableConformanceIssue::MalformedExecutionBindingRef(reference.clone()));
        }
    }
    issues
}

fn validate_positive_limit(field: &'static str, actual: u64, maximum: u64, issues: &mut Vec<ResourceIssue>) {
    if actual == 0 {
        issues.push(ResourceIssue::ZeroLimit(field));
    }
    validate_limit(field, actual, maximum, issues);
}

fn validate_limit(field: &'static str, actual: u64, maximum: u64, issues: &mut Vec<ResourceIssue>) {
    if actual > maximum {
        issues.push(ResourceIssue::HardLimitExceeded { field, actual, maximum });
    }
}

fn validate_usage(field: &'static str, actual: u64, maximum: u64, issues: &mut Vec<ResourceIssue>) {
    if actual > maximum {
        issues.push(ResourceIssue::UsageExceedsEnvelope { field, actual, maximum });
    }
}
