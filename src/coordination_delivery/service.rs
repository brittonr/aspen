use molten_core::coordination_delivery::*;
use molten_core::fabric_time::AdmittedTimeProfile;

use super::*;

#[derive(Clone, Debug)]
pub struct DeliveryServiceRequest<'a> {
    pub manifest: &'a DeliveryManifest,
    pub policy: &'a DeliveryPolicy,
    pub time_profile: &'a AdmittedTimeProfile,
    pub host_binding: &'a DeliveryHostBindingFacts,
    pub expected: ExpectedDeliveryState,
    pub request: &'a DeliveryRequest,
}

#[derive(Clone, Debug)]
pub struct DeliveryServiceOutcome {
    pub transition: DeliveryTransition,
    pub receipt: CanonicalDeliveryCommitReceipt,
    pub commit_observation: Option<DeliveryCommitObservation>,
    pub timer_observation: DeliveryTimerObservation,
    pub status_observation: DeliveryStatusObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryServiceError {
    Port(DeliveryPortError),
    Receipt(String),
    Host(DeliveryIssue),
    Status(DeliveryIssue),
}

pub type DeliveryServiceResult<T> = std::result::Result<T, DeliveryServiceError>;

// r[impl molten.coordination_delivery.consistency_durability]
// r[impl molten.coordination_delivery.fenced_completion]
// r[impl molten.coordination_delivery.logical_time]
pub fn apply_delivery_request(
    commit_port: &mut impl DeliveryCommitPort,
    timer_port: &mut impl DeliveryTimerPort,
    status_port: &mut impl DeliveryStatusPort,
    service: &DeliveryServiceRequest<'_>,
) -> DeliveryServiceResult<DeliveryServiceOutcome> {
    validate_delivery_host_binding(service.host_binding, service.manifest).map_err(DeliveryServiceError::Host)?;
    let observed = commit_port.load(&service.request.queue_id).map_err(DeliveryServiceError::Port)?;
    let state = observed.as_ref().map_or_else(
        || {
            DeliveryState::empty(
                &service.request.queue_id,
                service.manifest.policy_ref.clone(),
                service.request.service_generation,
                service.request.consistency_epoch,
            )
        },
        |published| published.state.clone(),
    );
    if !expected_matches(&service.expected, observed.as_ref()) {
        return stale_outcome(&state, service);
    }
    let transition = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: service.manifest,
        policy: service.policy,
        time_profile: service.time_profile,
        state: &state,
        request: service.request,
    });
    if transition.decision != DeliveryDecisionKind::Applied {
        return no_commit_outcome(transition, service);
    }

    let next = PublishedDeliveryState::from_state(transition.next_state.clone());
    let commit_request = DeliveryCommitRequest {
        queue_id: service.request.queue_id.clone(),
        expected: service.expected.clone(),
        next,
        requested_engine_epoch: service.request.engine_epoch,
    };
    let (status, commit_observation) = match commit_port.compare_and_commit(&commit_request) {
        Ok(observation) if observation.disposition == DeliveryCommitDisposition::Unknown => {
            reconcile_unknown_commit(commit_port, &commit_request, Some(observation))?
        }
        Ok(observation) => (status_from_observation(&observation, service.request), observation),
        Err(error) if error.outcome_unknown => reconcile_unknown_commit(commit_port, &commit_request, None)?,
        Err(error) => return Err(DeliveryServiceError::Port(error)),
    };
    let mut timer_observation = DeliveryTimerObservation::empty();
    let mut status_observation = DeliveryStatusObservation {
        published_status_ref: None,
        outcome_unknown: false,
    };
    if status.commit_confirmed() {
        timer_observation = observe_timers(timer_port, &transition.timer_intents);
        let projected = project_delivery_status(&transition.next_state, &StatusProjectionInput {
            policy: service.policy,
            requested_limit: service.policy.status_item_limit,
            resource_refs: &service.request.resource_refs,
            evidence_refs: &service.request.evidence_refs,
        })
        .map_err(DeliveryServiceError::Status)?;
        status_observation = observe_status(status_port, &projected);
    }
    let receipt = build_receipt(&transition, status, &commit_observation, &timer_observation, &status_observation)?;
    Ok(DeliveryServiceOutcome {
        transition,
        receipt,
        commit_observation: Some(commit_observation),
        timer_observation,
        status_observation,
    })
}

