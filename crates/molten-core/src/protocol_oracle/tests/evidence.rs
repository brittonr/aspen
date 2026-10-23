use super::*;

fn stabilization(participant: &str) -> ParticipantStabilization {
    ParticipantStabilization {
        participant: participant.to_owned(),
        is_ready: true,
        has_required_membership: true,
        is_disruptive_fault_active: false,
        is_durability_satisfied: true,
        start_progress: 2,
        end_progress: Some(5),
        observed_ticks: 8,
    }
}

// r[verify molten.fabric_simulation.participant_liveness]
#[test]
fn liveness_is_participant_scoped_and_conditional_on_explicit_facts() {
    let preconditions = profile().liveness;
    let mut stalled = stabilization("node-b");
    stalled.end_progress = Some(2);
    let results = evaluate_cohort_liveness(&preconditions, &[stabilization("node-a"), stalled]);
    assert_eq!(results[0].verdict, LivenessVerdict::Pass);
    assert_eq!(results[1].verdict, LivenessVerdict::Fail, "cohort progress cannot hide a stalled participant");

    let mut partitioned = stabilization("node-a");
    partitioned.has_required_membership = false;
    assert_eq!(
        evaluate_participant_liveness(&preconditions, &partitioned).verdict,
        LivenessVerdict::NotEvaluated(LivenessCondition::MembershipUnavailable)
    );
    let mut faulted = stabilization("node-a");
    faulted.is_disruptive_fault_active = true;
    faulted.end_progress = Some(2);
    assert_eq!(
        evaluate_participant_liveness(&preconditions, &faulted).verdict,
        LivenessVerdict::NotEvaluated(LivenessCondition::DisruptiveFaultActive),
        "a missing precondition is neither pass nor fail"
    );
    let mut lost = stabilization("node-a");
    lost.end_progress = None;
    assert_eq!(
        evaluate_participant_liveness(&preconditions, &lost).verdict,
        LivenessVerdict::Incomplete(LivenessCondition::EndNotObserved)
    );
    let mut short = stabilization("node-a");
    short.observed_ticks = 7;
    assert_eq!(
        evaluate_participant_liveness(&preconditions, &short).verdict,
        LivenessVerdict::Incomplete(LivenessCondition::HorizonNotCovered)
    );
}

fn evidence(safety: Vec<SafetyResult>) -> ProtocolOracleEvidence {
    let profile = profile();
    ProtocolOracleEvidence {
        run: ProtocolRunIdentity {
            world_ref: content("world"),
            runtime_ref: content("runtime"),
            scheduler_ref: content("scheduler"),
            workload_ref: content("workload"),
            fault_ref: content("faults"),
            replay_ref: content("replay"),
        },
        profile_id: profile.profile_id.clone(),
        projection_schema: PROJECTION_SCHEMA.to_owned(),
        oracle_ref: profile.oracle_ref.clone(),
        participants: vec!["node-a".to_owned(), "node-b".to_owned()],
        projection_refs: vec![projection_ref(&projection("node-a", 0, 0, &["a"]))],
        gaps: Vec::new(),
        safety,
        liveness: evaluate_cohort_liveness(&profile.liveness, &[stabilization("node-a")]),
        novelty_refs: vec![novelty_identity(
            &profile.novelty_fields,
            &projection("node-a", 0, 0, &["a"]),
        )],
        counters: [("transitions".to_owned(), 3)].into_iter().collect(),
        non_claims: profile.non_claims,
    }
}

fn safety(verdict: OracleVerdict) -> Vec<SafetyResult> {
    vec![SafetyResult {
        level: SafetyLevel::Pairwise,
        service: SERVICE.to_owned(),
        rule: "pairwise-agreement".to_owned(),
        verdict,
    }]
}

// r[verify molten.fabric_simulation.protocol_oracle_evidence]
#[test]
fn a_complete_passing_run_admits_only_a_simulation_claim() {
    let passing = evidence(safety(OracleVerdict::Pass));
    assert_eq!(evidence_verdict(&passing), EvidenceVerdict::Pass);
    assert_eq!(admit_protocol_claim(&passing, ProtocolClaim::DeterministicSimulation), Ok(()));
    for claim in [
        ProtocolClaim::Kvm,
        ProtocolClaim::Live,
        ProtocolClaim::Production,
        ProtocolClaim::Release,
        ProtocolClaim::HardwarePerformance,
    ] {
        assert_eq!(
            admit_protocol_claim(&passing, claim),
            Err(ProtocolClaimIssue::RequiresProfileEvidence(claim)),
            "simulation evidence is never promoted"
        );
    }
    assert_eq!(evidence_ref(&passing), evidence_ref(&passing.clone()));
    assert!(is_content_ref(&evidence_ref(&passing)));
}

// r[verify molten.fabric_simulation.protocol_oracle_evidence]
#[test]
fn incomplete_failed_unbound_or_overclaimed_evidence_never_passes() {
    let incomplete = evidence(safety(OracleVerdict::Incomplete));
    assert_eq!(evidence_verdict(&incomplete), EvidenceVerdict::Incomplete);
    assert_eq!(
        admit_protocol_claim(&incomplete, ProtocolClaim::DeterministicSimulation),
        Err(ProtocolClaimIssue::NotPassing(EvidenceVerdict::Incomplete))
    );
    let mut gapped = evidence(safety(OracleVerdict::Pass));
    gapped.gaps.push(CompletenessGap::MissingParticipant("node-b".to_owned()));
    assert_eq!(evidence_verdict(&gapped), EvidenceVerdict::Incomplete);

    let witness = FailureWitness {
        run_sequence: 1,
        participant: "node-b".to_owned(),
        scheduler_choice_ref: content("choice"),
        projection_ref: content("projection"),
        other_projection_ref: None,
    };
    assert_eq!(evidence_verdict(&evidence(safety(OracleVerdict::Fail(witness)))), EvidenceVerdict::Fail);

    let mut unbound = evidence(safety(OracleVerdict::Pass));
    unbound.run.replay_ref = "replay".to_owned();
    assert_eq!(
        admit_protocol_claim(&unbound, ProtocolClaim::DeterministicSimulation),
        Err(ProtocolClaimIssue::MalformedRef("replay"))
    );
    let mut silent = evidence(safety(OracleVerdict::Pass));
    silent.non_claims.pop();
    assert!(matches!(
        admit_protocol_claim(&silent, ProtocolClaim::DeterministicSimulation),
        Err(ProtocolClaimIssue::MissingNonClaim(_))
    ));
}
