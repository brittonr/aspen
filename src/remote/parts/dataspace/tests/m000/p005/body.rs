    fn introduction_ref(root: &Path) -> String {
        store_content_blob(root, b"introduced payload").expect("store the introduced blob")
    }

    fn unknown_ref(root: &Path) -> String {
        store_content_blob(root, b"unintroduced payload").expect("store the unintroduced blob")
    }

    fn introduction_payload(reference: &str) -> IoValue {
        record("service-ready", vec![string(reference)])
    }

    fn introduction_envelope(
        root: &Path,
        payload: IoValue,
        content_refs: Vec<String>,
    ) -> Delivery {
        let envelope = build_envelope(EnvelopeInput {
            from_peer: "peer:a".to_string(),
            from_actor: "producer".to_string(),
            to_peer: session_fixture_peer().to_string(),
            topic: session_fixture_topic().to_string(),
            operation: Operation::Assert,
            payload,
            content_refs,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        }, None)
        .expect("introduction envelope");
        publish_local_gossip(root, &envelope, "peer:a", None).expect("publish introduction envelope");
        deliver_local_gossip(root, session_fixture_topic(), &envelope.envelope_ref, session_fixture_peer())
            .expect("deliver introduction envelope")
    }

    fn incoming_introduction_message(root: &Path, reference: &str) -> Delivery {
        let envelope = incoming_envelope_fixture(EnvelopeInput {
            from_peer: "peer:a".to_owned(), from_actor: "producer".to_owned(),
            to_peer: session_fixture_peer().to_owned(), topic: session_fixture_topic().to_owned(),
            operation: Operation::Message, payload: record("service-note", vec![string(reference)]),
            content_refs: Vec::new(), capability_refs: Vec::new(), evidence_refs: Vec::new(),
        }).expect("incoming untrusted message wire");
        incoming_publish_fixture(root, &envelope, "peer:a").expect("publish incoming fixture");
        deliver_local_gossip(root, session_fixture_topic(), &envelope.envelope_ref, session_fixture_peer())
            .expect("receive incoming fixture")
    }

    fn introduction_session(
        sessions: &mut RemoteSessionRegistry,
        bootstrap_references: Vec<String>,
    ) -> RemoteSession {
        sessions
            .open_session_with_bootstrap(
                session_fixture_peer(),
                session_fixture_topic(),
                1,
                &bootstrap_references,
            )
            .expect("open session")
    }

    #[test]
    fn an_assertion_introduces_a_reference_and_a_later_message_is_admitted() {
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let root = temp_dir("remote-dataspace-introduction");
        let reference = introduction_ref(&root);
        let delivery = introduction_envelope(&root, introduction_payload(&reference), Vec::new());
        let mut sessions = RemoteSessionRegistry::new();
        let session = introduction_session(&mut sessions, Vec::new());
        let mut state = RuntimeState::new(1);
        session_admit(&mut state, &sessions, &session.session_ref, &delivery);

        let introduced = IntroducedReferences::derive(&state, &session.owner, &session.bootstrap_references);
        assert!(introduced.contains(&reference), "the assertion introduces the reference");

        let carrying = incoming_introduction_message(&root, &reference);
        let applied = session_admit(&mut state, &sessions, &session.session_ref, &carrying);
        assert_eq!(applied.session_ref, session.session_ref);
    }

    #[test]
    fn an_unknown_reference_denies_before_delivery_with_no_partial_state() {
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let root = temp_dir("remote-dataspace-unknown-reference");
        let reference = unknown_ref(&root);
        let delivery = incoming_introduction_message(&root, &reference);
        let mut sessions = RemoteSessionRegistry::new();
        let session = introduction_session(&mut sessions, Vec::new());
        let mut state = RuntimeState::new(1);
        let before = state.snapshot();
        let admission = admit_and_apply_delivered_envelope_for_session(
            &mut state,
            &sessions,
            &session.session_ref,
            &delivery,
            &evidence_fixture(),
        )
        .expect("session admission resolves");
        let denied = match admission {
            SessionAdmission::Denied(denied) => denied,
            SessionAdmission::Applied(applied) => {
                panic!("expected a denial, applied: {}", applied.session_ref)
            }
        };
        assert!(
            denied
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains(&reference)),
            "the denial names the unknown reference: {:?}",
            denied.diagnostics
        );
        assert_eq!(state.snapshot(), before, "the denial leaves no partial state");
    }

    #[test]
    fn a_retracted_assertion_withdraws_the_introduction() {
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let root = temp_dir("remote-dataspace-withdrawn-introduction");
        let reference = introduction_ref(&root);
        let mut sessions = RemoteSessionRegistry::new();
        let session = introduction_session(&mut sessions, Vec::new());
        let mut state = RuntimeState::new(1);

        let assertion = introduction_envelope(&root, introduction_payload(&reference), Vec::new());
        session_admit(&mut state, &sessions, &session.session_ref, &assertion);
        let introduced = IntroducedReferences::derive(&state, &session.owner, &session.bootstrap_references);
        assert!(introduced.contains(&reference));

        state.apply_step(&RuntimeStep::Retract {
            actor: session.owner.clone(),
            value: RuntimeValue::new(introduction_payload(&reference)).expect("runtime value"),
        });
        let introduced = IntroducedReferences::derive(&state, &session.owner, &session.bootstrap_references);
        assert!(!introduced.contains(&reference), "the withdrawn assertion no longer introduces");
        let carrying = incoming_introduction_message(&root, &reference);
        let before = state.snapshot();
        assert!(matches!(admit_and_apply_delivered_envelope_for_session(
            &mut state, &sessions, &session.session_ref, &carrying, &evidence_fixture(),
        ).expect("message admission"), SessionAdmission::Denied(_)));
        assert_eq!(state.snapshot(), before);
    }


    #[test]
    fn the_bootstrap_references_are_introduced_and_a_closed_session_keeps_no_facts() {
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let root = temp_dir("remote-dataspace-bootstrap-introduction");
        let reference = introduction_ref(&root);
        let mut sessions = RemoteSessionRegistry::new();
        let session = introduction_session(&mut sessions, vec![reference.clone()]);
        let mut state = RuntimeState::new(1);
        let introduced = IntroducedReferences::derive(&state, &session.owner, &session.bootstrap_references);
        assert!(introduced.contains(&reference), "the bootstrap reference is introduced");

        let delivery = introduction_envelope(&root, introduction_payload(&reference), Vec::new());
        session_admit(&mut state, &sessions, &session.session_ref, &delivery);
        let closed = close_remote_session(&mut state, &mut sessions, &session.session_ref, "session closed")
            .expect("close the session");
        assert_eq!(closed.session_ref, session.session_ref);
        let snapshot = state.snapshot();
        assert!(
            !snapshot
                .assertions
                .iter()
                .any(|assertion| assertion.actor == session.owner),
            "the closed session keeps no facts"
        );
    }

    #[test]
    fn reference_scanning_handles_unicode_before_and_inside_candidates() {
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let reference = crate::preserves_rail::content_ref_from_bytes(b"unicode reference");
        let text = format!("état 🦀 {reference} 漢字 {reference}");
        assert_eq!(references_in_text(&text), vec![reference.clone()]);

        let malformed = format!("blake3:{}é {}", "a".repeat(63), reference);
        assert_eq!(references_in_text(&malformed), vec![reference]);
        assert!(references_in_text(&format!("漢字 blake3:{}", "a".repeat(63))).is_empty());
    }
