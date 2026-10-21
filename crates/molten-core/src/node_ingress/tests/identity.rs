use super::*;

#[test]
fn lowercase_hex_references_and_bounded_operation_tokens_are_accepted() {
    let reference = "blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let mut observation = input(DedupObservation::First);
    observation.operation_ref = reference;
    observation.operation = "control.status:v1-foo_bar";
    observation.intent = Some(PublicationIntentObservation {
        operation_ref: reference,
        ..intent(PublicationPhase::Reserved)
    });
    assert_eq!(reconcile(&observation), PublicationDecision::PublishFresh);
}

// r[verify molten.audit_f03.uncertainty]
#[test]
fn malformed_refs_never_grant_publication_even_with_matching_intent() {
    for malformed in [
        "",
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "blake3:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
        concat!("blake3:", "00000000000000000000000000000000", "0000000000000000000000000000000"),
        concat!("blake3:", "00000000000000000000000000000000", "000000000000000000000000000000000"),
        "blake3:1111111111111111111111111111111111111111111111111111111111111111\n",
    ] {
        let mut observation = input(DedupObservation::First);
        observation.operation_ref = malformed;
        observation.intent = Some(PublicationIntentObservation {
            operation_ref: malformed,
            ..intent(PublicationPhase::Reserved)
        });
        assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{malformed:?}");
        observation = input(DedupObservation::Duplicate);
        observation.request_ref = malformed;
        observation.intent = Some(PublicationIntentObservation {
            request_ref: malformed,
            ..intent(PublicationPhase::Reserved)
        });
        assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{malformed:?}");
    }
}

#[test]
fn malformed_control_verbs_do_not_reach_dedup_admission() {
    for operation in ["", "status now", "status\n", "../status", "státus"] {
        let mut observation = input(DedupObservation::Unobserved);
        observation.operation = operation;
        assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{operation:?}");
    }
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn mismatched_intent_blocks_even_matching_dispatch() {
    for mismatched in [
        PublicationIntentObservation {
            operation_ref: OTHER_REF,
            ..intent(PublicationPhase::Reserved)
        },
        PublicationIntentObservation {
            request_ref: OTHER_REF,
            ..intent(PublicationPhase::Reserved)
        },
    ] {
        let mut observation = input(DedupObservation::Duplicate);
        observation.intent = Some(mismatched);
        observation.dispatch = bound();
        assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{mismatched:?}");
    }
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn every_evidence_position_preserves_exact_bindings() {
    for conflicting in [
        PublicationObservation::Conflict,
        PublicationObservation::Bound {
            operation: "shutdown",
            request_ref: REQUEST_REF,
        },
        PublicationObservation::Bound {
            operation: "status",
            request_ref: OTHER_REF,
        },
        PublicationObservation::Bound {
            operation: "status",
            request_ref: "blake3:malformed",
        },
    ] {
        for [inbox, queue, dispatch] in [[conflicting, bound(), bound()], [bound(), conflicting, bound()], [
            bound(),
            bound(),
            conflicting,
        ]] {
            let mut observation = input(DedupObservation::Duplicate);
            observation.inbox = inbox;
            observation.queue = queue;
            observation.dispatch = dispatch;
            assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{observation:?}");
        }
    }
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_observation_blocks_dedup_admission_despite_unavailability() {
    for conflicting in [
        PublicationObservation::Conflict,
        PublicationObservation::Bound {
            operation: "shutdown",
            request_ref: REQUEST_REF,
        },
        PublicationObservation::Bound {
            operation: "status",
            request_ref: OTHER_REF,
        },
    ] {
        let absent = PublicationObservation::Absent;
        let unavailable = PublicationObservation::Unavailable;
        for [inbox, queue, dispatch] in [
            [conflicting, unavailable, absent],
            [absent, conflicting, unavailable],
            [unavailable, absent, conflicting],
        ] {
            let mut observation = input(DedupObservation::Unobserved);
            observation.inbox = inbox;
            observation.queue = queue;
            observation.dispatch = dispatch;
            assert_eq!(reconcile(&observation), PublicationDecision::Conflict, "{observation:?}");
        }
    }
}
