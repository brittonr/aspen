    use super::*;

    fn content_ref_from_bytes(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn to_text(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn failed_turn_rolls_back_pending_actions_and_records_discarded_refs() {
        let state = crate::runtime::RuntimeState::new(1);
        let step = crate::runtime::RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: crate::runtime::RuntimeValue::string("service.ready").expect("runtime value"),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let events = state.rollback_turn(turn.clone(), step.primary_actor(), "policy denied");
        let after = state.snapshot();
        let policy_ref = content_ref_from_bytes(b"policy");
        let receipt = super::turn_failure_receipt(&super::TurnFailureInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: super::TurnFailureKind::Denial,
            cause: "policy denied",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &[],
            policy_refs: &[policy_ref],
            evidence_refs: &[],
            logical_step: 3,
        })
        .expect("turn failure receipt");
        let rendered = to_text(&receipt.value).expect("render receipt");

        assert_eq!(receipt.decision, "pass");
        assert_eq!(before, after);
        assert!(matches!(events.as_slice(), [crate::runtime::RuntimeEvent::TurnRolledBack { .. }]));
        assert!(rendered.contains("lifecycle-turn-failure-v1"));
        assert!(rendered.contains("runtime-turn-action-assert-v1"));
    }

    #[test]
    fn failed_turn_receipt_denies_if_rollback_mutated_state() {
        let mut state = crate::runtime::RuntimeState::new(1);
        let step = crate::runtime::RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: crate::runtime::RuntimeValue::string("service.ready").expect("runtime value"),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        state.apply_step(&step);
        let after = state.snapshot();
        let receipt = super::turn_failure_receipt(&super::TurnFailureInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: super::TurnFailureKind::ValidationFailure,
            cause: "validation failed",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
            logical_step: 4,
        })
        .expect("turn failure receipt");

        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.diagnostics, vec!["rollback state differs from before state".to_owned()]);
    }

    #[test]
    fn scope_cleanup_retracts_owned_assertions_subscriptions_and_messages() {
        let mut state = crate::runtime::RuntimeState::new(1);
        let ready = crate::runtime::RuntimeValue::string("service.ready").expect("runtime value");
        state.apply_step(&crate::runtime::RuntimeStep::Observe {
            actor: "actor-1".to_owned(),
            pattern: ready.clone(),
        });
        state.apply_step(&crate::runtime::RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: ready.clone(),
        });
        state.apply_step(&crate::runtime::RuntimeStep::Send {
            from: "actor-1".to_owned(),
            to: "actor-2".to_owned(),
            body: ready,
        });
        let before = state.snapshot();
        let cleanup = state.cleanup_actor_scope("actor-1").expect("cleanup scope");
        let after = state.snapshot();
        let evidence_ref = content_ref_from_bytes(b"cleanup-evidence");
        let receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            cause: "stop",
            before: &before,
            after_cleanup: &after,
            cleanup: &cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[evidence_ref],
            logical_step: 5,
        })
        .expect("cleanup receipt");
        let rendered = to_text(&receipt.value).expect("render cleanup receipt");

        assert_eq!(receipt.decision, "pass");
        assert_ne!(before, after);
        assert_eq!(after.assertions.len(), 0);
        assert_eq!(after.observers.len(), 0);
        assert_eq!(after.messages.len(), 0);
        assert_eq!(cleanup.assertion_refs.len(), 1);
        assert_eq!(cleanup.observer_refs.len(), 1);
        assert_eq!(cleanup.message_refs.len(), 1);
        assert!(rendered.contains("lifecycle-scope-cleanup-v1"));
    }

    #[test]
    fn scope_cleanup_is_idempotent_and_receipt_backed() {
        let mut state = crate::runtime::RuntimeState::new(1);
        let ready = crate::runtime::RuntimeValue::string("service.ready").expect("runtime value");
        state.apply_step(&crate::runtime::RuntimeStep::Assert {
            actor: "actor-1".to_owned(),
            value: ready,
        });
        let before_first = state.snapshot();
        let first_cleanup = state.cleanup_actor_scope("actor-1").expect("first cleanup");
        let after_first = state.snapshot();
        let first_receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            cause: "cleanup",
            before: &before_first,
            after_cleanup: &after_first,
            cleanup: &first_cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
            logical_step: 6,
        })
        .expect("first cleanup receipt");

        let before_second = state.snapshot();
        let second_cleanup = state.cleanup_actor_scope("actor-1").expect("second cleanup");
        let after_second = state.snapshot();
        let second_receipt = super::scope_cleanup_receipt(&super::ScopeCleanupInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            cause: "cleanup",
            before: &before_second,
            after_cleanup: &after_second,
            cleanup: &second_cleanup,
            live_ref_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
            logical_step: 7,
        })
        .expect("second cleanup receipt");

        assert_eq!(first_receipt.decision, "pass");
        assert_eq!(second_receipt.decision, "pass");
        assert_ne!(before_first, after_first);
        assert_eq!(before_second, after_second);
        assert!(second_cleanup.assertion_refs.is_empty());
        assert!(second_cleanup.observer_refs.is_empty());
        assert!(second_cleanup.message_refs.is_empty());
    }

    #[test]
    fn failed_turn_discloses_one_shot_effects() {
        let state = crate::runtime::RuntimeState::new(1);
        let step = crate::runtime::RuntimeStep::Clock {
            actor: "actor-1".to_owned(),
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let after = state.snapshot();
        let effect_refs = vec![content_ref_from_bytes(b"irreversible-effect")];
        let receipt = super::turn_failure_receipt(&super::TurnFailureInput {
            entity_kind: super::EntityKind::Actor,
            entity_id: "actor-1",
            failure_kind: super::TurnFailureKind::Panic,
            cause: "panic after one-shot effect",
            before: &before,
            after_rollback: &after,
            pending_turn: &turn,
            vat_delta_refs: &[],
            one_shot_effect_refs: &effect_refs,
            policy_refs: &[],
            evidence_refs: &[],
            logical_step: 8,
        })
        .expect("turn failure receipt");
        let rendered = to_text(&receipt.value).expect("render receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(rendered.contains("one-shot-effects"));
        assert!(rendered.contains(&effect_refs[0]));
    }

    #[test]
    fn monitor_observes_failure_without_authority_escalation() {
        let policy_ref = content_ref_from_bytes(b"monitor-policy");
        let failure_ref = content_ref_from_bytes(b"child-failure");
        let receipt = super::monitor_receipt(&super::MonitorInput {
            observer_id: "monitor-1",
            child_id: "child-1",
            child_failure_ref: &failure_ref,
            policy_refs: &[policy_ref],
            evidence_refs: &[],
            logical_step: 9,
        })
        .expect("monitor receipt");
        let rendered = to_text(&receipt.value).expect("render monitor receipt");

        assert_eq!(receipt.decision, "pass");
        assert!(rendered.contains("authority-escalated #f"));
    }

    #[test]
    fn service_demand_waits_without_start_until_dependencies_are_ready() {
        let demand_ref = content_ref_from_bytes(b"service-demand");
        let manifest_ref = content_ref_from_bytes(b"service-manifest");
        let dependency_ref = content_ref_from_bytes(b"database-ready");
        let authority_ref = content_ref_from_bytes(b"service-authority");
        let resource_ref = content_ref_from_bytes(b"service-resource");
        let evidence_ref = content_ref_from_bytes(b"service-evidence");
        let wait = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
            service_id: "service:frontend",
            demand_ref: &demand_ref,
            manifest_ref: &manifest_ref,
            required_dependency_refs: std::slice::from_ref(&dependency_ref),
            ready_dependency_refs: &[],
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("dependency wait");
        assert_eq!(wait.decision, "wait");
        assert_eq!(wait.lifecycle_kind, "dependency-wait");
        assert!(!wait.start_side_effect_admitted);
        assert!(wait.readiness_assertion.is_none());

        let ready = super::evaluate_service_demand(&super::ServiceDemandEvaluationInput {
            service_id: "service:frontend",
            demand_ref: &demand_ref,
            manifest_ref: &manifest_ref,
            required_dependency_refs: std::slice::from_ref(&dependency_ref),
            ready_dependency_refs: std::slice::from_ref(&dependency_ref),
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("service ready demand");
        assert_eq!(ready.decision, "pass");
        assert_eq!(ready.lifecycle_kind, "start");
        assert!(ready.start_side_effect_admitted);
        assert!(ready.readiness_assertion.is_some());
    }