fn stale_outcome(
    state: &DeliveryState,
    service: &DeliveryServiceRequest<'_>,
) -> DeliveryServiceResult<DeliveryServiceOutcome> {
    let state_ref = identify_delivery_state(state);
    let transition = DeliveryTransition {
        schema: DELIVERY_TRANSITION_SCHEMA.to_string(),
        decision: DeliveryDecisionKind::Denied,
        kind: DeliveryTransitionKind::DeniedPreserve,
        request_ref: identify_delivery_request(service.request),
        operation_ref: identify_applied_operation(
            &identify_delivery_request(service.request),
            "stale-expected-state",
            service.request.operation.item_ref(),
            None,
        ),
        before_state_ref: state_ref.clone(),
        after_state_ref: state_ref,
        next_state: state.clone(),
        token: None,
        timer_intents: Vec::new(),
        issue: Some(DeliveryIssue::CurrentnessRequired),
        prior_operation_ref: None,
        worker_dispatch_authorized: false,
        external_effect_exactly_once: false,
    };
    no_commit_with_status(transition, service, DeliveryServiceStatus::Stale)
}

fn no_commit_outcome(
    transition: DeliveryTransition,
    service: &DeliveryServiceRequest<'_>,
) -> DeliveryServiceResult<DeliveryServiceOutcome> {
    let status = match transition.decision {
        DeliveryDecisionKind::DuplicateReplay => DeliveryServiceStatus::DuplicateReplay,
        DeliveryDecisionKind::Denied => DeliveryServiceStatus::Denied,
        DeliveryDecisionKind::Applied => DeliveryServiceStatus::Unknown,
    };
    no_commit_with_status(transition, service, status)
}

fn no_commit_with_status(
    transition: DeliveryTransition,
    service: &DeliveryServiceRequest<'_>,
    status: DeliveryServiceStatus,
) -> DeliveryServiceResult<DeliveryServiceOutcome> {
    let observation = DeliveryCommitObservation {
        disposition: DeliveryCommitDisposition::Stale,
        currentness: service.request.currentness,
        durability: DeliveryDurabilityOutcome::Unknown,
        engine_epoch: service.request.engine_epoch,
        observed_state_ref: Some(transition.before_state_ref.clone()),
    };
    let timer_observation = DeliveryTimerObservation::empty();
    let status_observation = DeliveryStatusObservation {
        published_status_ref: None,
        outcome_unknown: false,
    };
    let receipt = build_receipt(&transition, status, &observation, &timer_observation, &status_observation)?;
    Ok(DeliveryServiceOutcome {
        transition,
        receipt,
        commit_observation: None,
        timer_observation,
        status_observation,
    })
}

fn reconcile_unknown_commit(
    commit_port: &impl DeliveryCommitPort,
    request: &DeliveryCommitRequest,
    original: Option<DeliveryCommitObservation>,
) -> DeliveryServiceResult<(DeliveryServiceStatus, DeliveryCommitObservation)> {
    let observed = commit_port.load(&request.queue_id).map_err(DeliveryServiceError::Port)?;
    let (status, state_ref) = if observed.as_ref() == Some(&request.next) {
        (DeliveryServiceStatus::AppliedAfterReconciliation, Some(request.next.state_ref.clone()))
    } else if expected_matches(&request.expected, observed.as_ref()) {
        (DeliveryServiceStatus::NotAppliedAfterReconciliation, observed.map(|published| published.state_ref))
    } else {
        (DeliveryServiceStatus::Unknown, observed.map(|published| published.state_ref))
    };
    let observation = original.unwrap_or(DeliveryCommitObservation {
        disposition: DeliveryCommitDisposition::Unknown,
        currentness: DeliveryCurrentness::Unknown,
        durability: DeliveryDurabilityOutcome::Unknown,
        engine_epoch: request.requested_engine_epoch,
        observed_state_ref: state_ref,
    });
    Ok((status, observation))
}

