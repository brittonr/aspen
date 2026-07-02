
    #[hegel::test(test_cases = 8)]
    fn hegel_cleanup_idempotence_no_leaks_and_restart_bounds(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(5));
        let actor = format!("actor-{salt}");
        let mut state = crate::runtime::RuntimeState::new(1);
        for index in 0..=(salt % 2) {
            state.apply_step(&crate::runtime::RuntimeStep::Assert {
                actor: actor.clone(),
                value: crate::runtime::RuntimeValue::string(format!("service.ready.{index}")).expect("runtime value"),
            });
        }
        let _first = state.cleanup_actor_scope(&actor).expect("first cleanup");
        let before_second = state.snapshot();
        let second = state.cleanup_actor_scope(&actor).expect("second cleanup");
        let after_second = state.snapshot();
        assert_eq!(before_second, after_second);
        assert!(second.assertion_refs.is_empty());
        assert!(after_second.assertions.iter().all(|assertion| assertion.actor != actor));

        let policy_ref = content_ref_from_bytes(b"restart-policy");
        let failure_ref = content_ref_from_bytes(format!("failure-{salt}").as_bytes());
        let policy = super::SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: super::RestartStrategy::Bounded,
            restart_window: Some(super::RestartWindow {
                start_step: 0,
                end_step: 10,
                max_restarts: 3,
            }),
            policy_refs: vec![policy_ref],
        };
        let receipt = super::supervisor_decision_receipt(&super::SupervisorDecisionInput {
            policy: &policy,
            child_id: &actor,
            child_failure_ref: &failure_ref,
            restart_count_in_window: salt,
            logical_step: 5,
            evidence_refs: &[],
        })
        .expect("restart receipt");
        if salt >= 3 {
            assert_eq!(receipt.decision, "deny");
        } else {
            assert_eq!(receipt.decision, "restart");
        }
    }

    #[test]
    fn transition_receipt_passes_valid_spawn() {
        let policy_ref = content_ref_from_bytes(b"policy");
        let evidence_ref = content_ref_from_bytes(b"evidence");
        let input = super::TransitionInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1".to_owned(),
            from_state: super::State::Declared,
            to_state: super::State::Spawning,
            action: super::Action::Spawn,
            cause: "operator-request".to_owned(),
            policy_refs: vec![policy_ref],
            resource_refs: Vec::new(),
            evidence_refs: vec![evidence_ref],
            supervisor_ref: None,
            logical_step: 1,
        };

        let receipt = super::transition_receipt(&input).expect("receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert!(receipt.receipt_ref.starts_with("blake3:"));
    }

    #[test]
    fn transition_receipt_denies_impossible_jump() {
        let input = super::TransitionInput {
            entity_kind: super::EntityKind::Service,
            entity_id: "svc".to_owned(),
            from_state: super::State::Declared,
            to_state: super::State::Ready,
            action: super::Action::Ready,
            cause: "bad-adapter".to_owned(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 2,
        };

        let receipt = super::transition_receipt(&input).expect("receipt");

        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.diagnostics, vec!["invalid transition declared -> ready".to_owned()]);
    }

    #[test]
    fn trace_event_binds_transition_cause_and_policy() {
        let policy_ref = content_ref_from_bytes(b"policy-a");
        let input = super::TransitionInput {
            entity_kind: super::EntityKind::Job,
            entity_id: "job-7".to_owned(),
            from_state: super::State::Ready,
            to_state: super::State::Failed,
            action: super::Action::Fail,
            cause: "stage-denied".to_owned(),
            policy_refs: vec![policy_ref],
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 9,
        };

        let event = super::trace_event(&input).expect("trace event");
        let rendered = to_text(&event.value).expect("render event");

        assert!(event.event_ref.starts_with("blake3:"));
        assert!(rendered.contains("lifecycle-trace-event-v1"));
        assert!(rendered.contains("stage-denied"));
    }

    #[test]
    fn refs_must_be_sorted_and_canonical() {
        let mut refs = vec![content_ref_from_bytes(b"z"), content_ref_from_bytes(b"a")];
        refs.sort();
        refs.reverse();
        let input = super::TransitionInput {
            entity_kind: super::EntityKind::Vat,
            entity_id: "vat".to_owned(),
            from_state: super::State::Declared,
            to_state: super::State::Spawning,
            action: super::Action::Spawn,
            cause: "test".to_owned(),
            policy_refs: refs,
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: 0,
        };

        let error = super::transition_receipt(&input).expect_err("unsorted refs fail");
        assert!(error.to_string().contains("policy refs must be sorted and unique"));
    }
