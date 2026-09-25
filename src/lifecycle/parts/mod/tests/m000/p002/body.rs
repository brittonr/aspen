
    #[test]
    fn supervisor_restart_strategies_and_windows_are_deterministic() {
        let policy_ref = content_ref_from_bytes(b"supervisor-policy");
        let failure_ref = content_ref_from_bytes(b"child-failure");
        let one_for_one = super::SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: super::RestartStrategy::OneForOne,
            restart_window: None,
            policy_refs: vec![policy_ref.clone()],
        };
        let restart = super::supervisor_decision_receipt(&super::SupervisorDecisionInput {
            policy: &one_for_one,
            child_id: "child",
            child_failure_ref: &failure_ref,
            restart_count_in_window: 0,
            logical_step: 10,
            evidence_refs: &[],
        })
        .expect("restart decision");
        assert_eq!(restart.decision, "restart");

        let bounded = super::SupervisorPolicy {
            supervisor_id: "sup".to_owned(),
            strategy: super::RestartStrategy::Bounded,
            restart_window: Some(super::RestartWindow {
                start_step: 0,
                end_step: 20,
                max_restarts: 2,
            }),
            policy_refs: vec![policy_ref],
        };
        let denied = super::supervisor_decision_receipt(&super::SupervisorDecisionInput {
            policy: &bounded,
            child_id: "child",
            child_failure_ref: &failure_ref,
            restart_count_in_window: 2,
            logical_step: 10,
            evidence_refs: &[],
        })
        .expect("bounded decision");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.diagnostics, vec!["restart budget exhausted".to_owned()]);
    }

    #[test]
    fn monitor_notifications_bind_failure_refs_deterministically() {
        let policy_ref = content_ref_from_bytes(b"monitor-policy");
        let failure_ref = content_ref_from_bytes(b"service-failure");
        let first = super::monitor_receipt(&super::MonitorInput {
            observer_id: "monitor:service",
            child_id: "service:frontend",
            child_failure_ref: &failure_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: &[],
            logical_step: 11,
        })
        .expect("first monitor receipt");
        let replay = super::monitor_receipt(&super::MonitorInput {
            observer_id: "monitor:service",
            child_id: "service:frontend",
            child_failure_ref: &failure_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: &[],
            logical_step: 11,
        })
        .expect("replayed monitor receipt");
        let rendered = to_text(&first.value).expect("monitor text");
        assert_eq!(first.receipt_ref, replay.receipt_ref);
        assert!(rendered.contains(&failure_ref));
        assert!(rendered.contains("authority-escalated #f"));
    }

    #[test]
    fn service_scope_cleanup_is_idempotent_and_ownership_bound() {
        let mut state = crate::runtime::RuntimeState::new(1);
        let ready = crate::runtime::RuntimeValue::string("service.ready").expect("runtime value");
        state.apply_step(&crate::runtime::RuntimeStep::Assert {
            actor: "service:frontend".to_owned(),
            value: ready,
        });
        let before_first = state.snapshot();
        let first_cleanup = state.cleanup_actor_scope("service:frontend").expect("first cleanup");
        let after_first = state.snapshot();
        let first_input = super::ScopeCleanupInput {
            entity_kind: super::EntityKind::Service,
            entity_id: "service:frontend",
            cause: "stop",
            before: &before_first,
            after_cleanup: &after_first,
            cleanup: &first_cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
            logical_step: 12,
        };
        let first_receipt = super::scope_cleanup_receipt(&first_input)
        .expect("first service cleanup receipt");
        let before_second = state.snapshot();
        let second_cleanup = state.cleanup_actor_scope("service:frontend").expect("second cleanup");
        let after_second = state.snapshot();
        let second_receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput { before: &before_second, after_cleanup: &after_second, cleanup: &second_cleanup, logical_step: 13, ..first_input })
        .expect("second service cleanup receipt");
        let stale_cleanup = crate::runtime::RuntimeScopeCleanup {
            actor: "service:frontend".to_owned(),
            assertion_refs: vec![content_ref_from_bytes(b"stale-assertion")],
            observer_refs: Vec::new(),
            message_refs: Vec::new(),
        };
        let stale_receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput { cause: "stale-cleanup", before: &before_second, after_cleanup: &after_second, cleanup: &stale_cleanup, logical_step: 14, ..first_input })
        .expect("stale service cleanup receipt");
        let non_owned_receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput { entity_id: "service:backend", cause: "wrong-owner", logical_step: 15, ..first_input })
        .expect("non-owned service cleanup receipt");
        assert_eq!(first_receipt.decision, "pass");
        assert_eq!(second_receipt.decision, "pass");
        assert_eq!(before_second, after_second);
        assert!(second_cleanup.assertion_refs.is_empty());
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("state did not change")));
        assert_eq!(non_owned_receipt.decision, "deny");
        assert!(non_owned_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("cleanup actor")));
    }

    #[test]
    fn service_lifecycle_states_are_dataspace_assertions() {
        let evidence_ref = content_ref_from_bytes(b"readiness-evidence");
        let assertion =
            super::service_lifecycle_assertion("service:frontend", super::ServiceAssertionKind::Ready, None, &[
                evidence_ref,
            ])
            .expect("service assertion");
        let mut state = crate::runtime::RuntimeState::new(1);
        state.apply_step(&crate::runtime::RuntimeStep::Assert {
            actor: "service:frontend".to_owned(),
            value: assertion.clone(),
        });

        assert_eq!(state.snapshot().assertions.len(), 1);
        assert!(
            to_text(assertion.as_iovalue())
                .expect("render assertion")
                .contains("lifecycle-service-assertion-v1")
        );
    }

    #[test]
    fn readiness_probe_updates_status() {
        let probe = LifecycleProbe {
            kind: ProbeKind::Readiness,
            success: true,
            observed_generation: 1,
            probe_evidence_ref: content_ref_from_bytes(b"probe-ev"),
            status_condition_ref: Some(content_ref_from_bytes(b"status-ref")),
            policy_refs: vec![],
        };
        let result = evaluate_readiness(&probe, 1).expect("readiness");
        assert_eq!(result, "ready");
    }

    #[test]
    fn restart_with_budget_and_probe_passes() {
        let input = RestartDecisionInput {
            entity_ref: content_ref_from_bytes(b"entity"),
            entity_kind: "service".to_string(),
            current_generation: 1,
            probe_results: vec![LifecycleProbe {
                kind: ProbeKind::Liveness,
                success: false,
                observed_generation: 1,
                probe_evidence_ref: content_ref_from_bytes(b"liveness"),
                status_condition_ref: None,
                policy_refs: vec![],
            }],
            prior_restart_attempts: 0,
            backoff_profile: Some(BackoffProfile {
                name: "default".to_string(),
                initial_delay_ms: 100,
                max_delay_ms: 10000,
                multiplier: 2.0,
                max_attempts: 5,
            }),
            authority_refs: vec![content_ref_from_bytes(b"auth")],
            resource_budget_refs: vec![],
        };
        let decision = evaluate_restart_decision(&input).expect("restart");
        assert_eq!(decision.decision, "pass");
    }

    #[test]
    fn restart_budget_exhausted_denies() {
        let input = RestartDecisionInput {
            entity_ref: content_ref_from_bytes(b"entity"),
            entity_kind: "service".to_string(),
            current_generation: 1,
            probe_results: vec![],
            prior_restart_attempts: 5,
            backoff_profile: Some(BackoffProfile {
                name: "default".to_string(),
                initial_delay_ms: 100,
                max_delay_ms: 10000,
                multiplier: 2.0,
                max_attempts: 5,
            }),
            authority_refs: vec![content_ref_from_bytes(b"auth")],
            resource_budget_refs: vec![],
        };
        let decision = evaluate_restart_decision(&input).expect("restart");
        assert_eq!(decision.decision, "deny");
    }

    #[test]
    fn restart_without_profile_denies() {
        let input = RestartDecisionInput {
            entity_ref: content_ref_from_bytes(b"entity"),
            entity_kind: "service".to_string(),
            current_generation: 1,
            probe_results: vec![],
            prior_restart_attempts: 0,
            backoff_profile: None,
            authority_refs: vec![content_ref_from_bytes(b"auth")],
            resource_budget_refs: vec![],
        };
        let decision = evaluate_restart_decision(&input).expect("restart");
        assert_eq!(decision.decision, "deny");
    }
