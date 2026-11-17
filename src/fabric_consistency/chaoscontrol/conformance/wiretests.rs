//! Offline instrumentation controls only: these supplied committed-apply facts
//! and source identities do not claim a VM, transport, or application execution.

#[test]
fn source_sequence_is_not_the_committed_command_index() {
    let (protocol, plan) = super::fixtures::profiles();
    let mut source = super::fixtures::context(1);
    source.source_sequence = 19;
    let encoded = super::project(&protocol, &plan, &source, &super::fixtures::observed(&protocol, &plan))
        .expect("independent sequence");
    super::validate(&protocol, &plan, &source.group_ref, &encoded).expect("source-bound transition");
    let report = chaoscontrol_smr::validate_history(&plan, std::slice::from_ref(&encoded.apply.chain))
        .expect("producer history");
    assert_eq!(report.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    let mut changed = encoded;
    changed.envelope.source_sequence = 1;
    assert_eq!(super::validate(&protocol, &plan, &source.group_ref, &changed), Err(super::Issue::SourceSequence));
    source.source_sequence = 0;
    assert_eq!(
        super::project(&protocol, &plan, &source, &super::fixtures::observed(&protocol, &plan)),
        Err(super::Issue::SourceSequence)
    );
}

#[test]
fn source_identity_cannot_be_relabelled_under_an_admitted_projection() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    let group = super::fixtures::id("group");
    let mut changed = original.clone();
    changed.envelope.participant_ref = protocol.participants[1].participant_ref.clone();
    changed.envelope.producer_ref = protocol.participants[1].producer_ref.clone();
    changed.envelope.guest_id = protocol.participants[1].guest_id;
    assert_eq!(super::validate(&protocol, &plan, &group, &changed), Err(super::Issue::Participant));
    let mut changed = original.clone();
    changed.envelope.generation += 1;
    changed.apply.lifecycle_generation += 1;
    assert_eq!(super::validate(&protocol, &plan, &group, &changed), Err(super::Issue::Envelope));
    let mut changed = original.clone();
    changed.envelope.producer_ref = protocol.participants[1].producer_ref.clone();
    assert_eq!(super::validate(&protocol, &plan, &group, &changed), Err(super::Issue::Envelope));
    let mut changed = original;
    changed.envelope.guest_id += 1;
    assert_eq!(super::validate(&protocol, &plan, &group, &changed), Err(super::Issue::Envelope));
}

#[test]
fn valid_projection_hash_does_not_replace_actual_command_transition_validation() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    let mut changed = original.clone();
    changed.apply.command_bytes = b"swapped-command".to_vec();
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &changed),
        Err(super::Issue::Transition)
    );
    assert_eq!(super::binding_ref(&changed.apply), Err(super::Issue::Transition));
    let mut changed = original;
    changed.apply.chain.digest = super::fixtures::id("forged-next-digest");
    // Even coherent inline/sidecar fields and the correct producer projection
    // framing cannot turn a fabricated chain transition into a committed one.
    changed.envelope.projection = Some(serde_json::to_vec(&changed.apply.chain).expect("encode forged chain"));
    changed.envelope.projection_ref =
        chaoscontrol_smr::observation::projection_ref(changed.envelope.projection.as_ref().expect("inline"));
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &changed),
        Err(super::Issue::Transition)
    );
    let mut committed = super::fixtures::observed(&protocol, &plan);
    committed.command_bytes = b"swapped-command".to_vec();
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &committed),
        Err(super::Issue::Transition)
    );
}

#[test]
fn legacy_hash_only_and_raw_hash_events_are_rejected_without_repair() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    let mut missing = original.clone();
    missing.envelope.projection = None;
    // Producer envelope admission alone permits hash-only generic protocols;
    // this consumer deliberately requires the real inline-chain contract.
    missing
        .envelope
        .admit(&protocol, &protocol.admit().expect("profile"))
        .expect("generic hash-only envelope");
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &missing),
        Err(super::Issue::MissingInline)
    );
    let mut raw = original;
    raw.envelope.projection_ref = format!("blake3:{}", blake3::hash(raw.envelope.projection.as_ref().expect("inline")));
    assert_eq!(super::validate(&protocol, &plan, &super::fixtures::id("group"), &raw), Err(super::Issue::Envelope));
}

#[test]
fn preserves_a_real_producer_safety_failure_instead_of_repairing_it() {
    let (protocol, plan) = super::fixtures::profiles();
    let mut chains = Vec::with_capacity(2);
    for participant in 0..2 {
        let mut committed = super::fixtures::apply(&protocol, participant, 1, b"same-command");
        if participant == 1 {
            committed.application_state_ref = super::fixtures::id("divergent-application-state");
        }
        let observed = super::fixtures::projector(&plan).project(&committed).expect("real chain transition");
        let encoded = super::project(&protocol, &plan, &super::fixtures::context(1), &observed)
            .expect("valid source and transition");
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &encoded)
            .expect("well-formed evidence is not a safety pass");
        chains.push(encoded.apply.chain);
    }
    let report = chaoscontrol_smr::validate_history(&plan, &chains).expect("producer safety evaluation");
    assert_eq!(report.verdict, chaoscontrol_smr::SafetyVerdict::Fail);
    assert!(
        report
            .violations
            .iter()
            .any(|violation| { violation.class == chaoscontrol_smr::SafetyViolationClass::Divergence })
    );
}

#[test]
fn invalid_reference_and_stale_committed_generation_are_typed_denials() {
    let (protocol, plan) = super::fixtures::profiles();
    let mut committed = super::fixtures::observed(&protocol, &plan);
    committed.lifecycle_generation -= 1;
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &committed),
        Err(super::Issue::Generation)
    );
    let mut committed = super::fixtures::observed(&protocol, &plan);
    committed.replica_ref = committed.replica_ref.to_uppercase();
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &committed),
        Err(super::Issue::Reference)
    );
    let mut source = super::fixtures::context(1);
    source.scheduler_ref = "arbitrary scheduler name".into();
    assert_eq!(
        super::project(&protocol, &plan, &source, &super::fixtures::observed(&protocol, &plan)),
        Err(super::Issue::Reference)
    );
    let mut encoded = super::fixtures::bound(&protocol, &plan);
    encoded.apply.chain.command_index = 0;
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &encoded),
        Err(super::Issue::CommandBounds)
    );
}
