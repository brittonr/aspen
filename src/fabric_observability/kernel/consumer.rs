use super::*;

pub struct EbpfReadCapability<'a> {
    pub root: &'a crate::node_state::Scope,
    pub leaf: crate::node_state::RelativePath,
    /// Signed exact bytes for copied node endpoints; absent only for direct provider capabilities.
    pub content_ref: Option<&'a str>,
}

pub struct EbpfServiceControl {
    pub epoch: u64,
    pub now: u64,
    pub deadline: u64,
    pub cancelled: bool,
}

pub struct EbpfServiceInput<'a> {
    pub profile: &'a ObservationProfile,
    pub adapter: &'a EbpfProfile,
    pub capability: Option<EbpfReadCapability<'a>>,
    pub control: EbpfServiceControl,
    pub context: ObservationContext,
}

pub struct EbpfServiceObservation {
    pub samples: Vec<Artifact<MetricSample>>,
    pub health: Artifact<HealthInput>,
    pub status: Artifact<ObservationAdapterStatus>,
    pub outcome: Artifact<AdapterOutcome>,
    /// Finite adapter lifecycle event only; never a raw kernel record event.
    pub event: Artifact<ObservationEvent>,
    pub deny_required_readiness: bool,
    pub accounting: Option<EbpfAccounting>,
}

// r[impl linux_ebpf_observability.boundary]
// r[impl linux_ebpf_observability.failure]
pub fn observe_linux_ebpf(input: EbpfServiceInput<'_>) -> crate::error::Result<EbpfServiceObservation> {
    validate_ebpf_profile(input.profile, input.adapter).map_err(|_| invalid("Linux profile denied"))?;
    if input.context.scope != ClaimScope::LocalComponent || input.context.resource_ref != input.adapter.binding.scope {
        return Err(invalid("Linux observation scope denied"));
    }
    let mut context = input.context;
    context.observed_tick = input.control.now;
    context.valid_until_tick = input
        .control
        .now
        .checked_add(input.adapter.freshness_ticks)
        .ok_or_else(|| invalid("Linux freshness overflow"))?;
    let result = read_selected(input.capability, input.profile, input.adapter, &input.control);
    let (artifacts, fact, dropped, accounting) = match result {
        Ok(readback) => {
            context.source_ref = readback.endpoint_ref.clone();
            context.observed_tick = readback.window.end;
            context.valid_until_tick = readback
                .window
                .end
                .checked_add(input.adapter.freshness_ticks)
                .ok_or_else(|| invalid("Linux freshness overflow"))?;
            bind_evidence(&mut context, input.adapter, readback.endpoint_ref)?;
            let artifacts =
                canonical_ebpf_window(input.profile, input.adapter, &readback.window, &context, input.control.now)?;
            let accounting =
                readback.window.accounting.as_ref().ok_or_else(|| invalid("Linux accounting unavailable"))?;
            let dropped = accounting
                .producer_dropped
                .checked_add(accounting.consumer_dropped)
                .and_then(|count| count.checked_add(accounting.pending))
                .ok_or_else(|| invalid("Linux loss overflow"))?;
            let fact = if artifacts.complete {
                EbpfLifecycleFact::Complete
            } else {
                EbpfLifecycleFact::Partial
            };
            (Some(artifacts), fact, dropped, readback.window.accounting)
        }
        Err(fact) => (None, fact, 0, None),
    };
    let decision = decide_ebpf_service(input.adapter.required, fact);
    compose(input.profile, input.adapter, context, artifacts, (decision, dropped, accounting))
}

fn read_selected(
    capability: Option<EbpfReadCapability<'_>>,
    profile: &ObservationProfile,
    adapter: &EbpfProfile,
    control: &EbpfServiceControl,
) -> Result<EbpfEndpointReadback, EbpfLifecycleFact> {
    if !cfg!(target_os = "linux") {
        return Err(EbpfLifecycleFact::Unsupported);
    }
    if control.cancelled {
        return Err(EbpfLifecycleFact::Cancelled);
    }
    let capability = capability.ok_or(EbpfLifecycleFact::Unavailable)?;
    let selection = EbpfEndpointSelection {
        epoch: control.epoch,
        deadline: control.deadline,
        content_ref: capability.content_ref,
    };
    let mut reader = EbpfEndpointReader::open(capability.root, capability.leaf, profile, adapter, selection)
        .map_err(endpoint_fact)?;
    reader.poll(control.now, control.cancelled).map_err(endpoint_fact)
}

fn endpoint_fact(failure: EbpfEndpointFailure) -> EbpfLifecycleFact {
    match failure {
        EbpfEndpointFailure::Timeout => EbpfLifecycleFact::Timeout,
        EbpfEndpointFailure::Read(issue) => match issue {
            EbpfReadFailure::Missing => EbpfLifecycleFact::Unavailable,
            EbpfReadFailure::Permission => EbpfLifecycleFact::Permission,
            EbpfReadFailure::OverBound => EbpfLifecycleFact::Bounds,
            EbpfReadFailure::Malformed => EbpfLifecycleFact::Malformed,
            EbpfReadFailure::Binding => EbpfLifecycleFact::Binding,
            EbpfReadFailure::Cancelled => EbpfLifecycleFact::Cancelled,
            EbpfReadFailure::Closed => EbpfLifecycleFact::Closed,
        },
        EbpfEndpointFailure::Admission(issue) => match issue {
            EbpfIssue::Stale => EbpfLifecycleFact::Stale,
            EbpfIssue::Bounds => EbpfLifecycleFact::Bounds,
            EbpfIssue::Sequence => EbpfLifecycleFact::Sequence,
            EbpfIssue::Accounting => EbpfLifecycleFact::Accounting,
            _ => EbpfLifecycleFact::Binding,
        },
    }
}

