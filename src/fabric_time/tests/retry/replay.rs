use super::support::*;
use super::*;

// r[verify molten.audit_f12.compatibility]
// r[verify molten.audit_f12.validation]
#[test]
fn canonical_fixed_and_saturated_observations_replay_without_history_mutation() {
    let profile = simulation_profile();
    for (backoff, expected_delay) in [
        (RetryBackoff::Fixed, BASE_DELAY),
        (RetryBackoff::Exponential, SATURATED_DELAY),
    ] {
        let mut request = input(&profile);
        request.policy.backoff = backoff;
        let original_request = request.clone();
        let (plan, recorded) = fixture_retry::plan_events(&profile, GENERATION, &request).expect("retry observations");
        assert_eq!(recorded.len(), PLAN_EVENT_COUNT);
        assert_eq!(plan.delay.ticks, expected_delay);
        assert_eq!(plan.deadline.target.ticks(), FIXTURE_RETRY_NOW + expected_delay);
        let decoded = recorded.iter().map(canonical_readback).collect::<Vec<_>>();
        let original = decoded.clone();
        let replayed = fixture_retry::replay(&profile, GENERATION, &request, &decoded).expect("matching replay");
        assert_eq!(replayed, plan);
        assert_eq!(decoded, original);
        assert_eq!(request, original_request);
    }
}

fn canonical_readback(event: &CanonicalTimeEvent) -> CanonicalTimeEvent {
    let bytes = crate::preserves_rail::canonical_bytes(&event.value).expect("canonical event bytes");
    let value = crate::preserves_rail::parse_canonical_bytes(&bytes).expect("canonical event read-back");
    assert_eq!(crate::preserves_rail::content_ref_from_bytes(&bytes), event.evidence_ref);
    let mut truncated = bytes;
    truncated.pop().expect("non-empty event bytes");
    assert!(crate::preserves_rail::parse_canonical_bytes(&truncated).is_err());
    CanonicalTimeEvent { value, ..event.clone() }
}

// r[verify molten.audit_f12.validation]
#[test]
fn historical_wrapped_delay_is_a_visible_divergence_and_never_rewrites_history() {
    let profile = simulation_profile();
    let request = input(&profile);
    let recorded = [("retry-delay", 0), ("retry-planned", FIXTURE_RETRY_NOW)]
        .into_iter()
        .map(|(action, ticks)| observation(&profile, &request, action, ticks))
        .collect::<Vec<_>>();
    assert_divergence_preserves_history(&profile, &request, &recorded);
}

// r[verify molten.audit_f12.validation]
#[test]
fn replay_rejects_changed_deadline_identity_order_and_missing_observations() {
    let profile = simulation_profile();
    let request = input(&profile);
    let (_, recorded) = fixture_retry::plan_events(&profile, GENERATION, &request).expect("retry observations");
    let mut wrong_deadline = recorded.clone();
    wrong_deadline[1] = observation(&profile, &request, "retry-planned", SATURATED_DEADLINE + 1);
    let mut wrong_identity = recorded.clone();
    wrong_identity[0].evidence_ref = HASH_B.to_string();
    let mut wrong_order = recorded.clone();
    wrong_order.swap(0, 1);
    let mut missing = recorded;
    missing.pop().expect("deadline observation");
    for candidate in [wrong_deadline, wrong_identity, wrong_order, missing, Vec::new()] {
        assert_divergence_preserves_history(&profile, &request, &candidate);
    }
}

fn observation(
    profile: &CanonicalTimeProfile,
    request: &fixture_retry::RetryFixtureInput,
    action: &str,
    ticks: u64,
) -> CanonicalTimeEvent {
    canonical_named_event(
        &profile.profile_ref,
        CanonicalTimeEventKind::Deadline,
        request.generation,
        &request.subject_id,
        action,
        ticks,
    )
    .expect("recorded retry observation")
}

fn assert_divergence_preserves_history(
    profile: &CanonicalTimeProfile,
    request: &fixture_retry::RetryFixtureInput,
    recorded: &[CanonicalTimeEvent],
) {
    let original_history = recorded.to_vec();
    let original_request = request.clone();
    let error = fixture_retry::replay(profile, GENERATION, request, recorded).expect_err("retry divergence");
    assert!(error.to_string().contains("retry replay diverged from recorded delay or deadline"));
    let crate::error::MoltenError::HarnessDivergence(divergence) = error else {
        panic!("expected a typed replay divergence");
    };
    assert_eq!(divergence.kind, "fabric-time-retry");
    assert_eq!(divergence.step, Some(ADMITTED_ATTEMPT));
    assert_eq!(recorded, original_history);
    assert_eq!(request, &original_request);
}
