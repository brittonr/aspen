
    fn session_fixture_peer() -> &'static str {
        "peer:b"
    }

    fn session_fixture_topic() -> &'static str {
        "services"
    }

    fn session_assert_delivery(root: &Path, payload: IoValue) -> Delivery {
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: session_fixture_peer(),
            topic: session_fixture_topic(),
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("session envelope");
        publish_local_gossip(root, &envelope, "peer:a").expect("publish session envelope");
        deliver_local_gossip(root, session_fixture_topic(), &envelope.envelope_ref, session_fixture_peer())
            .expect("deliver session envelope")
    }

    fn session_admit(
        state: &mut RuntimeState,
        sessions: &RemoteSessionRegistry,
        session_ref: &str,
        delivery: &Delivery,
    ) -> SessionApplied {
        match admit_and_apply_delivered_envelope_for_session(state, sessions, session_ref, delivery, &evidence_fixture())
        {
            Ok(SessionAdmission::Applied(applied)) => applied,
            Ok(SessionAdmission::Denied(denied)) => {
                panic!("expected session admission to pass: {:?}", denied.diagnostics)
            }
            Err(error) => panic!("expected session admission to resolve: {error}"),
        }
    }

    #[test]
    fn remote_assertion_is_session_owned_and_owner_is_readable_from_evidence() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        let root = temp_dir("remote-dataspace-session-owner");
        let payload = record("service-ready", vec![string("db")]);
        let delivery = session_assert_delivery(&root, payload);
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        crate::preserves_rail::validate_content_ref(&session.session_ref).expect("session ref is canonical");
        let mut state = RuntimeState::new(1);
        let applied = session_admit(&mut state, &sessions, &session.session_ref, &delivery);
        assert_eq!(applied.owner, remote_session_owner(session_fixture_peer(), session_fixture_topic(), 1));
        let asserted_value = RuntimeValue::new(record("service-ready", vec![string("db")])).expect("runtime value");
        assert!(applied.events.iter().any(|event| matches!(event, RuntimeEvent::AssertionCommitted { actor, value }
            if actor == &applied.owner && value == &asserted_value)));
        let snapshot = state.snapshot();
        let assertion = snapshot
            .assertions
            .iter()
            .find(|assertion| assertion.actor == applied.owner)
            .expect("session-owned assertion lives in runtime state");
        assert_eq!(assertion.value, asserted_value);
        let applied_assertion = applied.applied_assertion_value.expect("applied assertion record");
        assert_eq!(
            crate::ledger::artifact_kind(&applied_assertion),
            "remote-dataspace-applied-assertion"
        );
        let text = crate::preserves_rail::to_text(&applied_assertion).expect("applied assertion text");
        assert!(text.contains(&applied.owner), "{text}");
        assert!(text.contains(&session.session_ref), "{text}");
        assert!(text.contains("owner-is-receiving-session"), "{text}");
    }

    #[test]
    fn session_close_retracts_peer_facts_and_notifies_observers() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        let root = temp_dir("remote-dataspace-session-close");
        let payload = record("service-ready", vec![string("db")]);
        let pattern = RuntimeValue::new(payload.clone()).expect("observer pattern");
        let delivery = session_assert_delivery(&root, payload);
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Observe {
            actor: "consumer".to_owned(),
            pattern,
        });
        let applied = session_admit(&mut state, &sessions, &session.session_ref, &delivery);
        let owner = applied.owner.clone();
        let closed = close_remote_session_for_disconnect(&mut state, &mut sessions, &session.session_ref)
            .expect("close session");
        assert_eq!(closed.owner, owner);
        assert!(closed.retraction_events.iter().any(|event| matches!(
            event,
            RuntimeEvent::AssertionRetracted { actor, .. } if actor == &owner
        )));
        assert!(closed.retraction_events.iter().any(|event| matches!(
            event,
            RuntimeEvent::AssertionRetractionObserved { observer, owner: event_owner, .. }
                if observer == "consumer" && event_owner == &owner
        )));
        assert_eq!(closed.cleanup.actor, owner);
        assert_eq!(closed.cleanup.assertion_refs.len(), 1);
        assert_eq!(closed.cleanup_decision, "pass");
        assert_ne!(closed.before_state_ref, closed.after_state_ref);
        assert!(state.snapshot().assertions.iter().all(|assertion| assertion.actor != owner));
        assert_eq!(sessions.session(&session.session_ref).expect("closed session").state, RemoteSessionState::Closed);
    }

    #[test]
    fn late_delivery_to_closed_session_denies_before_staging() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        let root = temp_dir("remote-dataspace-session-late");
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        let mut state = RuntimeState::new(1);
        let first = session_assert_delivery(&root, record("service-ready", vec![string("db")]));
        session_admit(&mut state, &sessions, &session.session_ref, &first);
        close_remote_session_for_disconnect(&mut state, &mut sessions, &session.session_ref)
            .expect("close session before late delivery");
        let late = session_assert_delivery(&root, record("service-ready", vec![string("cache")]));
        let before_ref = state.snapshot().snapshot_ref().expect("state before late delivery");
        match admit_and_apply_delivered_envelope_for_session(
            &mut state,
            &sessions,
            &session.session_ref,
            &late,
            &evidence_fixture(),
        )
        .expect("late delivery resolves to a denial")
        {
            SessionAdmission::Denied(denied) => {
                assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("is closed")), "{:?}",
                    denied.diagnostics);
                assert_eq!(
                    crate::ledger::artifact_kind(&denied.admission_receipt_value),
                    "remote-dataspace-admission-receipt"
                );
                assert!(crate::preserves_rail::to_text(&denied.admission_receipt_value)
                    .expect("deny receipt text")
                    .contains("deny"));
            }
            SessionAdmission::Applied(applied) => panic!("late delivery must not apply: {:?}", applied.events),
        }
        assert_eq!(state.snapshot().snapshot_ref().expect("state after late delivery"), before_ref);
    }

    #[test]
    fn unknown_declared_owner_denies_before_staging() {
        let sessions = RemoteSessionRegistry::new();
        let root = temp_dir("remote-dataspace-session-unknown-owner");
        let delivery = session_assert_delivery(&root, record("service-ready", vec![string("db")]));
        let unknown_ref = crate::preserves_rail::canonical_hash(&record("unknown-session", vec![string("peer:b")]))
            .expect("unknown session ref");
        let mut state = RuntimeState::new(1);
        let before_ref = state.snapshot().snapshot_ref().expect("state before unknown owner");
        match admit_and_apply_delivered_envelope_for_session(
            &mut state,
            &sessions,
            &unknown_ref,
            &delivery,
            &evidence_fixture(),
        )
        .expect("unknown owner resolves to a denial")
        {
            SessionAdmission::Denied(denied) => {
                assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("is unknown")), "{:?}",
                    denied.diagnostics);
            }
            SessionAdmission::Applied(applied) => panic!("unknown owner must not apply: {:?}", applied.events),
        }
        assert_eq!(state.snapshot().snapshot_ref().expect("state after unknown owner"), before_ref);
    }

    #[test]
    fn session_identity_cannot_be_reused_after_close() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        let mut state = RuntimeState::new(1);
        close_remote_session_for_disconnect(&mut state, &mut sessions, &session.session_ref).expect("close session");
        let reused = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect_err("closed session identity cannot be reused");
        assert!(reused.to_string().contains("cannot be reused"));
        let reopened = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 2)
            .expect("reconnect opens a fresh session identity");
        assert_ne!(reopened.session_ref, session.session_ref);
        assert_ne!(reopened.owner, session.owner);
    }

    #[test]
    fn replay_for_closed_session_stays_diagnostic_and_requires_reassertion() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        let root = temp_dir("remote-dataspace-session-replay");
        let delivery = session_assert_delivery(&root, record("service-ready", vec![string("db")]));
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        let mut state = RuntimeState::new(1);
        session_admit(&mut state, &sessions, &session.session_ref, &delivery);
        let log = delivery_log(std::slice::from_ref(&delivery), true).expect("delivery log");
        close_remote_session_for_disconnect(&mut state, &mut sessions, &session.session_ref).expect("close session");
        let after_close_ref = state.snapshot().snapshot_ref().expect("state after close");
        let replayed = replay_delivery_log_for_session(&mut state, &sessions, &session.session_ref, &log)
            .expect("closed-session replay");
        assert!(replayed.is_diagnostic_only);
        assert!(replayed.events.is_empty());
        assert!(replayed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("must re-assert")), "{:?}", replayed.diagnostics);
        assert_eq!(state.snapshot().snapshot_ref().expect("state after replay"), after_close_ref);

        let reconnect = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 2)
            .expect("reconnect session");
        let fresh = session_assert_delivery(&root, record("service-ready", vec![string("db")]));
        let applied = session_admit(&mut state, &sessions, &reconnect.session_ref, &fresh);
        assert!(state
            .snapshot()
            .assertions
            .iter()
            .any(|assertion| assertion.actor == applied.owner), "peer must re-assert after reconnect");
    }

    #[test]
    fn open_session_replay_matches_observed_session_apply() {
        // r[verify molten.runtime_spine.remote_assertion_ownership]
        const REPLAY_SEED: u64 = 1;
        let root = temp_dir("remote-dataspace-session-replay-equivalence");
        let delivery = session_assert_delivery(&root, record("service-ready", vec![string("db")]));
        let mut sessions = RemoteSessionRegistry::new();
        let session = sessions
            .open_session(session_fixture_peer(), session_fixture_topic(), 1)
            .expect("open session");
        let mut observed_state = RuntimeState::new(REPLAY_SEED);
        let observed = session_admit(&mut observed_state, &sessions, &session.session_ref, &delivery);
        let observed_state_ref = observed_state.snapshot().snapshot_ref().expect("observed state ref");
        let log = delivery_log(std::slice::from_ref(&delivery), true).expect("delivery log");
        let mut replay_state = RuntimeState::new(REPLAY_SEED);
        let replayed = replay_delivery_log_for_session(&mut replay_state, &sessions, &session.session_ref, &log)
            .expect("session replay");
        assert!(!replayed.is_diagnostic_only);
        assert_eq!(replayed.events, observed.events);
        assert_eq!(
            replay_state.snapshot().snapshot_ref().expect("replayed state ref"),
            observed_state_ref
        );
    }
