use super::*;
use crate::fabric::valid_blake3_ref;

// r[impl molten.fabric_observability.failure_semantics]
pub fn evaluate_adapter_delivery(
    observation_profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    request: &AdapterDeliveryRequest,
    runtime: &AdapterRuntimeObservation,
    last_export_tick: Option<u64>,
) -> AdapterOutcome {
    let mut issues = validate_adapter_profile(observation_profile, adapter);
    validate_delivery_request(observation_profile, adapter, request, last_export_tick, &mut issues);
    let kind = adapter_outcome_kind(adapter, request, runtime, &mut issues);
    AdapterOutcome {
        operation_ref: request.operation_ref.clone(),
        adapter_ref: adapter.adapter_ref.clone(),
        payload_ref: request.payload_ref.clone(),
        kind,
        dropped_observations: runtime.dropped_observations,
        service_policy_signal: adapter.required && kind != AdapterOutcomeKind::Exported,
        issues,
    }
}

pub fn validate_adapter_outcome(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    outcome: &AdapterOutcome,
) -> Vec<ObservabilityIssue> {
    let mut issues = validate_adapter_profile(profile, adapter);
    for (field, reference) in [
        ("adapter-outcome-operation-ref", outcome.operation_ref.as_str()),
        ("adapter-outcome-adapter-ref", outcome.adapter_ref.as_str()),
        ("adapter-outcome-payload-ref", outcome.payload_ref.as_str()),
    ] {
        if !valid_blake3_ref(reference) {
            issues.push(ObservabilityIssue::MalformedRef(field));
        }
    }
    if outcome.adapter_ref != adapter.adapter_ref {
        issues.push(ObservabilityIssue::AdapterMismatch);
    }
    if outcome.issues.len() > profile.bounds.max_diagnostics {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("adapter-outcome-issues"));
    }
    issues
}

pub fn validate_adapter_status(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    status: &ObservationAdapterStatus,
) -> Vec<ObservabilityIssue> {
    let mut issues = validate_adapter_profile(profile, adapter);
    if status.schema != OBSERVATION_ADAPTER_STATUS_SCHEMA {
        issues.push(ObservabilityIssue::SchemaMismatch("adapter-status-schema"));
    }
    if status.adapter_ref != adapter.adapter_ref || status.class != adapter.class {
        issues.push(ObservabilityIssue::AdapterMismatch);
    }
    if !valid_blake3_ref(&status.adapter_ref) {
        issues.push(ObservabilityIssue::MalformedRef("adapter-status-ref"));
    }
    if status.queued_bytes > adapter.max_queued_bytes {
        issues.push(ObservabilityIssue::QueueBoundExceeded);
    }
    if status.evidence_refs.iter().any(|reference| !valid_blake3_ref(reference)) {
        issues.push(ObservabilityIssue::MalformedRef("adapter-status-evidence-ref"));
    }
    if status.issues.len() > profile.bounds.max_diagnostics {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("adapter-status-issues"));
    }
    issues
}

pub fn adapter_status_from_outcome(
    adapter: &ObservationAdapterProfile,
    outcome: &AdapterOutcome,
    runtime: &AdapterRuntimeObservation,
) -> ObservationAdapterStatus {
    ObservationAdapterStatus {
        schema: OBSERVATION_ADAPTER_STATUS_SCHEMA.to_string(),
        adapter_ref: adapter.adapter_ref.clone(),
        class: adapter.class,
        kind: outcome.kind,
        observed_tick: runtime.completed_tick,
        queued_bytes: runtime.queued_bytes,
        dropped_observations: runtime.dropped_observations,
        evidence_refs: adapter.evidence_refs.clone(),
        issues: outcome.issues.clone(),
    }
}

