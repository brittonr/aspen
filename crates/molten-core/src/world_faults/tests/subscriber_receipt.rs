use super::*;

fn receipt_with_subscriber(
    profile: &WorldFaultProfile,
    observation: Option<CommittedSubscriberObservation>,
) -> WorldFaultConformanceReceipt {
    build_world_fault_conformance_receipt(
        &standard_world_mutation_inventory(),
        profile,
        &passing_observations(profile),
        passing_schedules(profile),
        observation,
    )
    .expect("both passing and failing receipts are valid evidence")
}

// r[verify molten.world_faults.receipt]
#[test]
fn subscriber_evidence_is_required_for_aggregate_pass_and_revalidated_independently() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("profile");
    let missing = receipt_with_subscriber(&profile, None);
    assert_eq!(missing.subscriber.disposition, ConformanceDisposition::Failed);
    assert_eq!(missing.decision, ConformanceDisposition::Failed);
    assert!(validate_world_fault_receipt(&missing, &profile).is_empty());

    let mut forged = missing;
    forged.subscriber.disposition = ConformanceDisposition::Passed;
    forged.subscriber.diagnostics.clear();
    forged.decision = ConformanceDisposition::Passed;
    let issues = validate_world_fault_receipt(&forged, &profile);
    for field in ["result-disposition", "result-diagnostics", "aggregate-disposition"] {
        assert!(issues.contains(&WorldFaultIssue::SubscriberObservationMismatch(field)));
    }
}

// r[verify molten.world_faults.receipt]
#[test]
fn receipt_revalidation_detects_tampered_ack_even_when_result_claims_pass() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("profile");
    let observation = exercise_committed_subscriber_case(&profile.subscriber).expect("exercise");
    let mut receipt = receipt_with_subscriber(&profile, Some(observation));
    assert_eq!(receipt.decision, ConformanceDisposition::Passed);
    let observed = receipt.subscriber.observation.as_mut().expect("subscriber evidence");
    observed.checkpoint_after_restart.ordinal += 1;
    let issues = validate_world_fault_receipt(&receipt, &profile);
    assert!(issues.contains(&WorldFaultIssue::SubscriberObservationMismatch("result-disposition")));
    assert!(issues.contains(&WorldFaultIssue::SubscriberObservationMismatch("aggregate-disposition")));

    receipt.subscriber = evaluate_committed_subscriber(&profile.subscriber, receipt.subscriber.observation);
    receipt.decision = ConformanceDisposition::Failed;
    assert!(validate_world_fault_receipt(&receipt, &profile).is_empty());
}

// r[verify molten.world_faults.receipt]
#[test]
fn dropped_delivery_produces_failed_receipt_without_becoming_an_encoding_error() {
    let profile = standard_world_fault_profile(TEST_SOURCE_REVISION).expect("profile");
    let mut observation = exercise_committed_subscriber_case(&profile.subscriber).expect("exercise");
    observation.delivered_after_restart.remove(1);
    let mut receipt = receipt_with_subscriber(&profile, Some(observation));
    assert_eq!(receipt.subscriber.disposition, ConformanceDisposition::Failed);
    assert_eq!(receipt.decision, ConformanceDisposition::Failed);
    assert!(validate_world_fault_receipt(&receipt, &profile).is_empty());
    receipt.decision = ConformanceDisposition::Passed;
    assert!(
        validate_world_fault_receipt(&receipt, &profile)
            .contains(&WorldFaultIssue::SubscriberObservationMismatch("aggregate-disposition"))
    );
}
