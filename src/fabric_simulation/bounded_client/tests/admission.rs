use super::*;

// r[verify molten.chaoscontrol_client.bounded_input]
#[test]
fn canonical_input_round_trips_and_rejects_invalid_boundaries() {
    let (input, _) = fixture();
    let bytes = crate::preserves_rail::canonical_bytes(&input_value(&input)).expect("pack");
    assert_eq!(decode_input(&bytes).expect("decode"), input);
    assert!(decode_input(&vec![0; MAX_CLIENT_INPUT_BYTES + 1]).is_err());
    let mut wrong = input.clone();
    wrong.steps.swap(0, 4);
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.schema = "v999".to_string();
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.participants.pop();
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.max_payload_bytes = 1;
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.recovery_ticks = MAX_RECOVERY_TICKS + 1;
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.steps[1].request_ref = wrong.steps[0].request_ref.clone();
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.submit_request_ref = reference("substitution");
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.command_ref = reference("different-command");
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.max_event_loop_iterations = 0;
    assert!(wrong.validate().is_err());
    wrong = input.clone();
    wrong.child_timeout_seconds = 0;
    assert!(wrong.validate().is_err());
    assert!(Input::native(0, SemanticControl::Healthy).is_err());
    for length in 1..=CLIENT_PHASES.len() as u32 {
        assert!(input.valid_prefix(length).expect("prefix").validate().is_ok());
    }
    assert!(input.valid_prefix(0).is_err());
    assert!(input.valid_prefix(u32::MAX).is_err());
}

// r[verify molten.chaoscontrol_client.bounded_input]
#[test]
fn canonical_phase_requests_reject_arbitrary_and_swapped_valid_refs() {
    let (input, _) = fixture();
    for (index, replacement) in input.steps.iter().cycle().skip(1).take(input.steps.len()).enumerate() {
        let mut substituted = input.clone();
        substituted.steps[index].request_ref = reference("otherwise-valid-phase-substitution");
        assert!(substituted.validate().is_err());
        let bytes = crate::preserves_rail::canonical_bytes(&input_value(&substituted)).expect("pack substitution");
        assert!(decode_input(&bytes).is_err());
        let mut swapped = input.clone();
        swapped.steps[index].request_ref = replacement.request_ref.clone();
        assert!(swapped.validate().is_err());
    }
}

// r[verify molten.chaoscontrol_client.bounded_input]
#[test]
fn reduction_retains_original_and_only_valid_failure_preserving_prefixes() {
    let input = Input::native(4, SemanticControl::DuplicateApplication).expect("input");
    let reduced = reduce_valid_prefix(&input, APPLY_ONCE_PROPERTY, |candidate, property| {
        assert_eq!(property, APPLY_ONCE_PROPERTY);
        candidate.validate()?;
        Ok(candidate.steps.len() >= 3)
    })
    .expect("reduce");
    assert_eq!(reduced.original, input);
    assert_eq!(reduced.prefix.steps.len(), 3);
    assert!(reduced_value(&reduced).is_ok());
    assert!(reduce_valid_prefix(&input, APPLY_ONCE_PROPERTY, |_, _| Ok(false)).is_err());
}

// r[verify molten.chaoscontrol_client.guest_boundary]
#[test]
fn guest_handoff_rejects_smp_and_unavailable_dependency_identities() {
    let dependencies = GuestDependencies {
        vcpus_per_guest: 2,
        source_revision: "unavailable".to_string(),
        source_ref: reference("source"),
        binary_ref: None,
        image_ref: None,
        kernel_ref: None,
        dependency_closure_ref: None,
        adapter_ref: reference("adapter"),
        oracle_ref: reference("oracle"),
        workload_schema_ref: reference("schema"),
        property_catalog_ref: reference("properties"),
        generator_ref: reference("generator"),
    };
    let blockers = guest_dependency_blockers(&dependencies);
    assert!(blockers.iter().any(|blocker| blocker.contains("vCPU")));
    assert!(blockers.iter().any(|blocker| blocker.contains("dependency closure")));
    let complete = GuestDependencies {
        vcpus_per_guest: 1,
        source_revision: "1".repeat(40),
        binary_ref: Some(reference("binary")),
        image_ref: Some(reference("image")),
        kernel_ref: Some(reference("kernel")),
        dependency_closure_ref: Some(reference("closure")),
        ..dependencies
    };
    assert!(guest_dependency_blockers(&complete).is_empty());
}
