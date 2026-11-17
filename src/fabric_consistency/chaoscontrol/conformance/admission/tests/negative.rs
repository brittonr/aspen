#[test]
fn selected_fault_cannot_be_promoted_by_a_shaped_effect_reference() {
    let (mut e, mut b) = fixture();
    e.faults.push(super::ExpectedFault {
        action_id: "drop-control".into(),
        class: chaoscontrol_smr::FaultClass::Network,
    });
    b.faults.push(chaoscontrol_smr::FaultOutcome {
        action_id: "drop-control".into(),
        class: chaoscontrol_smr::FaultClass::Network,
        stages: vec![chaoscontrol_smr::FaultStage::Selected],
        effect_record_ref: None,
    });
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Pass, &[]);
    assert_eq!(super::import(&e, &b).unwrap().status, "bounded-instrumentation-valid");
    b.faults[0].effect_record_ref = Some(id("fake-effect"));
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Fault);
    b.faults[0].stages.extend([
        chaoscontrol_smr::FaultStage::Applicable,
        chaoscontrol_smr::FaultStage::Applied,
        chaoscontrol_smr::FaultStage::Observed,
    ]);
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Fault);
    let first = &b.observations[0].envelope;
    let effect = super::FaultEffect {
        action_id: "drop-control".into(),
        class: chaoscontrol_smr::FaultClass::Network,
        participant_ref: first.participant_ref.clone(),
        source_sequence: first.source_sequence,
        boundary_ref: first.boundary_ref.clone(),
        scheduler_ref: first.scheduler_ref.clone(),
        before_ref: id("before"),
        after_ref: id("after"),
    };
    b.faults[0].effect_record_ref = Some(super::artifact_ref("molten-chaoscontrol-fault-effect-v1", &effect).unwrap());
    b.fault_effects.push(effect);
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Pass, &[]);
    assert_eq!(super::import(&e, &b).unwrap().evidence_class, "instrumentation");
    b.fault_effects[0].scheduler_ref = id("foreign-schedule");
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Fault);
}

#[test]
fn decoding_is_bounded_closed_and_discards_parser_payloads() {
    let (e, b) = fixture();
    assert_eq!(super::expected_json(&vec![b' '; usize::try_from(super::MAX_EXPECTED_BYTES).unwrap() + 1]).unwrap_err(), super::ImportIssue::Bounds);
    assert_eq!(super::import_json(&e, &vec![b' '; usize::try_from(super::MAX_BUNDLE_BYTES).unwrap() + 1]).unwrap_err(), super::ImportIssue::Bounds);
    let mut policy = serde_json::to_value(&e).unwrap();
    policy["candidate_override"] = true.into();
    assert_eq!(super::expected_json(&serde_json::to_vec(&policy).unwrap()).unwrap_err(), super::ImportIssue::Decode);
    let mut candidate = serde_json::to_value(&b).unwrap();
    candidate["molten"]["secret-untrusted-payload"] = true.into();
    let issue = super::import_json(&e, &serde_json::to_vec(&candidate).unwrap()).unwrap_err();
    assert_eq!(issue, super::ImportIssue::Decode);
    assert_eq!(
        super::bounds::decode::<serde_json::Value>(format!("\"{}\"", "x".repeat(4_097)).as_bytes(), 8_192).unwrap_err(),
        super::ImportIssue::Decode
    );
    let nested = format!("{}0{}", "[".repeat(34), "]".repeat(34));
    assert_eq!(super::bounds::decode::<serde_json::Value>(nested.as_bytes(), 1_024).unwrap_err(), super::ImportIssue::Decode);
    let collection = format!("[{}0]", "0,".repeat(65_536));
    assert_eq!(
        super::bounds::decode::<serde_json::Value>(collection.as_bytes(), u64::try_from(collection.len()).unwrap()).unwrap_err(),
        super::ImportIssue::Decode
    );
    let mut raw = serde_json::to_vec(&b).unwrap();
    raw.extend_from_slice(b" {}");
    assert_eq!(super::import_json(&e, &raw).unwrap_err(), super::ImportIssue::Decode);
}

