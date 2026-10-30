//! External-cohort live twin. These tests never construct a supervisor in the test process.
//! Explicit invocation without actual signed artifacts is a failure, not a simulated pass.

#[path = "recovery/authority.rs"]
mod authority;
#[path = "recovery/operations.rs"]
mod operations;
#[path = "../outside_in/shell/owned.rs"]
mod owned;
#[path = "recovery/processes.rs"]
mod processes;
#[path = "recovery/scenarios.rs"]
mod scenarios;
#[path = "recovery/twin.rs"]
mod twin;
#[path = "recovery/wire.rs"]
mod wire;

use crate::support::*;

// r[verify molten.testing.service_recovery_composition]
// r[verify molten.system_extension.node_hosting]
#[test]
#[ignore = "requires independently signed real Nix node and actual admitted native callback cohort"]
fn live_public_node_composition() -> TestResult<()> {
    let trust = std::env::var_os("MOLTEN_TEST_SOURCE_TRUST").or_fail("actual independent signed-source trust path")?;
    let cohort =
        std::env::var_os("MOLTEN_TEST_SERVICE_HOST_COHORT").or_fail("actual admitted service cohort directory")?;
    let mut crash = twin::Live::open("public-node-recovery-crash", &trust, &cohort)?;
    let facts = scenarios::crash_readmission(&mut crash)?;
    for scenario in [
        molten::harness::composition::FailureScenario::DelayedPreRestartEventDelivery,
        molten::harness::composition::FailureScenario::DurableWorkReadmission,
        molten::harness::composition::FailureScenario::MemberFailureWithDependentRestart,
    ] {
        assert_correct(scenario, &facts);
    }
    let mut storm = twin::Live::open("public-node-recovery-storm", &trust, &cohort)?;
    let facts = scenarios::storm(&mut storm)?;
    assert_correct(molten::harness::composition::FailureScenario::RestartStormToEscalation, &facts);
    assert!(facts.sibling_progressed, "sibling must execute actual callbacks while the storm terminates");
    let mut upgrade = twin::Live::open("public-node-recovery-upgrade", &trust, &cohort)?;
    let facts = scenarios::upgrade(&mut upgrade)?;
    assert_correct(molten::harness::composition::FailureScenario::GenerationUpgradeWithStaleWorkRejection, &facts);
    Ok(())
}

// r[verify molten.testing.service_recovery_composition]
#[test]
#[ignore = "requires separately admitted genuine fencing/history/transformation fault-build cohorts"]
fn live_public_node_broken_variants_fail_shared_properties() -> TestResult<()> {
    let fencing_trust =
        std::env::var_os("MOLTEN_TEST_FENCING_BROKEN_SOURCE_TRUST").or_fail("fencing fault-build trust")?;
    let fencing_cohort = std::env::var_os("MOLTEN_TEST_FENCING_BROKEN_COHORT").or_fail("fencing fault-build cohort")?;
    let mut fencing = twin::Live::open("public-node-broken-fencing", &fencing_trust, &fencing_cohort)?;
    let facts = scenarios::crash_readmission(&mut fencing)?;
    assert_violation(
        molten::harness::composition::FailureScenario::DelayedPreRestartEventDelivery,
        molten::harness::composition::ServiceProperty::StaleInstancesLoseInfluence,
        &facts,
    );
    let storm_trust = std::env::var_os("MOLTEN_TEST_STORM_BROKEN_SOURCE_TRUST").or_fail("storm fault-build trust")?;
    let storm_cohort = std::env::var_os("MOLTEN_TEST_STORM_BROKEN_COHORT").or_fail("storm fault-build cohort")?;
    let mut storm = twin::Live::open("public-node-broken-storm", &storm_trust, &storm_cohort)?;
    let facts = scenarios::storm(&mut storm)?;
    assert_violation(
        molten::harness::composition::FailureScenario::RestartStormToEscalation,
        molten::harness::composition::ServiceProperty::StormsTerminateWithinWindow,
        &facts,
    );
    let migration_trust =
        std::env::var_os("MOLTEN_TEST_MIGRATION_BROKEN_SOURCE_TRUST").or_fail("migration fault-build trust")?;
    let migration_cohort =
        std::env::var_os("MOLTEN_TEST_MIGRATION_BROKEN_COHORT").or_fail("migration fault-build cohort")?;
    let mut migration = twin::Live::open("public-node-broken-migration", &migration_trust, &migration_cohort)?;
    let facts = scenarios::upgrade(&mut migration)?;
    assert_violation(
        molten::harness::composition::FailureScenario::GenerationUpgradeWithStaleWorkRejection,
        molten::harness::composition::ServiceProperty::StaleInstancesLoseInfluence,
        &facts,
    );
    Ok(())
}

fn assert_correct(
    scenario: molten::harness::composition::FailureScenario,
    facts: &molten::harness::composition::ScenarioFacts,
) {
    let outcomes = molten::harness::composition::evaluate_properties(scenario, facts);
    assert!(
        outcomes.iter().all(|outcome| outcome.holds),
        "actual public-node shared property failure: {outcomes:?}"
    );
}

fn assert_violation(
    scenario: molten::harness::composition::FailureScenario,
    property: molten::harness::composition::ServiceProperty,
    facts: &molten::harness::composition::ScenarioFacts,
) {
    let outcomes = molten::harness::composition::evaluate_properties(scenario, facts);
    assert!(
        outcomes.iter().any(|outcome| outcome.property == property && !outcome.holds),
        "actual faulty build did not falsify the shared property: {outcomes:?}"
    );
}