fn bind_evidence(
    context: &mut ObservationContext,
    adapter: &EbpfProfile,
    endpoint_ref: String,
) -> crate::error::Result<()> {
    let binding = &adapter.binding;
    context.evidence_refs = vec![
        canonical_ebpf_binding_ref(binding)?,
        binding.program.clone(),
        binding.object.clone(),
        binding.btf.clone(),
        binding.scope.clone(),
        binding.accounting.clone(),
        binding.adapter.clone(),
        endpoint_ref,
    ];
    context.evidence_refs.sort();
    context.evidence_refs.dedup();
    Ok(())
}

fn compose(
    profile: &ObservationProfile,
    source: &EbpfProfile,
    context: ObservationContext,
    artifacts: Option<EbpfArtifacts>,
    outcome: (EbpfServiceDecision, u64, Option<EbpfAccounting>),
) -> crate::error::Result<EbpfServiceObservation> {
    let (decision, dropped, accounting) = outcome;
    let adapter = reader_adapter_profile(profile, source, &context);
    let issues = if accounting.is_none() {
        vec![ObservabilityIssue::ObservationUnavailable(
            "linux-ebpf-accounting".into(),
        )]
    } else {
        Vec::new()
    };
    let status = adapter_status(profile, &adapter, &ObservationAdapterStatus {
        schema: OBSERVATION_ADAPTER_STATUS_SCHEMA.into(),
        adapter_ref: adapter.adapter_ref.clone(),
        class: adapter.class,
        kind: decision.kind,
        observed_tick: context.observed_tick,
        queued_bytes: 0,
        dropped_observations: dropped,
        evidence_refs: context.evidence_refs.clone(),
        issues: issues.clone(),
    })?;
    let outcome = reader_adapter_outcome(profile, &adapter, &context, (decision, dropped), issues)?;
    let event = observation_event(
        profile,
        &ObservationEvent {
            schema: OBSERVATION_EVENT_SCHEMA.into(),
            event_ref: status.artifact_ref.clone(),
            event_kind: "linux-observation-status".into(),
            severity: if decision.deny_required_readiness {
                EventSeverity::Warning
            } else {
                EventSeverity::Info
            },
            context: context.clone(),
            detail: decision.kind.as_str().into(),
            attributes: Vec::new(),
        },
        context.observed_tick,
    )?;
    let (samples, health) = match artifacts {
        Some(artifacts) => (artifacts.samples, artifacts.health),
        None => (
            Vec::new(),
            health_input(
                profile,
                &HealthInput {
                    schema: HEALTH_INPUT_SCHEMA.into(),
                    health_ref: status.artifact_ref.clone(),
                    context: context.clone(),
                    state: decision.health,
                    diagnostic_refs: Vec::new(),
                },
                context.observed_tick,
            )?,
        ),
    };
    Ok(EbpfServiceObservation {
        samples,
        health,
        status,
        outcome,
        event,
        accounting,
        deny_required_readiness: decision.deny_required_readiness,
    })
}

fn reader_adapter_profile(
    profile: &ObservationProfile,
    source: &EbpfProfile,
    context: &ObservationContext,
) -> ObservationAdapterProfile {
    ObservationAdapterProfile {
        schema: OBSERVATION_ADAPTER_PROFILE_SCHEMA.into(),
        adapter_id: "linux-ebpf-reader".into(),
        adapter_ref: source.binding.adapter.clone(),
        profile_ref: profile.profile_ref.clone(),
        class: ObservationAdapterClass::RuntimeCounter,
        max_queued_bytes: profile.bounds.max_queued_bytes,
        timeout_ticks: source.freshness_ticks,
        drop_on_backpressure: true,
        required: source.required,
        evidence_refs: context.evidence_refs.clone(),
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn reader_adapter_outcome(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    context: &ObservationContext,
    decision: (EbpfServiceDecision, u64),
    issues: Vec<ObservabilityIssue>,
) -> crate::error::Result<Artifact<AdapterOutcome>> {
    let (decision, dropped) = decision;
    let operation_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("linux-ebpf-read-operation-v1", vec![
            crate::preserves_rail::string(&profile.profile_ref),
            crate::preserves_rail::string(&adapter.adapter_ref),
            crate::preserves_rail::string(&context.source_ref),
            crate::preserves_rail::u64_value(context.generation),
            crate::preserves_rail::u64_value(context.observed_tick),
        ]))?;
    adapter_outcome(profile, adapter, &AdapterOutcome {
        operation_ref,
        adapter_ref: adapter.adapter_ref.clone(),
        payload_ref: context.source_ref.clone(),
        kind: decision.kind,
        dropped_observations: dropped,
        service_policy_signal: decision.deny_required_readiness,
        issues,
    })
}

fn invalid(message: &'static str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
