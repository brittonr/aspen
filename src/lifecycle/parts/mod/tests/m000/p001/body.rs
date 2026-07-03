
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

    const MATRIX_ENTITY_ID: &str = "matrix-entity";
    const MATRIX_CAUSE: &str = "matrix-proof";
    const MATRIX_LOGICAL_STEP: u64 = 13;
    const PATH_EDGE_WINDOW: usize = 2;

    fn matrix_transition_input(
        from_state: super::State,
        to_state: super::State,
        action: super::Action,
    ) -> super::TransitionInput {
        super::TransitionInput {
            entity_kind: super::EntityKind::Service,
            entity_id: MATRIX_ENTITY_ID.to_owned(),
            from_state,
            to_state,
            action,
            cause: MATRIX_CAUSE.to_owned(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: MATRIX_LOGICAL_STEP,
        }
    }

    fn matching_action_for_target(to_state: super::State) -> super::Action {
        super::action_target_relation()
            .iter()
            .find(|target| target.to_state == to_state)
            .map_or(super::Action::SupervisorDecision, |target| target.action)
    }

    fn mismatched_action_for_target(to_state: super::State) -> super::Action {
        super::lifecycle_actions()
            .iter()
            .copied()
            .find(|action| *action != super::Action::SupervisorDecision && !super::action_matches_target(*action, to_state))
            .expect("finite action set contains at least one mismatched action for every state")
    }

    fn contains_diagnostic(receipt: &super::TransitionReceipt, expected: &str) -> bool {
        receipt.diagnostics.iter().any(|diagnostic| diagnostic == expected)
    }

    #[test]
    fn lifecycle_transition_relation_table_is_unique_and_exposes_finite_sets() {
        // r[verify molten.lifecycle_state_machine_proof.transition_relation_table]
        assert_eq!(super::lifecycle_states().len(), super::LIFECYCLE_STATE_COUNT);
        assert_eq!(super::lifecycle_actions().len(), super::LIFECYCLE_ACTION_COUNT);
        assert_eq!(
            super::allowed_transition_relation().len(),
            super::LIFECYCLE_TRANSITION_COUNT
        );
        assert_eq!(super::action_target_relation().len(), super::LIFECYCLE_ACTION_TARGET_COUNT);
        assert!(super::lifecycle_states().iter().all(|state| !state.as_str().is_empty()));
        assert!(super::lifecycle_actions().iter().all(|action| !action.as_str().is_empty()));

        for (left_index, left) in super::allowed_transition_relation().iter().enumerate() {
            for right in super::allowed_transition_relation().iter().skip(left_index + 1) {
                assert_ne!(left, right, "duplicate lifecycle transition edge");
            }
        }
    }

    #[test]
    fn every_declared_lifecycle_edge_passes_with_matching_action() {
        // r[verify molten.lifecycle_state_machine_proof.transition_relation_table]
        // r[verify molten.lifecycle_state_machine_proof.action_target_matrix]
        for transition in super::allowed_transition_relation() {
            let input = matrix_transition_input(
                transition.from_state,
                transition.to_state,
                matching_action_for_target(transition.to_state),
            );
            let receipt = super::transition_receipt(&input).expect("matrix receipt");

            assert_eq!(receipt.decision, "pass");
            assert!(receipt.diagnostics.is_empty());
        }
    }

    #[test]
    fn supervisor_decision_is_explicit_action_target_escape_hatch_for_allowed_edges() {
        // r[verify molten.lifecycle_state_machine_proof.action_target_matrix]
        for transition in super::allowed_transition_relation() {
            let input = matrix_transition_input(
                transition.from_state,
                transition.to_state,
                super::Action::SupervisorDecision,
            );
            let receipt = super::transition_receipt(&input).expect("supervisor matrix receipt");

            assert_eq!(receipt.decision, "pass");
            assert!(receipt.diagnostics.is_empty());
        }
    }

    #[test]
    fn every_unlisted_lifecycle_edge_denies() {
        // r[verify molten.lifecycle_state_machine_proof.transition_relation_table]
        for from_state in super::lifecycle_states() {
            for to_state in super::lifecycle_states() {
                if super::allowed_transition(*from_state, *to_state) {
                    continue;
                }
                let input = matrix_transition_input(*from_state, *to_state, matching_action_for_target(*to_state));
                let receipt = super::transition_receipt(&input).expect("matrix denial receipt");
                let expected = format!("invalid transition {} -> {}", from_state.as_str(), to_state.as_str());

                assert_eq!(receipt.decision, "deny");
                assert!(contains_diagnostic(&receipt, &expected));
            }
        }
    }

    #[test]
    fn mismatched_actions_deny_even_for_allowed_edges() {
        // r[verify molten.lifecycle_state_machine_proof.action_target_matrix]
        for transition in super::allowed_transition_relation() {
            let action = mismatched_action_for_target(transition.to_state);
            let input = matrix_transition_input(transition.from_state, transition.to_state, action);
            let receipt = super::transition_receipt(&input).expect("action mismatch receipt");
            let expected = format!(
                "action {} does not match target state {}",
                action.as_str(),
                transition.to_state.as_str()
            );

            assert_eq!(receipt.decision, "deny");
            assert_eq!(receipt.diagnostics, vec![expected]);
        }
    }

    fn assert_path_passes(path: &[super::State]) {
        for pair in path.windows(PATH_EDGE_WINDOW) {
            let from_state = pair[0];
            let to_state = pair[1];
            let input = matrix_transition_input(from_state, to_state, matching_action_for_target(to_state));
            let receipt = super::transition_receipt(&input).expect("path receipt");

            assert_eq!(receipt.decision, "pass");
            assert!(receipt.diagnostics.is_empty());
        }
    }

    fn assert_transition_denies(from_state: super::State, to_state: super::State) {
        let input = matrix_transition_input(from_state, to_state, super::Action::SupervisorDecision);
        let receipt = super::transition_receipt(&input).expect("denial receipt");
        let expected = format!("invalid transition {} -> {}", from_state.as_str(), to_state.as_str());

        assert_eq!(receipt.decision, "deny");
        assert!(contains_diagnostic(&receipt, &expected));
    }

    #[test]
    fn lifecycle_reachability_from_declared_covers_expected_paths() {
        // r[verify molten.lifecycle_state_machine_proof.reachability]
        let reachable = super::reachable_lifecycle_states(super::State::Declared);
        for state in super::lifecycle_states() {
            assert!(reachable.contains(state), "state must be reachable: {}", state.as_str());
            assert!(super::lifecycle_state_reachable(super::State::Declared, *state));
        }

        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
        ]);
        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
            super::State::Degraded,
            super::State::Stopping,
            super::State::Stopped,
            super::State::Cleaned,
        ]);
        assert_path_passes(&[
            super::State::Declared,
            super::State::Spawning,
            super::State::Starting,
            super::State::Ready,
            super::State::Failed,
            super::State::Restarting,
            super::State::Starting,
        ]);
    }

    #[test]
    fn forbidden_lifecycle_shortcuts_deny() {
        // r[verify molten.lifecycle_state_machine_proof.reachability]
        assert_transition_denies(super::State::Declared, super::State::Ready);
        assert_transition_denies(super::State::Ready, super::State::Cleaned);
    }

    #[test]
    fn cleaned_state_has_no_outgoing_passing_transition() {
        // r[verify molten.lifecycle_state_machine_proof.terminal_cleanup]
        assert!(super::lifecycle_successor_states(super::State::Cleaned).is_empty());
        for to_state in super::lifecycle_states() {
            assert_transition_denies(super::State::Cleaned, *to_state);
        }
    }

    #[test]
    fn terminal_and_cleanup_boundary_successors_are_closed() {
        // r[verify molten.lifecycle_state_machine_proof.terminal_cleanup]
        assert_eq!(
            super::lifecycle_successor_states(super::State::Stopped),
            vec![super::State::Cleaned]
        );
        assert_eq!(
            super::lifecycle_successor_states(super::State::Failed),
            vec![super::State::Restarting, super::State::Cleaned]
        );
        assert_eq!(
            super::lifecycle_successor_states(super::State::Restarting),
            vec![super::State::Starting, super::State::Cleaned]
        );

        assert_transition_denies(super::State::Stopped, super::State::Starting);
        assert_transition_denies(super::State::Failed, super::State::Ready);
        assert_transition_denies(super::State::Restarting, super::State::Ready);
    }
