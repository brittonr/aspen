    const DEMAND_SERVICE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OTHER_SERVICE_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DEMANDER_REF: &str = "scope:demander-a";
    const OTHER_DEMANDER_REF: &str = "scope:demander-b";
    const OPERATION_REF: &str = "operation:demand-a";

    fn demand(service_ref: &str, kind: ServiceDemandKind, demander_ref: &str) -> ServiceDemandAssertion {
        ServiceDemandAssertion {
            service_ref: service_ref.to_string(),
            kind,
            demander_ref: demander_ref.to_string(),
            operation_identity: OPERATION_REF.to_string(),
        }
    }

    fn assert_demand(state: &mut RuntimeState, demand: &ServiceDemandAssertion) {
        state.apply_step(&Step::Assert {
            actor: demand.demander_ref.clone(),
            value: demand.to_assertion_value().expect("the demand assertion value"),
        });
    }

    fn retract_demand(state: &mut RuntimeState, demand: &ServiceDemandAssertion) {
        state.apply_step(&Step::Retract {
            actor: demand.demander_ref.clone(),
            value: demand.to_assertion_value().expect("the demand assertion value"),
        });
    }

    fn live_assertions(state: &RuntimeState) -> Vec<Assertion> {
        state.assertions.iter().cloned().collect()
    }

    #[test]
    fn the_demand_record_round_trips_through_the_canonical_value() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let demand = demand(DEMAND_SERVICE_REF, ServiceDemandKind::ForceRun, DEMANDER_REF);
        let decoded = ServiceDemandAssertion::from_value(&demand.to_value()).expect("the demand record decodes");
        assert_eq!(decoded, demand);
        assert_eq!(decoded.kind.as_str(), "force-run");
        assert_eq!(ServiceDemandKind::parse("dependency-gated"), Some(ServiceDemandKind::DependencyGated));
        assert_eq!(ServiceDemandKind::parse("unknown"), None);
    }

    #[test]
    fn one_owner_cannot_double_count_demand() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(11);
        let first = demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, DEMANDER_REF);
        let second = demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, DEMANDER_REF);
        assert_demand(&mut state, &first);
        assert_demand(&mut state, &second);
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert_eq!(demands.live_demand_count(DEMAND_SERVICE_REF), 1);
        assert_eq!(demands.demanders_for(DEMAND_SERVICE_REF), vec![DEMANDER_REF.to_string()]);
    }

    #[test]
    fn another_demander_keeps_the_service_alive() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(13);
        let first = demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, DEMANDER_REF);
        let second = demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, OTHER_DEMANDER_REF);
        assert_demand(&mut state, &first);
        assert_demand(&mut state, &second);
        retract_demand(&mut state, &first);
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert_eq!(demands.live_demand_count(DEMAND_SERVICE_REF), 1);
        assert_eq!(demands.demanders_for(DEMAND_SERVICE_REF), vec![OTHER_DEMANDER_REF.to_string()]);
        assert!(!demands.is_shutdown_eligible(DEMAND_SERVICE_REF));
    }

    #[test]
    fn withdrawing_the_last_demand_makes_shutdown_eligible() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(17);
        let demand = demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, DEMANDER_REF);
        assert_demand(&mut state, &demand);
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert!(!demands.is_shutdown_eligible(DEMAND_SERVICE_REF));
        retract_demand(&mut state, &demand);
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert_eq!(demands.live_demand_count(DEMAND_SERVICE_REF), 0);
        assert!(demands.is_shutdown_eligible(DEMAND_SERVICE_REF));
    }

    #[test]
    fn force_run_demand_is_explicit_in_the_derived_facts() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(19);
        assert_demand(
            &mut state,
            &demand(OTHER_SERVICE_REF, ServiceDemandKind::ForceRun, DEMANDER_REF),
        );
        assert_demand(
            &mut state,
            &demand(DEMAND_SERVICE_REF, ServiceDemandKind::DependencyGated, OTHER_DEMANDER_REF),
        );
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert_eq!(demands.demanded_service_refs(), vec![
            DEMAND_SERVICE_REF.to_string(),
            OTHER_SERVICE_REF.to_string(),
        ]);
        assert_eq!(demands.force_run_service_refs(), vec![OTHER_SERVICE_REF.to_string()]);
    }

    #[test]
    fn a_malformed_demand_record_is_a_typed_failure() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(23);
        let malformed = Value::new(crate::preserves_rail::record("runtime-service-demand-v1", vec![
            crate::preserves_rail::string("runtime-service-demand.v1"),
        ]))
        .expect("the malformed demand value");
        state.apply_step(&Step::Assert {
            actor: DEMANDER_REF.to_string(),
            value: malformed,
        });
        let diagnostics = ServiceDemandSet::derive(&live_assertions(&state)).expect_err("a malformed demand denies");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn a_restart_message_creates_no_demand_state() {
        // r[verify molten.runtime_spine.demand_assertion_lifetime]
        let mut state = RuntimeState::new(29);
        state.apply_step(&Step::Send {
            from: DEMANDER_REF.to_string(),
            to: DEMAND_SERVICE_REF.to_string(),
            body: Value::string("restart").expect("the restart body"),
        });
        let demands = ServiceDemandSet::derive(&live_assertions(&state)).expect("the demand set derives");
        assert!(demands.demanded_service_refs().is_empty());
        assert!(demands.is_shutdown_eligible(DEMAND_SERVICE_REF));
    }
