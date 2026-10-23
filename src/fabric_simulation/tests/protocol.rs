const GENERATED_PROTOCOL_ORACLE_PROFILE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/protocol-oracle/generated/profile.json"));

fn canonical_iovalue(value: &molten_core::protocol_oracle::CanonicalValue) -> preserves::IOValue {
    match value {
        molten_core::protocol_oracle::CanonicalValue::Integer(integer) => crate::preserves_rail::u64_value(u64::try_from(*integer).expect("unsigned")),
        molten_core::protocol_oracle::CanonicalValue::String(text) => crate::preserves_rail::string(text),
        molten_core::protocol_oracle::CanonicalValue::Symbol(name) => preserves::IOValue::symbol(name.clone()),
        molten_core::protocol_oracle::CanonicalValue::Bytes(_) => panic!("projections carry no byte strings"),
        molten_core::protocol_oracle::CanonicalValue::Record { label, fields } => preserves::IOValue::record(
            preserves::IOValue::symbol(label.clone()),
            fields.iter().map(canonical_iovalue).collect(),
        ),
        molten_core::protocol_oracle::CanonicalValue::Sequence(items) => crate::preserves_rail::sequence(items.iter().map(canonical_iovalue).collect()),
    }
}

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn the_core_projection_encoding_is_the_preserves_canonical_encoding() {
    let state = reference_state_value(&initial_reference_state(crate::fabric::ReferenceSystemKind::ReplicatedLog));
    let projection = molten_core::protocol_oracle::ProtocolProjection {
        protocol: REFERENCE_PROTOCOL.to_string(),
        service: "replicated-log".to_string(),
        participant: "node-replicated-log".to_string(),
        generation: 1,
        source_sequence: 3,
        run_sequence: 300,
        scheduler_choice_ref: blake3_ref(b"choice"),
        transition_class: molten_core::protocol_oracle::TransitionClass::Applied,
        logical_position: 0,
        state_ref: molten_core::protocol_oracle::state_ref_of(&state),
        state,
        physical_ref: Some(blake3_ref(b"layout")),
        history_ref: blake3_ref(b"history"),
        durability: molten_core::protocol_oracle::DurabilityFact::Durable,
        progress: 2,
        completeness: molten_core::protocol_oracle::ObservationCompleteness::Complete,
        reported_invariants: vec!["log-offsets-contiguous".to_string()],
    };
    let value = molten_core::protocol_oracle::projection_value(&projection);
    let preserves_bytes = crate::preserves_rail::canonical_bytes(&canonical_iovalue(&value)).expect("canonical bytes");
    assert_eq!(molten_core::protocol_oracle::encode_canonical(&value), preserves_bytes);
}

// r[verify molten.fabric_simulation.oracle_independence]
// r[verify molten.fabric_simulation.protocol_oracle_evidence]
#[test]
fn the_reference_run_binds_independent_oracle_evidence_into_its_canonical_record() {
    let fixture = run_reference_fixture().expect("reference fixture");
    let protocol = &fixture.protocol;
    assert_eq!(fixture.run.summary.decision, SimulationDecision::Pass);
    assert_ne!(protocol.status.verdict, "fail");
    assert_eq!(protocol.evidence_ref, molten_core::protocol_oracle::evidence_ref(&protocol.evidence));
    assert!(protocol.evidence.gaps.is_empty(), "every participant is observed losslessly");
    for observation in &fixture.observations {
        assert!(protocol.evidence.projection_refs.contains(&observation.observation.projection_ref));
    }
    let local = protocol
        .evidence
        .safety
        .iter()
        .filter(|result| result.level == molten_core::protocol_oracle::SafetyLevel::Local)
        .collect::<Vec<_>>();
    assert_eq!(local.len(), 6, "two local rules for each of three services");
    assert!(local.iter().all(|result| result.verdict == molten_core::protocol_oracle::OracleVerdict::Pass));
    assert!(!protocol.evidence.novelty_refs.is_empty());
    assert_eq!(protocol.evidence.counters.len(), 4);

    let readback = parse_run_readback(&fixture.run.value).expect("run readback");
    assert_eq!(readback.protocol, protocol.status);
    assert_eq!(
        molten_core::protocol_oracle::admit_protocol_claim(
            &protocol.evidence,
            molten_core::protocol_oracle::ProtocolClaim::Live
        ),
        Err(molten_core::protocol_oracle::ProtocolClaimIssue::RequiresProfileEvidence(
            molten_core::protocol_oracle::ProtocolClaim::Live
        ))
    );
}

// r[verify molten.fabric_simulation.protocol_projection]
// r[verify molten.fabric_simulation.protocol_novelty]
#[test]
fn repeated_reference_runs_repeat_their_projection_novelty_and_evidence_identities() {
    let first = run_reference_fixture().expect("first run");
    let second = run_reference_fixture().expect("second run");
    assert_eq!(first.protocol.evidence.projection_refs, second.protocol.evidence.projection_refs);
    assert_eq!(first.protocol.evidence.novelty_refs, second.protocol.evidence.novelty_refs);
    assert_eq!(first.protocol.evidence_ref, second.protocol.evidence_ref);
    assert_eq!(first.run.run_ref, second.run.run_ref);
    for state in first.service_states.values() {
        assert!(first.run.summary.final_state_refs.contains(&reference_state_ref(state)));
    }
}