fn status_from_observation(
    observation: &DeliveryCommitObservation,
    request: &DeliveryRequest,
) -> DeliveryServiceStatus {
    if observation.engine_epoch != request.engine_epoch
        || !observation.currentness.is_current()
        || observation.durability != DeliveryDurabilityOutcome::Durable
    {
        return DeliveryServiceStatus::Unknown;
    }
    match observation.disposition {
        DeliveryCommitDisposition::Applied => DeliveryServiceStatus::Applied,
        DeliveryCommitDisposition::AlreadyApplied => DeliveryServiceStatus::AlreadyApplied,
        DeliveryCommitDisposition::Stale => DeliveryServiceStatus::Stale,
        DeliveryCommitDisposition::Unknown => DeliveryServiceStatus::Unknown,
    }
}

fn observe_timers(
    timer_port: &mut impl DeliveryTimerPort,
    intents: &[DeliveryTimerIntent],
) -> DeliveryTimerObservation {
    if intents.is_empty() {
        return DeliveryTimerObservation::empty();
    }
    match timer_port.apply_timer_intents(intents) {
        Ok(observation) => observation,
        Err(error) => DeliveryTimerObservation {
            accepted_timer_refs: Vec::new(),
            failed_timer_refs: intents.iter().map(|intent| intent.timer_id.clone()).collect(),
            outcome_unknown: error.outcome_unknown,
        },
    }
}

fn observe_status(status_port: &mut impl DeliveryStatusPort, status: &DeliveryStatus) -> DeliveryStatusObservation {
    match status_port.publish_status(status) {
        Ok(observation) => observation,
        Err(error) => DeliveryStatusObservation {
            published_status_ref: None,
            outcome_unknown: error.outcome_unknown,
        },
    }
}

fn build_receipt(
    transition: &DeliveryTransition,
    status: DeliveryServiceStatus,
    commit: &DeliveryCommitObservation,
    timers: &DeliveryTimerObservation,
    status_observation: &DeliveryStatusObservation,
) -> DeliveryServiceResult<CanonicalDeliveryCommitReceipt> {
    canonical_delivery_commit_receipt(&DeliveryCommitReceipt {
        queue_id: transition.next_state.queue_id.clone(),
        request_ref: transition.request_ref.clone(),
        operation_ref: transition.operation_ref.clone(),
        before_state_ref: transition.before_state_ref.clone(),
        after_state_ref: transition.after_state_ref.clone(),
        revision: transition.next_state.revision,
        status,
        currentness: commit.currentness,
        durability: commit.durability,
        engine_epoch: commit.engine_epoch,
        timer_refs: timers.accepted_timer_refs.clone(),
        failed_timer_refs: timers.failed_timer_refs.clone(),
        status_ref: status_observation.published_status_ref.clone(),
        issue: transition.issue.clone(),
        authorizes_future_mutation: false,
        authorizes_worker_effects: false,
        claims_exactly_once: false,
        non_claims: required_delivery_non_claims(),
    })
    .map_err(|error| DeliveryServiceError::Receipt(error.to_string()))
}

fn expected_matches(expected: &ExpectedDeliveryState, observed: Option<&PublishedDeliveryState>) -> bool {
    match (&expected.state_ref, observed) {
        (None, None) => expected.revision == INITIAL_DELIVERY_REVISION,
        (Some(expected_ref), Some(observed)) => {
            expected.revision == observed.revision && expected_ref == &observed.state_ref
        }
        _ => false,
    }
}
