//! Property evaluation regressions over recorded facts, with no scenario driver.

/// Facts under which every property of every scenario holds.
fn healthy() -> crate::vocabulary::ScenarioFacts {
    crate::vocabulary::ScenarioFacts {
        sibling_progressed: true,
        stale_delivery_acted: false,
        durable_work_readmitted: true,
        durable_work_lost: false,
        uncertain_effect_repeated: false,
        overload_denied: true,
        storm_terminated: true,
        stale_generation_acted: false,
        upgrade_completed: true,
        dependent_restarted_after_dependency: true,
        restarted_member_incarnation_advanced: true,
        saturating_work_bounded: true,
    }
}

// r[verify molten.testing.service_recovery_composition]
// r[verify molten.project.tier1_no_std_kernel_crates]
#[test]
fn healthy_facts_hold_every_property_and_empty_facts_prove_nothing() {
    for scenario in crate::vocabulary::FailureScenario::ALL {
        let outcomes = crate::judgment::evaluate_properties(scenario, &healthy());
        assert!(!outcomes.is_empty(), "{} asserts no property", scenario.as_str());
        assert!(outcomes.iter().all(|outcome| outcome.holds), "{}", scenario.as_str());
        let unproven = crate::judgment::evaluate_properties(scenario, &crate::vocabulary::ScenarioFacts::empty());
        assert_eq!(unproven.len(), outcomes.len());
        let proven_by_absence = unproven.iter().filter(|outcome| outcome.holds).count();
        // Only a pure absence check ("no stale delivery acted", "no effect was repeated") can hold
        // on an empty observation set; every other property needs a recorded positive fact.
        assert!(proven_by_absence <= 1, "{}", scenario.as_str());
    }
}

#[test]
fn each_violated_fact_fails_exactly_the_scenarios_that_depend_on_it() {
    let mut stale = healthy();
    stale.stale_delivery_acted = true;
    let outcomes = crate::judgment::evaluate_properties(
        crate::vocabulary::FailureScenario::DelayedPreRestartEventDelivery,
        &stale,
    );
    assert!(
        outcomes
            .iter()
            .any(|outcome| outcome.property == crate::judgment::ServiceProperty::StaleInstancesLoseInfluence
                && !outcome.holds)
    );
    assert!(
        crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::QueueOverload, &stale)
            .iter()
            .all(|outcome| outcome.holds)
    );

    let mut storm = healthy();
    storm.storm_terminated = false;
    assert!(
        !crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::RestartStormToEscalation, &storm)[0]
            .holds
    );

    let mut lost = healthy();
    lost.durable_work_lost = true;
    assert!(
        !crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::DurableWorkReadmission, &lost)[0]
            .holds
    );
    assert!(
        !crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::UncertainEffectSurfacing, &lost)[0]
            .holds
    );

    let mut starved = healthy();
    starved.saturating_work_bounded = false;
    assert!(
        !crate::judgment::evaluate_properties(
            crate::vocabulary::FailureScenario::CpuSaturationUnderTimeSlicing,
            &starved
        )[0]
        .holds
    );
    assert!(
        crate::judgment::evaluate_properties(
            crate::vocabulary::FailureScenario::MemberFailureWithDependentRestart,
            &starved
        )[0]
        .holds
    );

    let mut skipped = healthy();
    skipped.upgrade_completed = false;
    let upgrade = crate::judgment::evaluate_properties(
        crate::vocabulary::FailureScenario::GenerationUpgradeWithStaleWorkRejection,
        &skipped,
    );
    assert_eq!(
        upgrade.iter().map(|outcome| (outcome.property, outcome.holds)).collect::<alloc::vec::Vec<_>>(),
        alloc::vec![
            (crate::judgment::ServiceProperty::StaleInstancesLoseInfluence, false),
            (crate::judgment::ServiceProperty::DurableWorkIsReadmitted, true),
            (crate::judgment::ServiceProperty::UnaffectedServicesContinue, true),
        ]
    );
}

#[test]
fn vocabulary_names_are_distinct() {
    let scenarios = crate::vocabulary::FailureScenario::ALL
        .iter()
        .map(|scenario| scenario.as_str())
        .collect::<alloc::collections::BTreeSet<_>>();
    assert_eq!(scenarios.len(), crate::vocabulary::FailureScenario::ALL.len());
    let variants = [
        crate::vocabulary::ScenarioVariant::Correct,
        crate::vocabulary::ScenarioVariant::FencingRemoved,
        crate::vocabulary::ScenarioVariant::GroupBudgetReset,
        crate::vocabulary::ScenarioVariant::SlicingDisabled,
        crate::vocabulary::ScenarioVariant::MigrationSkipped,
    ]
    .map(crate::vocabulary::ScenarioVariant::as_str);
    assert_eq!(variants.iter().collect::<alloc::collections::BTreeSet<_>>().len(), variants.len());
    let roles = [
        crate::judgment::ServiceRole::Stateful,
        crate::judgment::ServiceRole::DependentWorker,
        crate::judgment::ServiceRole::Sibling,
    ]
    .map(crate::judgment::ServiceRole::as_str);
    assert_eq!(roles.iter().collect::<alloc::collections::BTreeSet<_>>().len(), roles.len());
}

#[hegel::test(test_cases = 64)]
fn hegel_queue_overload_holds_only_when_the_sibling_progressed_and_overload_was_denied(tc: hegel::TestCase) {
    let mut facts = healthy();
    facts.sibling_progressed = tc.draw(hegel::generators::booleans());
    facts.overload_denied = tc.draw(hegel::generators::booleans());
    facts.saturating_work_bounded = tc.draw(hegel::generators::booleans());
    let overload = crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::QueueOverload, &facts);
    assert_eq!(overload[0].holds, facts.sibling_progressed && facts.overload_denied);
    let saturation =
        crate::judgment::evaluate_properties(crate::vocabulary::FailureScenario::CpuSaturationUnderTimeSlicing, &facts);
    assert_eq!(saturation[0].holds, facts.sibling_progressed && facts.saturating_work_bounded);
}
