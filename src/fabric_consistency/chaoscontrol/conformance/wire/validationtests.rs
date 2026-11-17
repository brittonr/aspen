//! Inline payload, admitted profile, size-bound, and canonical provenance controls.

#[test]
fn inline_payload_and_molten_sidecar_must_agree() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    let mut changed = original.clone();
    changed.apply.chain.application_state_ref = super::fixtures::id("different-state");
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &changed),
        Err(super::Issue::ProjectionMismatch)
    );
    let mut changed = original.clone();
    let mut inline = serde_json::to_value(&changed.apply.chain).expect("chain JSON");
    inline["unexpected"] = true.into();
    let bytes = serde_json::to_vec(&inline).expect("unknown-field JSON");
    changed.envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(&bytes);
    changed.envelope.projection = Some(bytes);
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &changed),
        Err(super::Issue::ProjectionEncoding)
    );
    let mut changed = original;
    changed.envelope.dropped = 1;
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &changed),
        Err(super::Issue::LossAccounting)
    );
}

#[test]
fn supplied_profiles_and_group_are_revalidated_not_trusted_as_cached_plans() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("other-group"), &original),
        Err(super::Issue::Group)
    );
    let mut changed_plan = plan.clone();
    changed_plan.profile.bounds.maximum_commands += 1;
    assert_eq!(
        super::validate(&protocol, &changed_plan, &super::fixtures::id("group"), &original),
        Err(super::Issue::WorkloadProfile)
    );
    let mut changed_plan = plan.clone();
    changed_plan.genesis_digest = super::fixtures::id("wrong-genesis");
    assert_eq!(
        super::validate(&protocol, &changed_plan, &super::fixtures::id("group"), &original),
        Err(super::Issue::WorkloadProfile)
    );
    let mut changed_protocol = protocol.clone();
    changed_protocol.novelty_selection_ref = super::fixtures::id("other-protocol-profile");
    assert_eq!(
        super::validate(&changed_protocol, &plan, &super::fixtures::id("group"), &original),
        Err(super::Issue::Envelope)
    );
    let mut changed_protocol = protocol.clone();
    changed_protocol.independent_oracle_ref = changed_protocol.oracle_adapter_ref.clone();
    assert_eq!(
        super::validate(&changed_protocol, &plan, &super::fixtures::id("group"), &original),
        Err(super::Issue::ProtocolProfile)
    );
    let mut committed = super::fixtures::observed(&protocol, &plan);
    committed.chaoscontrol_profile_ref = protocol.admit().expect("protocol identity");
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &committed),
        Err(super::Issue::WorkloadProfile)
    );
    committed = super::fixtures::observed(&protocol, &plan);
    committed.group_ref = super::fixtures::id("other-group");
    assert_eq!(super::project(&protocol, &plan, &super::fixtures::context(1), &committed), Err(super::Issue::Group));
}

#[test]
fn configured_command_and_envelope_byte_limits_have_exact_boundaries() {
    let (protocol, plan) = super::fixtures::profiles();
    let command_size = usize::try_from(plan.profile.bounds.maximum_command_bytes).expect("host command bound");
    let command = vec![42; command_size];
    let committed = super::fixtures::projector(&plan)
        .project(&super::fixtures::apply(&protocol, 0, 1, &command))
        .expect("maximum command");
    let encoded = super::project(&protocol, &plan, &super::fixtures::context(1), &committed).expect("at command bound");
    super::validate(&protocol, &plan, &super::fixtures::id("group"), &encoded).expect("at command bound accepted");
    let mut too_large = committed;
    too_large.command_bytes.push(42);
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &too_large),
        Err(super::Issue::CommandBounds)
    );
    let mut empty = super::fixtures::observed(&protocol, &plan);
    empty.command_bytes.clear();
    assert_eq!(
        super::project(&protocol, &plan, &super::fixtures::context(1), &empty),
        Err(super::Issue::CommandBounds)
    );

    // The protocol identity itself has a fixed-width digest, so recomputing it
    // after tightening record_bytes does not change the envelope encoded size.
    let exact = u64::try_from(
        serde_json::to_vec(&super::fixtures::bound(&protocol, &plan).envelope)
            .expect("envelope bytes")
            .len(),
    )
    .expect("fixed-width envelope size");
    let mut at_limit = protocol.clone();
    at_limit.bounds.record_bytes = exact;
    super::project(&at_limit, &plan, &super::fixtures::context(1), &super::fixtures::observed(&at_limit, &plan))
        .expect("exact envelope bound");
    let mut below_limit = at_limit;
    below_limit.bounds.record_bytes -= 1;
    assert_eq!(
        super::project(
            &below_limit,
            &plan,
            &super::fixtures::context(1),
            &super::fixtures::observed(&below_limit, &plan),
        ),
        Err(super::Issue::Envelope)
    );

    let mut oversized = encoded;
    let bytes = oversized.envelope.projection.as_mut().expect("inline chain");
    bytes.resize(usize::try_from(super::MAX_INLINE_PROJECTION_BYTES).expect("host inline bound"), b' ');
    oversized.envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(bytes);
    super::validate(&protocol, &plan, &super::fixtures::id("group"), &oversized)
        .expect("bounded JSON whitespace at exact inline limit");
    oversized.envelope.projection.as_mut().expect("inline chain").push(b' ');
    assert_eq!(
        super::validate(&protocol, &plan, &super::fixtures::id("group"), &oversized),
        Err(super::Issue::ProjectionBounds)
    );
}

#[test]
fn canonical_binding_retains_provenance_not_carried_by_the_producer_projection() {
    let (protocol, plan) = super::fixtures::profiles();
    let original = super::fixtures::bound(&protocol, &plan);
    let original_ref = super::binding_ref(&original.apply).expect("canonical binding");
    let encoded = serde_json::to_string_pretty(&original.apply).expect("pretty JSON transport");
    let decoded: super::ApplyBinding = serde_json::from_str(&encoded).expect("same binding data");
    assert_eq!(super::binding_ref(&decoded).expect("canonical data identity"), original_ref);
    for field in ["operation_ref", "command_ref", "application_receipt_ref", "group_ref"] {
        let mut json = serde_json::to_value(&original.apply).expect("binding JSON");
        json[field] = super::fixtures::id("substituted-provenance").into();
        let changed: super::ApplyBinding = serde_json::from_value(json).expect("typed sidecar");
        assert_ne!(super::binding_ref(&changed).expect("structurally valid changed provenance"), original_ref);
        // The wire projection omits these Molten facts; retaining a separate
        // canonical binding is essential to import-policy identity checks.
        assert_eq!(changed.chain, original.apply.chain);
    }
    let mut json = serde_json::to_value(&original).expect("bound JSON");
    json["legacy_upgrade"] = true.into();
    assert!(serde_json::from_value::<super::BoundObservation>(json).is_err());
    let mut json = serde_json::to_value(&original.apply).expect("apply JSON");
    json["legacy_upgrade"] = true.into();
    assert!(serde_json::from_value::<super::ApplyBinding>(json).is_err());
    let mut json = serde_json::to_value(super::fixtures::context(1)).expect("context JSON");
    json["legacy_upgrade"] = true.into();
    assert!(serde_json::from_value::<super::ProjectionContext>(json).is_err());
}
