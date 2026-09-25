
    #[hegel::test(test_cases = 16)]
    fn hegel_service_dependency_and_reference_predicates_fail_closed(tc: TestCase) {
        let salt = draw_property_salt(&tc);
        let dependency_count = draw_property_collection_len(&tc);
        let ready_count = draw_property_collection_len(&tc);
        let service = property_ref("service", salt, 0);
        let dependencies = property_refs("dependency", salt, dependency_count);
        let mut ready =
            dependencies.iter().take(std::cmp::min(dependency_count, ready_count)).cloned().collect::<Vec<_>>();
        ready.push(service.clone());
        ready.sort();
        let service_state = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: dependencies.clone(),
            ready_service_refs: ready,
            failed_service_refs: Vec::new(),
            force_run_refs: Vec::new(),
            restart_refs: Vec::new(),
            reverse_dependency_refs: Vec::new(),
            shutdown_refs: Vec::new(),
        };
        let service_result = evaluate_service_dependencies(&service_state).expect("service dependencies");
        let is_dependencies_ready = ready_count >= dependency_count;
        assert_eq!(service_result.is_allowed, is_dependencies_ready);
        if !is_dependencies_ready {
            assert!(
                service_result
                    .receipt
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic == "service-dependencies-not-ready")
            );
        }

        let failed_dependency = dependencies.first().cloned().unwrap_or_else(|| property_ref("dependency", salt, 99));
        let admitted_failure = RuntimeServiceDependenciesState {
            service_ref: service.clone(),
            demanded_service_refs: vec![service.clone()],
            dependency_refs: vec![failed_dependency.clone()],
            ready_service_refs: vec![service.clone()],
            failed_service_refs: vec![failed_dependency.clone()],
            force_run_refs: vec![service.clone()],
            restart_refs: vec![failed_dependency],
            reverse_dependency_refs: Vec::new(),
            shutdown_refs: Vec::new(),
        };
        let force_run = evaluate_service_dependencies(&admitted_failure).expect("force-run dependency");
        assert!(force_run.is_allowed);

        let reference_ref = property_ref("reference", salt, 0);
        let near_sync = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-a".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        assert!(evaluate_near_far_refs(&near_sync).expect("near sync").is_allowed);
        let far_sync = RuntimeNearFarRefState {
            reference_ref,
            reference_kind: RuntimeReferenceKind::Far,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let denied = evaluate_near_far_refs(&far_sync).expect("far sync");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "far-ref-synchronous-call-denied"));
    }
