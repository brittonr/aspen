use super::*;

// r[verify molten.world_replay_boundary.inventory]
// r[verify molten.world_replay_boundary.opaque_native]
#[test]
fn a_native_process_descriptor_binds_exact_identities_and_stays_detached() {
    let mut detached = valid_request();
    let mut native = diagnostic("chaoscontrol-native-cohort", vec![OpaqueDiagnosticClaim::DetachedDiagnostic]);
    native.native_process = Some(NativeProcessDiagnostic {
        profile_ref: digest("native-process-profile"),
        descriptor_ref: digest("native-process-descriptor"),
        producer_cohort: digest("chaoscontrol-native-cohort"),
        replay_class: NativeProcessReplayClass::RecordReplayMatched,
        record_count: 4,
        non_claims: vec![String::from("descriptor does not establish semantic equivalence")],
    });
    detached.boundary.opaque_diagnostics = vec![native];
    let admission = admit(&detached).expect("bound descriptor");
    assert_eq!(admission.opaque_diagnostic_refs, vec![digest("native-process-descriptor")]);

    let descriptor_ref = digest("native-process-descriptor");
    let mut swapped = detached.clone();
    if let Some(native) = &mut swapped.boundary.opaque_diagnostics[0].native_process {
        native.descriptor_ref = digest("another-descriptor");
    }
    assert_eq!(blockers(&swapped), vec![ReplayBoundaryBlocker::InvalidRecord("native-process-descriptor")]);

    let mut foreign = detached.clone();
    if let Some(native) = &mut foreign.boundary.opaque_diagnostics[0].native_process {
        native.producer_cohort = digest("another-cohort");
    }
    assert_eq!(blockers(&foreign), vec![ReplayBoundaryBlocker::OpaqueCohortMismatch(descriptor_ref.clone())]);

    let mut unborn = detached.clone();
    if let Some(native) = &mut unborn.boundary.opaque_diagnostics[0].native_process {
        native.record_count = 0;
    }
    assert!(blockers(&unborn).contains(&ReplayBoundaryBlocker::InvalidRecord("native-process-records")));

    let mut overclaim = detached.clone();
    if let Some(native) = &mut overclaim.boundary.opaque_diagnostics[0].native_process {
        native.non_claims.clear();
    }
    assert_eq!(blockers(&overclaim), vec![ReplayBoundaryBlocker::OpaqueOverclaim(descriptor_ref)]);
}

fn diagnostic(cohort: &str, claims: Vec<OpaqueDiagnosticClaim>) -> OpaqueReplayDiagnostic {
    OpaqueReplayDiagnostic {
        descriptor_ref: digest("native-process-descriptor"),
        byte_length: 4_096,
        cohort_ref: digest(cohort),
        claims,
        native_process: None,
    }
}

// r[verify molten.world_replay_boundary.opaque_native]
#[test]
fn an_opaque_diagnostic_stays_detached_and_satisfies_no_observation() {
    let mut detached = valid_request();
    detached.boundary.opaque_diagnostics = vec![diagnostic("chaoscontrol-native-cohort", vec![
        OpaqueDiagnosticClaim::DetachedDiagnostic,
    ])];
    let admission = admit(&detached).expect("detached diagnostic");
    assert_eq!(admission.opaque_diagnostic_refs, vec![digest("native-process-descriptor")]);
    let required = required_world_replay_boundary_closure(&detached.boundary);
    assert!(
        required
            .iter()
            .any(|requirement| requirement.role == WorldReplayCapsuleMemberRole::OpaqueDiagnostic)
    );

    let mut overclaim = detached.clone();
    overclaim.boundary.opaque_diagnostics[0].claims.push(OpaqueDiagnosticClaim::SatisfiesObservation);
    assert_eq!(blockers(&overclaim), vec![ReplayBoundaryBlocker::OpaqueOverclaim(digest(
        "native-process-descriptor"
    ))]);

    let mut foreign = detached.clone();
    foreign.boundary.opaque_diagnostics[0].cohort_ref = digest("another-cohort");
    assert_eq!(blockers(&foreign), vec![ReplayBoundaryBlocker::OpaqueCohortMismatch(digest(
        "native-process-descriptor"
    ))]);

    let mut replacement = detached;
    with_ledger(&mut replacement, |ledger| {
        ledger.observations.remove(0);
    });
    assert_eq!(blockers(&replacement), vec![ReplayBoundaryBlocker::MissingObservation {
        step_position: first_step(&replacement),
        logical_position: 0,
    }]);
}
