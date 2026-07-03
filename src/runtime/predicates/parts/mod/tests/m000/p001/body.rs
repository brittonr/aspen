
    #[test]
    fn service_dependencies_predicate_enforces_readiness_restart_and_shutdown_order() {
        let service = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("frontend-service"))
            .expect("service ref");
        let database = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("database-service"))
            .expect("database ref");
        let cache =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("cache-service")).expect("cache ref");
        let reverse =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("reverse-dependent-service"))
                .expect("reverse ref");
        let pass_state = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: sorted_refs(vec![database.clone(), cache.clone()]),
            ready_service_refs: sorted_refs(vec![service.clone(), database.clone(), cache.clone()]),
            failed_service_refs: Vec::new(),
            force_run_refs: Vec::new(),
            restart_refs: Vec::new(),
            reverse_dependency_refs: vec![reverse.clone()],
            shutdown_refs: sorted_refs(vec![service.clone(), reverse.clone()]),
        };
        let pass = evaluate_service_dependencies(&pass_state).expect("service dependencies predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let stale_restart = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("stale-restart"))
            .expect("stale restart ref");
        let denied_state = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: sorted_refs(vec![database.clone(), cache.clone()]),
            ready_service_refs: vec![service.clone()],
            failed_service_refs: vec![database],
            force_run_refs: Vec::new(),
            restart_refs: vec![stale_restart],
            reverse_dependency_refs: vec![reverse],
            shutdown_refs: vec![service],
        };
        let denied = evaluate_service_dependencies(&denied_state).expect("denied service dependencies predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "service-dependencies-not-ready"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "service-failed-dependency-without-admission")
        );
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "service-restart-without-failure"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "service-shutdown-before-reverse-dependencies")
        );
    }

    #[test]
    fn snapshot_authority_predicate_requires_admitted_claims_and_redaction_coverage() {
        let snapshot_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("snapshot")).expect("snapshot ref");
        let admitted = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("admitted-authority"))
            .expect("admitted authority");
        let readable = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("readable-assertion"))
            .expect("readable assertion");
        let redacted = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("redacted-assertion"))
            .expect("redacted assertion");
        let pass_state = RuntimeSnapshotAuthorityState {
            snapshot_ref: snapshot_ref.clone(),
            admitted_authority_refs: vec![readable.clone()],
            claimed_authority_refs: vec![readable.clone()],
            requested_assertion_refs: sorted_refs(vec![readable.clone(), redacted.clone()]),
            readable_assertion_refs: vec![readable.clone()],
            redacted_assertion_refs: vec![redacted.clone()],
        };
        let pass = evaluate_snapshot_authority(&pass_state).expect("snapshot authority predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let unadmitted = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("unadmitted-authority"))
            .expect("unadmitted authority");
        let uncovered = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("uncovered-assertion"))
            .expect("uncovered assertion");
        let denied_state = RuntimeSnapshotAuthorityState {
            snapshot_ref: "not-a-ref".to_string(),
            admitted_authority_refs: vec![admitted],
            claimed_authority_refs: vec![unadmitted],
            requested_assertion_refs: sorted_refs(vec![readable.clone(), uncovered]),
            readable_assertion_refs: vec![readable.clone()],
            redacted_assertion_refs: vec![readable],
        };
        let denied = evaluate_snapshot_authority(&denied_state).expect("denied snapshot authority predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "snapshot-ref-noncanonical"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "snapshot-claimed-authority-not-admitted")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "snapshot-readable-assertion-not-authorized")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "snapshot-assertion-readable-and-redacted")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "snapshot-requested-assertion-uncovered")
        );
    }

    #[test]
    fn near_far_refs_predicate_denies_dead_far_sync_and_cross_vat_near_refs() {
        // r[verify molten.vat_ref_state_proof.reference_lifetime]
        let reference_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string("object-ref")).expect("reference ref");
        let sync_near = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-a".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let pass = evaluate_near_far_refs(&sync_near).expect("near/far predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let far_sync = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Far,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let far_denied = evaluate_near_far_refs(&far_sync).expect("far sync predicate");
        assert!(!far_denied.is_allowed);
        assert!(
            far_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "far-ref-synchronous-call-denied")
        );

        let cross_vat_near = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Asynchronous,
        };
        let near_denied = evaluate_near_far_refs(&cross_vat_near).expect("cross-vat near predicate");
        assert!(!near_denied.is_allowed);
        assert!(near_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "near-ref-cross-vat"));

        let dead_ref = RuntimeNearFarRefState {
            reference_ref: "not-a-ref".to_string(),
            reference_kind: RuntimeReferenceKind::Far,
            is_live: false,
            caller_vat_id: String::new(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Asynchronous,
        };
        let dead_denied = evaluate_near_far_refs(&dead_ref).expect("dead ref predicate");
        assert!(!dead_denied.is_allowed);
        assert!(dead_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "reference-not-live"));
        assert!(dead_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "reference-ref-noncanonical"));
    }

    #[test]
    fn distributed_ref_lifetime_requires_live_session_or_admitted_handoff() {
        // r[verify molten.vat_ref_state_proof.reference_lifetime]
        let far_ref = deterministic_ref("distributed-far-ref");
        let replacement_ref = deterministic_ref("distributed-replacement-ref");
        let session_ref = deterministic_ref("distributed-session-ref");
        let pending_call_ref = deterministic_ref("distributed-pending-call-ref");

        let live = RuntimeDistributedRefLifetimeState {
            far_ref: far_ref.clone(),
            session_ref: session_ref.clone(),
            replacement_ref: None,
            is_session_live: true,
            is_handoff_admitted: false,
            pending_call_refs: Vec::new(),
            failed_pending_call_refs: Vec::new(),
            attempted_use_refs: vec![far_ref.clone()],
        };
        let live_result = evaluate_distributed_ref_lifetime(&live).expect("live distributed ref");
        assert!(live_result.is_allowed);
        assert_eq!(live_result.receipt.decision, PredicateDecision::Pass);

        let handoff = RuntimeDistributedRefLifetimeState {
            far_ref: far_ref.clone(),
            session_ref: session_ref.clone(),
            replacement_ref: Some(replacement_ref.clone()),
            is_session_live: false,
            is_handoff_admitted: true,
            pending_call_refs: Vec::new(),
            failed_pending_call_refs: Vec::new(),
            attempted_use_refs: vec![replacement_ref],
        };
        let handoff_result = evaluate_distributed_ref_lifetime(&handoff).expect("handoff distributed ref");
        assert!(handoff_result.is_allowed);

        let stale = RuntimeDistributedRefLifetimeState {
            far_ref: far_ref.clone(),
            session_ref: session_ref.clone(),
            replacement_ref: None,
            is_session_live: false,
            is_handoff_admitted: false,
            pending_call_refs: Vec::new(),
            failed_pending_call_refs: Vec::new(),
            attempted_use_refs: vec![far_ref.clone()],
        };
        let stale_result = evaluate_distributed_ref_lifetime(&stale).expect("stale distributed ref");
        assert!(!stale_result.is_allowed);
        assert!(
            stale_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "distributed-ref-stale-descriptor-used")
        );

        let pending_open = RuntimeDistributedRefLifetimeState {
            far_ref,
            session_ref,
            replacement_ref: None,
            is_session_live: false,
            is_handoff_admitted: false,
            pending_call_refs: vec![pending_call_ref],
            failed_pending_call_refs: Vec::new(),
            attempted_use_refs: Vec::new(),
        };
        let pending_open_result = evaluate_distributed_ref_lifetime(&pending_open).expect("pending open distributed ref");
        assert!(!pending_open_result.is_allowed);
        assert!(
            pending_open_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "distributed-ref-disconnected-pending-calls-not-failed")
        );
    }

    #[test]
    fn actormap_transaction_predicate_commits_rolls_back_and_invalidates_removed_objects() {
        let existing = deterministic_ref("existing-object");
        let spawned = deterministic_ref("spawned-object");
        let removed = deterministic_ref("removed-object");
        let committed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            used_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
        };
        let pass = evaluate_actormap_transaction(&committed).expect("actormap transaction predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        crate::preserves_rail::validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let stale_removed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), removed.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            used_object_refs: vec![removed.clone()],
        };
        let denied = evaluate_actormap_transaction(&stale_removed).expect("denied actormap predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "actormap-commit-delta-mismatch"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-present-after-commit")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-used-after-removal")
        );

        let rollback = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::RolledBack,
            before_object_refs: vec![existing],
            after_object_refs: vec![spawned.clone()],
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: Vec::new(),
            visible_object_refs: vec![spawned.clone()],
            used_object_refs: vec![spawned],
        };
        let rollback_denied = evaluate_actormap_transaction(&rollback).expect("rollback actormap predicate");
        assert!(!rollback_denied.is_allowed);
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "actormap-rollback-state-changed")
        );
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "spawned-object-visible-after-rollback")
        );
    }

    #[test]
    fn rights_amplification_predicate_denies_unsealed_authority_recovery() {
        // r[verify molten.vat_ref_state_proof.rollback_cleanup]
        let holder_ref = deterministic_ref("rights-holder-object");
        let sealed_value_ref = deterministic_ref("rights-sealed-value");
        let brand_ref = deterministic_ref("rights-brand");
        let wrong_brand_ref = deterministic_ref("rights-wrong-brand");
        let sealed_authority_ref = deterministic_ref("rights-sealed-authority");
        let unsealed_authority_ref = deterministic_ref("rights-unsealed-authority");

        let admitted = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref.clone(),
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: brand_ref.clone(),
            sealed_authority_refs: vec![sealed_authority_ref.clone()],
            recovered_authority_refs: vec![sealed_authority_ref.clone()],
        };
        let admitted_result = evaluate_rights_amplification(&admitted).expect("admitted rights amplification");
        assert!(admitted_result.is_allowed);
        assert_eq!(admitted_result.receipt.decision, PredicateDecision::Pass);

        let extra_recovery = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref.clone(),
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: brand_ref.clone(),
            sealed_authority_refs: vec![sealed_authority_ref.clone()],
            recovered_authority_refs: sorted_refs(vec![sealed_authority_ref, unsealed_authority_ref]),
        };
        let extra_result = evaluate_rights_amplification(&extra_recovery).expect("extra rights amplification");
        assert!(!extra_result.is_allowed);
        assert!(
            extra_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "rights-amplification-recovered-authority-not-sealed")
        );

        let wrong_brand = RuntimeRightsAmplificationState {
            holder_object_ref: holder_ref,
            sealed_value_ref,
            sealer_brand_ref: brand_ref,
            unsealer_brand_ref: wrong_brand_ref,
            sealed_authority_refs: vec![deterministic_ref("rights-sealed-authority")],
            recovered_authority_refs: vec![deterministic_ref("rights-sealed-authority")],
        };
        let wrong_brand_result = evaluate_rights_amplification(&wrong_brand).expect("wrong brand amplification");
        assert!(!wrong_brand_result.is_allowed);
        assert!(
            wrong_brand_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "rights-amplification-brand-mismatch")
        );
    }

    #[test]
    fn vat_rollback_cleanup_binds_snapshot_and_removes_dependent_state() {
        // r[verify molten.vat_ref_state_proof.rollback_cleanup]
        let staged_value = RuntimeValue::string("rollback-staged-assertion").expect("runtime value");
        let actor = "rollback-owner".to_string();
        let staged_assertion = crate::runtime::RuntimeAssertion {
            actor: actor.clone(),
            value: staged_value.clone(),
        };
        let staged_assertion_ref = staged_assertion.assertion_ref().expect("staged assertion ref");
        let staged_observer_ref = deterministic_ref("rollback-staged-observer");
        let staged_pending_call_ref = deterministic_ref("rollback-staged-pending-call");
        let staged_authority_snapshot_ref = deterministic_ref("rollback-staged-authority-snapshot");
        let mut state = RuntimeState::new(TURN_COMMIT_TEST_SEED);
        let before = state.snapshot();
        let before_ref = before.snapshot_ref().expect("before snapshot ref");
        let step = RuntimeStep::Assert {
            actor,
            value: staged_value,
        };
        let turn = state.begin_turn(&step);
        let (_events, rollback_receipt) = state
            .rollback_turn_with_predicate_receipt(turn, step.primary_actor(), "policy denied")
            .expect("rollback receipt");
        let final_snapshot = state.snapshot();
        assert_eq!(final_snapshot, before);
        assert!(
            !final_snapshot
                .assertions
                .iter()
                .any(|assertion| assertion.assertion_ref().expect("assertion ref") == staged_assertion_ref)
        );

        let cleaned = RuntimeVatRollbackCleanupState {
            rollback_receipt_ref: rollback_receipt.receipt_ref.clone(),
            before_snapshot_ref: before_ref.clone(),
            final_snapshot_ref: before_ref.clone(),
            rolled_back_refs: sorted_refs(vec![
                staged_assertion_ref.clone(),
                staged_observer_ref.clone(),
                staged_pending_call_ref.clone(),
                staged_authority_snapshot_ref.clone(),
            ]),
            remaining_assertion_refs: Vec::new(),
            remaining_observer_refs: Vec::new(),
            remaining_pending_call_refs: Vec::new(),
            remaining_authority_snapshot_refs: Vec::new(),
        };
        let cleaned_result = evaluate_vat_rollback_cleanup(&cleaned).expect("rollback cleanup");
        assert!(cleaned_result.is_allowed);
        assert_eq!(cleaned_result.receipt.decision, PredicateDecision::Pass);

        let leaked = RuntimeVatRollbackCleanupState {
            rollback_receipt_ref: rollback_receipt.receipt_ref,
            before_snapshot_ref: before_ref,
            final_snapshot_ref: deterministic_ref("rollback-mutated-final-snapshot"),
            rolled_back_refs: sorted_refs(vec![
                staged_assertion_ref.clone(),
                staged_observer_ref.clone(),
                staged_pending_call_ref.clone(),
                staged_authority_snapshot_ref.clone(),
            ]),
            remaining_assertion_refs: vec![staged_assertion_ref],
            remaining_observer_refs: vec![staged_observer_ref],
            remaining_pending_call_refs: vec![staged_pending_call_ref],
            remaining_authority_snapshot_refs: vec![staged_authority_snapshot_ref],
        };
        let leaked_result = evaluate_vat_rollback_cleanup(&leaked).expect("leaked rollback cleanup");
        assert!(!leaked_result.is_allowed);
        assert!(
            leaked_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vat-rollback-final-snapshot-changed")
        );
        assert!(
            leaked_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vat-rollback-assertion-leak")
        );
        assert!(
            leaked_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vat-rollback-observer-leak")
        );
        assert!(
            leaked_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vat-rollback-pending-call-leak")
        );
        assert!(
            leaked_result
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "vat-rollback-authority-snapshot-leak")
        );
    }
