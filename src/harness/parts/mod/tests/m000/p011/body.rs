    use crate::harness::composition::*;

    const SCENARIOS: [FailureScenario; 8] = [
        FailureScenario::MemberFailureWithDependentRestart,
        FailureScenario::DelayedPreRestartEventDelivery,
        FailureScenario::DurableWorkReadmission,
        FailureScenario::UncertainEffectSurfacing,
        FailureScenario::QueueOverload,
        FailureScenario::RestartStormToEscalation,
        FailureScenario::GenerationUpgradeWithStaleWorkRejection,
        FailureScenario::CpuSaturationUnderTimeSlicing,
    ];

    #[test]
    fn the_definitions_bind_three_services_and_one_recovery_group() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        assert_eq!(definitions.group.members.len(), 2);
        let stateful = &definitions.group.members[0];
        let worker = &definitions.group.members[1];
        assert!(stateful.after.is_empty());
        assert_eq!(worker.after, vec![stateful.member_id.clone()]);
        assert_eq!(
            definitions.stateful_facade.profile,
            molten_core::system_extension::FacadeProfile::StateMachine
        );
        assert_eq!(
            definitions.worker_facade.profile,
            molten_core::system_extension::FacadeProfile::Service
        );
        assert_eq!(
            definitions.sibling_facade.profile,
            molten_core::system_extension::FacadeProfile::Service
        );
        assert_ne!(definitions.worker_facade.service_id, definitions.sibling_facade.service_id);
    }

    #[test]
    fn the_correct_composition_holds_every_property_in_every_scenario() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        for scenario in SCENARIOS {
            let facts = run_scenario(&definitions, scenario, ScenarioVariant::Correct).expect("the scenario runs");
            let outcomes = evaluate_properties(scenario, &facts);
            assert!(!outcomes.is_empty(), "{} asserted no property", scenario.as_str());
            for outcome in &outcomes {
                assert!(
                    outcome.holds,
                    "{} violated {}: {}",
                    scenario.as_str(),
                    outcome.property.as_str(),
                    outcome.detail
                );
            }
        }
    }

    #[test]
    fn the_broken_fencing_variant_violates_the_stale_instance_property() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        let scenario = FailureScenario::DelayedPreRestartEventDelivery;
        let facts = run_scenario(&definitions, scenario, ScenarioVariant::FencingRemoved).expect("the scenario runs");
        assert!(facts.stale_delivery_acted);
        let outcomes = evaluate_properties(scenario, &facts);
        let violated = outcomes
            .iter()
            .find(|outcome| outcome.property == ServiceProperty::StaleInstancesLoseInfluence);
        assert!(
            violated.is_some_and(|outcome| !outcome.holds),
            "the broken fencing variant must fail the stale-instance property"
        );
    }

    #[test]
    fn the_broken_budget_variant_violates_the_storm_property() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        let scenario = FailureScenario::RestartStormToEscalation;
        let facts = run_scenario(&definitions, scenario, ScenarioVariant::GroupBudgetReset).expect("the scenario runs");
        assert!(!facts.storm_terminated);
        let outcomes = evaluate_properties(scenario, &facts);
        let violated = outcomes
            .iter()
            .find(|outcome| outcome.property == ServiceProperty::StormsTerminateWithinWindow);
        assert!(
            violated.is_some_and(|outcome| !outcome.holds),
            "the broken budget variant must fail the storm property"
        );
    }

    // r[verify molten.testing.service_recovery_composition]
    #[test]
    fn disabled_slicing_starves_the_sibling_and_violates_the_unaffected_services_property() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        let scenario = FailureScenario::CpuSaturationUnderTimeSlicing;
        let correct = run_scenario(&definitions, scenario, ScenarioVariant::Correct).expect("the scenario runs");
        assert!(correct.sibling_progressed);
        assert!(correct.saturating_work_bounded);
        let broken = run_scenario(&definitions, scenario, ScenarioVariant::SlicingDisabled).expect("the scenario runs");
        assert!(!broken.sibling_progressed, "without slicing the sibling waits for the saturating callback");
        assert!(broken.saturating_work_bounded);
        let outcomes = evaluate_properties(scenario, &broken);
        assert!(
            outcomes
                .iter()
                .any(|outcome| outcome.property == ServiceProperty::UnaffectedServicesContinue && !outcome.holds),
            "the disabled-slicing variant must fail the unaffected-services property"
        );
    }

    #[test]
    fn the_recovery_group_orders_the_dependent_worker_after_the_stateful_service() {
        let definitions = group_definitions().expect("the composition definitions are admitted");
        let scenario = FailureScenario::MemberFailureWithDependentRestart;
        let facts = run_scenario(&definitions, scenario, ScenarioVariant::Correct).expect("the scenario runs");
        assert!(facts.sibling_progressed);
        assert!(facts.dependent_restarted_after_dependency);
        assert!(facts.restarted_member_incarnation_advanced);
        let outcomes = evaluate_properties(scenario, &facts);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].property, ServiceProperty::UnaffectedServicesContinue);
    }