// r[verify molten.fabric_simulation.protocol_safety]
#[test]
fn a_crash_recovery_is_checked_against_the_states_each_participant_held() {
    for is_delayed in [false, true] {
        let manifest = causal_acknowledgment_manifest(is_delayed).expect("causal manifest");
        let fixture = run_reference_world(&manifest, DEFAULT_REFERENCE_SEED).expect("causal run");
        let durability = fixture
            .protocol
            .evidence
            .safety
            .iter()
            .find(|result| result.level == molten_core::protocol_oracle::SafetyLevel::Durability)
            .expect("durability result");
        assert_eq!(durability.verdict, molten_core::protocol_oracle::OracleVerdict::Pass, "delayed={is_delayed}");
        assert!(
            fixture.protocol.evidence.liveness.iter().all(|result| matches!(
                result.verdict,
                molten_core::protocol_oracle::LivenessVerdict::NotEvaluated(
                    molten_core::protocol_oracle::LivenessCondition::DisruptiveFaultActive
                )
            )),
            "liveness is not evaluated across a disruptive crash"
        );
    }
}

// r[verify molten.fabric_simulation.protocol_oracle_validation]
#[test]
fn the_generated_profile_is_admitted_and_a_drifted_profile_is_refused() {
    let profile = reference_protocol_oracle_profile().expect("reviewed profile");
    assert_eq!(profile.participants.len(), 3);
    let mut document =
        serde_json::from_str::<serde_json::Value>(GENERATED_PROTOCOL_ORACLE_PROFILE).expect("profile JSON");
    document["oracle_ref"] = serde_json::Value::String(blake3_ref(b"another oracle"));
    assert!(parse_protocol_oracle_profile(&document.to_string()).is_err());
    let mut unknown = serde_json::from_str::<serde_json::Value>(GENERATED_PROTOCOL_ORACLE_PROFILE).expect("profile JSON");
    unknown["debug_payload"] = serde_json::Value::Bool(true);
    assert!(parse_protocol_oracle_profile(&unknown.to_string()).is_err());
}

// r[verify molten.fabric_simulation.protocol_novelty]
#[test]
fn novelty_guidance_scores_only_protocol_state_no_earlier_seed_reached() {
    let manifest = reference_world_manifest().expect("reference manifest");
    let scores = explore_reference_seeds(&manifest, &[DEFAULT_REFERENCE_SEED, DEFAULT_REFERENCE_SEED]).expect("explore");
    assert!(scores[0].score.new > 0);
    assert_eq!(scores[1].score.new, 0, "a repeated schedule reaches nothing new");
    assert_eq!(scores[1].score.repeated, scores[0].score.new);
    assert_eq!(scores[0].run_ref, scores[1].run_ref);
}

// r[verify molten.fabric_simulation.protocol_cost]
#[test]
fn work_counters_compare_exactly_within_one_cohort_and_refuse_another_schedule() {
    let profile = reference_protocol_oracle_profile().expect("profile");
    let first = run_reference_fixture().expect("first run");
    let second = run_reference_fixture().expect("second run");
    let deltas = molten_core::protocol_oracle::compare_counters(
        (&first.protocol.counter_cohort(&profile), &first.protocol.evidence.counters),
        (&second.protocol.counter_cohort(&profile), &second.protocol.evidence.counters),
    )
    .expect("one cohort");
    assert!(deltas.iter().all(|delta| delta.delta == 0));
    let mut other = second.protocol.counter_cohort(&profile);
    other.scheduler_ref = blake3_ref(b"another schedule");
    assert_eq!(
        molten_core::protocol_oracle::compare_counters(
            (&first.protocol.counter_cohort(&profile), &first.protocol.evidence.counters),
            (&other, &second.protocol.evidence.counters),
        ),
        Err(molten_core::protocol_oracle::CounterComparisonIssue::CohortMismatch("scheduler"))
    );
}

// r[verify molten.fabric_simulation.protocol_oracle_evidence]
#[test]
fn the_published_consumer_contract_matches_the_projection_encoding() {
    use molten_core::protocol_oracle::*;
    let contract = serde_json::from_str::<serde_json::Value>(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/config/protocol-oracle/generated/consumer-contract.json"
    )))
    .expect("consumer contract JSON");
    let names = contract["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| field["name"].as_str().expect("name"))
        .collect::<Vec<_>>();
    assert_eq!(names, PROJECTION_FIELDS);
    assert_eq!(contract["record_label"], PROJECTION_LABEL);
    assert_eq!(contract["projection_schema"], PROJECTION_SCHEMA);
    for (key, domain) in [
        ("projection_domain", PROJECTION_DOMAIN),
        ("state_domain", STATE_DOMAIN),
        ("novelty_domain", NOVELTY_DOMAIN),
        ("evidence_domain", EVIDENCE_DOMAIN),
    ] {
        assert_eq!(contract["identity"][key], domain);
    }
    let reported = contract["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .find(|field| field["name"] == "reported-invariants")
        .expect("reported invariants");
    assert_eq!(reported["is_evidential"], false, "a self-report is never evidence");
}