fn validate_delivery_request(
    observation_profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    request: &AdapterDeliveryRequest,
    last_export_tick: Option<u64>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    for (field, reference) in [
        ("adapter-operation-ref", request.operation_ref.as_str()),
        ("adapter-request-adapter-ref", request.adapter_ref.as_str()),
        ("adapter-payload-ref", request.payload_ref.as_str()),
    ] {
        if !valid_blake3_ref(reference) {
            issues.push(ObservabilityIssue::MalformedRef(field));
        }
    }
    if request.adapter_ref != adapter.adapter_ref {
        issues.push(ObservabilityIssue::AdapterMismatch);
    }
    if request.payload_bytes == 0 {
        issues.push(ObservabilityIssue::ZeroBound("adapter-payload-bytes"));
    }
    if request.payload_bytes > adapter.max_queued_bytes
        || request.payload_bytes > observation_profile.bounds.max_queued_bytes
    {
        issues.push(ObservabilityIssue::QueueBoundExceeded);
    }
    if request.deadline_tick < request.submitted_tick {
        issues.push(ObservabilityIssue::DeadlineExceeded);
    }
    match request.submitted_tick.checked_add(adapter.timeout_ticks) {
        Some(maximum_deadline) if request.deadline_tick > maximum_deadline => {
            issues.push(ObservabilityIssue::DeadlineExceeded);
        }
        Some(_) => {}
        None => issues.push(ObservabilityIssue::ArithmeticOverflow),
    }
    if let Some(last_tick) = last_export_tick {
        match last_tick.checked_add(observation_profile.bounds.min_export_interval_ticks) {
            Some(next_tick) if request.submitted_tick < next_tick => {
                issues.push(ObservabilityIssue::ExportFrequencyExceeded);
            }
            Some(_) => {}
            None => issues.push(ObservabilityIssue::ArithmeticOverflow),
        }
    }
}

fn adapter_outcome_kind(
    adapter: &ObservationAdapterProfile,
    request: &AdapterDeliveryRequest,
    runtime: &AdapterRuntimeObservation,
    issues: &mut Vec<ObservabilityIssue>,
) -> AdapterOutcomeKind {
    if let Some(failure) = runtime.failure {
        return failure_outcome(failure, issues);
    }
    if runtime.cancelled {
        issues.push(ObservabilityIssue::Cancelled);
        return AdapterOutcomeKind::Cancelled;
    }
    if !runtime.available {
        issues.push(ObservabilityIssue::ExporterUnavailable);
        return AdapterOutcomeKind::Unavailable;
    }
    if issues.iter().any(|issue| {
        matches!(
            issue,
            ObservabilityIssue::AdapterMismatch
                | ObservabilityIssue::MalformedRef(_)
                | ObservabilityIssue::ZeroBound(_)
                | ObservabilityIssue::ArithmeticOverflow
        )
    }) {
        return AdapterOutcomeKind::Failed;
    }
    if issues.contains(&ObservabilityIssue::ExportFrequencyExceeded) {
        return AdapterOutcomeKind::Stale;
    }
    if runtime.completed_tick > request.deadline_tick {
        issues.push(ObservabilityIssue::DeadlineExceeded);
        return AdapterOutcomeKind::Timeout;
    }
    let queued_with_payload = runtime.queued_bytes.checked_add(request.payload_bytes);
    match queued_with_payload {
        Some(total) if total > adapter.max_queued_bytes => {
            issues.push(ObservabilityIssue::QueueBoundExceeded);
            if adapter.drop_on_backpressure {
                issues.push(ObservabilityIssue::ObservationDropped);
                return AdapterOutcomeKind::Dropped;
            }
            return AdapterOutcomeKind::Backpressure;
        }
        Some(_) => {}
        None => {
            issues.push(ObservabilityIssue::ArithmeticOverflow);
            return AdapterOutcomeKind::Failed;
        }
    }
    if runtime.dropped_observations > 0 {
        issues.push(ObservabilityIssue::ObservationDropped);
        AdapterOutcomeKind::Dropped
    } else if issues.contains(&ObservabilityIssue::QueueBoundExceeded)
        || issues.contains(&ObservabilityIssue::DeadlineExceeded)
    {
        AdapterOutcomeKind::Failed
    } else {
        AdapterOutcomeKind::Exported
    }
}

fn failure_outcome(failure: AdapterFailureClass, issues: &mut Vec<ObservabilityIssue>) -> AdapterOutcomeKind {
    match failure {
        AdapterFailureClass::PermissionDenied => {
            issues.push(ObservabilityIssue::PermissionDenied);
            AdapterOutcomeKind::PermissionDenied
        }
        AdapterFailureClass::UnsupportedCapability => {
            issues.push(ObservabilityIssue::UnsupportedCapability);
            AdapterOutcomeKind::Unsupported
        }
        AdapterFailureClass::CorruptInput => {
            issues.push(ObservabilityIssue::CorruptInput);
            AdapterOutcomeKind::Corrupt
        }
        AdapterFailureClass::AdapterFailure => {
            issues.push(ObservabilityIssue::AdapterFailure);
            AdapterOutcomeKind::Failed
        }
    }
}
