
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
