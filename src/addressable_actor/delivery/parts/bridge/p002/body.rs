fn exact_ack_replay(state: &DeliveryState, request: &DeliveryRequest, token: &DeliveryToken) -> bool {
    state
        .completed
        .get(&token.item_ref)
        .is_some_and(|completed| completed.delivery_id == token.delivery_id)
        && state.operations.get(&request.operation_id).is_some_and(|operation| {
            operation.operation_kind == "acknowledge"
                && operation.request_ref == identify_delivery_request(request)
                && operation.token_ref.as_deref() == Some(token.token_ref.as_str())
        })
}

#[derive(Default)]
struct ObservedStatus {
    observed_ref: Option<String>,
}

impl crate::coordination_delivery::DeliveryStatusPort for ObservedStatus {
    fn publish_status(&mut self, status: &DeliveryStatus) -> crate::coordination_delivery::PortResult<crate::coordination_delivery::StatusObservation> {
        self.observed_ref = Some(
            crate::coordination_delivery::identify_canonical_status(status)
                .map_err(|error| crate::coordination_delivery::PortError::new("actor-delivery-status", error.to_string(), false))?,
        );
        Ok(crate::coordination_delivery::StatusObservation {
            published_status_ref: None,
            outcome_unknown: false,
        })
    }
}

fn problem(error: impl std::fmt::Debug) -> crate::error::Failure {
    invalid(format!("actor coordination delivery: {error:?}"))
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn admit_time_profile(profile: &crate::fabric_time::AdmittedTimeProfile) -> crate::error::Result<String> {
    let admitted = crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: profile.profile_id.clone(),
        profile_ref: profile.profile_ref.clone(),
        kind: profile.kind,
        supported_domains: profile.supported_domains.clone(),
        max_duration_ticks: profile.max_duration_ticks,
        max_uncertainty_ticks: profile.max_uncertainty_ticks,
        max_timers: profile.max_timers,
        max_runnables: profile.max_runnables,
        max_entropy_request_bytes: profile.max_entropy_request_bytes,
        max_entropy_total_bytes: profile.max_entropy_total_bytes,
        max_scheduler_concurrency: profile.max_scheduler_concurrency,
        max_scheduler_queue_depth: profile.max_scheduler_queue_depth,
        fairness_bound_turns: profile.fairness_bound_turns,
        scheduler_policy: profile.scheduler_policy,
        evidence_mode: profile.evidence_mode,
        non_claims: profile.non_claims.clone(),
    })?;
    Ok(admitted.profile_ref)
}

fn pending_items(state: &DeliveryState) -> crate::error::Result<u32> {
    let pending = state
        .ready
        .len()
        .checked_add(state.in_flight.len())
        .ok_or_else(|| invalid("actor pending mailbox count overflow"))?;
    u32::try_from(pending).map_err(problem)
}