#[test]
fn exact_duplicate_chain_facts_do_not_hide_a_second_application() {
    let (e, mut b) = fixture();
    let mut duplicate = b.observations[0].clone();
    duplicate.apply.chain.event_sequence = 2;
    duplicate.apply.application_receipt_ref = id("second-application-receipt");
    duplicate.envelope.source_sequence = 2;
    let inline = serde_json::to_vec(&duplicate.apply.chain).unwrap();
    duplicate.envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(&inline);
    duplicate.envelope.projection = Some(inline);
    b.observations[1].apply.chain.event_sequence = 3;
    b.observations[1].envelope.source_sequence = 3;
    let inline = serde_json::to_vec(&b.observations[1].apply.chain).unwrap();
    b.observations[1].envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(&inline);
    b.observations[1].envelope.projection = Some(inline);
    b.observations.insert(1, duplicate);
    b.drains[0].last_sequence = 3;
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Fail, &[
        "committed-index-rollback-or-duplicate",
        "operation-applied-more-than-once",
    ]);
    assert_eq!(b.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert_eq!(b.history.exact_duplicates, 1);
    assert_eq!(super::import(&e, &b).unwrap().status, "external-failure");
}

#[test]
fn exhausted_liveness_horizon_is_not_rewritten_as_success() {
    let (e, mut b) = fixture();
    b.stabilization.as_mut().unwrap().committed_before = 1;
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Fail, &["bounded-liveness-failure"]);
    assert_eq!(b.liveness.verdict, chaoscontrol_smr::LivenessVerdict::Fail);
    assert_eq!(super::import(&e, &b).unwrap().status, "external-failure");
    b.liveness.verdict = chaoscontrol_smr::LivenessVerdict::Pass;
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Liveness);
}

#[test]
fn expected_policy_requires_canonical_operations_commands_and_distinct_sources() {
    let (e, _) = fixture();
    for mutate in [
        |p: &mut super::ExpectedCohort| p.operations[0].operation_ref = id("arbitrary-shaped-operation"),
        |p: &mut super::ExpectedCohort| p.operations[0].command_ref = id("arbitrary-shaped-command"),
        |p: &mut super::ExpectedCohort| {
            p.sources.pop();
        },
        |p: &mut super::ExpectedCohort| p.protocol.participants[1].guest_id = p.protocol.participants[0].guest_id,
        |p: &mut super::ExpectedCohort| p.workload.observation_mode = chaoscontrol_smr::ObservationMode::Sampled,
        |p: &mut super::ExpectedCohort| p.protocol.bounds.oracle_work = super::MAX_OBSERVATIONS + 1,
    ] {
        let mut policy = e.clone();
        mutate(&mut policy);
        assert!(super::expected_json(&serde_json::to_vec(&policy).unwrap()).is_err());
    }
}

#[test]
fn packed_command_preflight_bounds_depth_and_tokens_before_tree_decode() {
    let nested = |depth: usize| {
        let mut bytes = vec![u8::from(preserves::packed::constants::Tag::Sequence); depth];
        bytes.push(u8::from(preserves::packed::constants::Tag::True));
        bytes.extend(std::iter::repeat_n(u8::from(preserves::packed::constants::Tag::End), depth));
        bytes
    };
    let exact = nested(32);
    assert!(super::bounds::command(&exact, &crate::preserves_rail::content_ref_from_bytes(&exact)).is_ok());
    let deep = nested(33);
    assert_eq!(
        super::bounds::command(&deep, &crate::preserves_rail::content_ref_from_bytes(&deep)).unwrap_err(),
        super::ImportIssue::Bounds
    );
    let wide = |count: usize| {
        let mut bytes = vec![u8::from(preserves::packed::constants::Tag::Sequence)];
        bytes.extend(std::iter::repeat_n(u8::from(preserves::packed::constants::Tag::True), count));
        bytes.push(u8::from(preserves::packed::constants::Tag::End));
        bytes
    };
    let exact = wide(8_190);
    assert!(super::bounds::command(&exact, &crate::preserves_rail::content_ref_from_bytes(&exact)).is_ok());
    let excess = wide(8_191);
    assert_eq!(
        super::bounds::command(&excess, &crate::preserves_rail::content_ref_from_bytes(&excess)).unwrap_err(),
        super::ImportIssue::Bounds
    );
}