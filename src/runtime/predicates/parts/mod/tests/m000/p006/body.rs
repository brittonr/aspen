    const SERVICE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DEPENDENCY_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DEMANDER_REF: &str = "scope:demander-a";
    const OTHER_DEMANDER_REF: &str = "scope:demander-b";

    fn demand(service_ref: &str, kind: crate::runtime::dataspace::ServiceDemandKind, demander_ref: &str) -> crate::runtime::dataspace::ServiceDemandAssertion {
        crate::runtime::dataspace::ServiceDemandAssertion {
            service_ref: service_ref.to_string(),
            kind,
            demander_ref: demander_ref.to_string(),
            operation_identity: "operation:demand-a".to_string(),
        }
    }

    fn demand_set(demands: &[crate::runtime::dataspace::ServiceDemandAssertion]) -> crate::runtime::dataspace::ServiceDemandSet {
        let assertions: Vec<crate::runtime::RuntimeAssertion> = demands
            .iter()
            .map(|demand| crate::runtime::RuntimeAssertion {
                actor: demand.demander_ref.clone(),
                value: demand.to_assertion_value().expect("the demand assertion value"),
                facet: None,
            })
            .collect();
        crate::runtime::dataspace::ServiceDemandSet::derive(&assertions).expect("the demand set derives")
    }

    fn dependency_state(demands: &crate::runtime::dataspace::ServiceDemandSet) -> RuntimeServiceDependenciesState {
        RuntimeServiceDependenciesState {
            service_ref: SERVICE_REF.to_string(),
            demanded_service_refs: Vec::new(),
            dependency_refs: vec![DEPENDENCY_REF.to_string()],
            ready_service_refs: Vec::new(),
            failed_service_refs: Vec::new(),
            force_run_refs: Vec::new(),
            restart_refs: Vec::new(),
            reverse_dependency_refs: Vec::new(),
            shutdown_refs: Vec::new(),
            live_demand_demanders: Vec::new(),
        }
        .with_demand_assertions(demands)
    }

    #[test]
    fn dependency_gated_demand_waits_for_the_declared_dependencies() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let demands = demand_set(&[demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::DependencyGated, DEMANDER_REF)]);
        let waiting = dependency_state(&demands);
        assert_eq!(waiting.demanded_service_refs, vec![SERVICE_REF.to_string()]);
        assert!(waiting.force_run_refs.is_empty());
        let denied = evaluate_service_dependencies(&waiting).expect("service dependencies");
        assert!(!denied.is_allowed, "diagnostics: {:?}", denied.receipt.diagnostics);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "service-dependencies-not-ready"),
            "diagnostics: {:?}",
            denied.receipt.diagnostics
        );

        let ready = RuntimeServiceDependenciesState {
            ready_service_refs: vec![DEPENDENCY_REF.to_string()],
            ..waiting
        };
        let admitted = evaluate_service_dependencies(&ready).expect("service dependencies");
        assert!(admitted.is_allowed);
    }

    #[test]
    fn force_run_demand_bypasses_the_ordering_by_declaration() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let demands = demand_set(&[demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::ForceRun, DEMANDER_REF)]);
        let state = dependency_state(&demands);
        assert_eq!(state.force_run_refs, vec![SERVICE_REF.to_string()]);
        let admitted = evaluate_service_dependencies(&state).expect("service dependencies");
        assert!(admitted.is_allowed, "diagnostics: {:?}", admitted.receipt.diagnostics);
        assert!(
            admitted
                .receipt
                .checks
                .iter()
                .any(|check| check == "demand-dependencies-ready"),
            "checks: {:?}",
            admitted.receipt.checks
        );
    }

    #[test]
    fn withdrawing_the_only_demand_allows_shutdown_and_another_demander_prevents_it() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let demanded = demand_set(&[demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::DependencyGated, DEMANDER_REF)]);
        let shutdown = RuntimeServiceDependenciesState {
            shutdown_refs: vec![SERVICE_REF.to_string()],
            ..dependency_state(&demanded)
        };
        let denied = evaluate_service_dependencies(&shutdown).expect("service dependencies");
        assert!(!denied.is_allowed);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "service-shutdown-with-live-demand")
        );

        let two_demanders = demand_set(&[
            demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::DependencyGated, DEMANDER_REF),
            demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::DependencyGated, OTHER_DEMANDER_REF),
        ]);
        let still_demanded = RuntimeServiceDependenciesState {
            shutdown_refs: vec![SERVICE_REF.to_string()],
            ..dependency_state(&two_demanders)
        };
        let denied = evaluate_service_dependencies(&still_demanded).expect("service dependencies");
        assert!(!denied.is_allowed);

        let withdrawn = demand_set(&[]);
        let eligible = RuntimeServiceDependenciesState {
            shutdown_refs: vec![SERVICE_REF.to_string()],
            ..dependency_state(&withdrawn)
        };
        let admitted = evaluate_service_dependencies(&eligible).expect("service dependencies");
        assert!(admitted.is_allowed);
    }

    #[test]
    fn an_unverified_provider_never_satisfies_demand() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let demands = demand_set(&[demand(SERVICE_REF, crate::runtime::dataspace::ServiceDemandKind::DependencyGated, DEMANDER_REF)]);
        let state = dependency_state(&demands);
        assert!(state.ready_service_refs.is_empty());
        let denied = evaluate_service_dependencies(&state).expect("service dependencies");
        assert!(!denied.is_allowed);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "service-dependencies-not-ready")
        );
    }
