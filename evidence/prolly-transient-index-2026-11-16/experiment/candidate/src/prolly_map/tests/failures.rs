use super::*;

// r[verify molten.prolly.publication.classification]
#[test]
fn a_commit_failure_on_a_mutating_phase_is_unknown() {
    for phase in [
        ProllyPortPhase::Staging,
        ProllyPortPhase::Publication,
        ProllyPortPhase::Deletion,
    ] {
        let outcome = classify_port_failure(phase, ProllyPortFailureKind::Commit);
        assert_eq!(outcome, ProllyPortFailureOutcome::Unknown, "a commit may have become durable");
        assert!(outcome.is_unknown());
    }
}

// r[verify molten.prolly.publication.classification]
#[test]
fn a_failure_before_any_mutation_is_not_unknown() {
    for phase in [
        ProllyPortPhase::Validation,
        ProllyPortPhase::Staging,
        ProllyPortPhase::Publication,
        ProllyPortPhase::Readback,
        ProllyPortPhase::Deletion,
    ] {
        for kind in [
            ProllyPortFailureKind::Read,
            ProllyPortFailureKind::Codec,
            ProllyPortFailureKind::Validation,
        ] {
            let outcome = classify_port_failure(phase, kind);
            assert!(!outcome.is_unknown(), "a {kind:?} failure in {phase:?} has a known outcome");
        }
    }
}

// r[verify molten.prolly.publication.classification]
#[test]
fn a_validation_failure_is_a_definite_rejection() {
    let outcome = classify_port_failure(ProllyPortPhase::Publication, ProllyPortFailureKind::Validation);
    assert_eq!(outcome, ProllyPortFailureOutcome::DefiniteRejection);
    assert!(!outcome.is_unknown());
}

// r[verify molten.prolly.publication.classification]
#[test]
fn a_read_or_codec_failure_before_a_mutation_did_not_apply() {
    let read = classify_port_failure(ProllyPortPhase::Publication, ProllyPortFailureKind::Read);
    assert_eq!(read, ProllyPortFailureOutcome::NotApplied);
    let codec = classify_port_failure(ProllyPortPhase::Staging, ProllyPortFailureKind::Codec);
    assert_eq!(codec, ProllyPortFailureOutcome::NotApplied);
}
