use super::super::fixture::retry as fixture_retry;
use super::*;

mod admission;
mod replay;
mod support;

const FIXTURE_RETRY_NOW: u64 = 40;
const SATURATED_DELAY: u64 = 128;
const SATURATED_DEADLINE: u64 = FIXTURE_RETRY_NOW + SATURATED_DELAY;
const PUBLIC_RETRY_NOW: u64 = 35;
const PUBLIC_RETRY_DEADLINE: u64 = PUBLIC_RETRY_NOW + SATURATED_DELAY;
const ORDINARY_BASE: u64 = 5;
const ORDINARY_FACTOR: u64 = 2;
const ORDINARY_JITTER: u64 = 2;
const ORDINARY_DEADLINE: u64 = FIXTURE_RETRY_NOW + ORDINARY_BASE * ORDINARY_FACTOR + ORDINARY_JITTER;
const SATURATION_SUBJECT: &str = "fixture-retry-saturation";

// r[verify molten.audit_f12.validation]
#[test]
fn executable_fixture_does_not_finish_before_its_retry_timer_fires() {
    let run =
        run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation).expect("retry fixture");
    assert!(
        run.report.report.final_time_ticks >= PUBLIC_RETRY_DEADLINE,
        "the run completed at {} before the retry timer deadline {PUBLIC_RETRY_DEADLINE}",
        run.report.report.final_time_ticks,
    );
}

// r[verify molten.audit_f12.saturation]
// r[verify molten.audit_f12.validation]
#[test]
fn executable_fixture_records_corrected_retry_delay_and_deadline() {
    let run =
        run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation).expect("retry fixture");
    for (action, ticks) in [
        ("retry-delay", SATURATED_DELAY),
        ("retry-planned", PUBLIC_RETRY_DEADLINE),
    ] {
        let expected = canonical_named_event(
            &run.simulation_profile.profile_ref,
            CanonicalTimeEventKind::Deadline,
            GENERATION,
            SATURATION_SUBJECT,
            action,
            ticks,
        )
        .expect("expected retry event");
        assert!(run.events.contains(&expected), "missing canonical retry observation: {action}");
        assert!(run.report.report.evidence_refs.contains(&expected.evidence_ref));
    }
}

// r[verify molten.audit_f12.compatibility]
// r[verify molten.audit_f12.validation]
#[test]
fn executable_fixture_preserves_ordinary_retry_and_exposes_wrapped_replay_rejection() {
    let run =
        run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation).expect("retry fixture");
    let ordinary = canonical_named_event(
        &run.simulation_profile.profile_ref,
        CanonicalTimeEventKind::Deadline,
        GENERATION,
        "fixture-retry",
        "retry-planned",
        ORDINARY_DEADLINE,
    )
    .expect("ordinary retry event");
    assert!(run.events.contains(&ordinary));
    let wrapped = canonical_named_event(
        &run.simulation_profile.profile_ref,
        CanonicalTimeEventKind::Deadline,
        GENERATION,
        SATURATION_SUBJECT,
        "retry-delay",
        0,
    )
    .expect("wrapped counterexample");
    assert!(!run.events.contains(&wrapped), "the fixture must not publish a wrapped retry delay");
    let rejection = canonical_named_event(
        &run.simulation_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        GENERATION,
        SATURATION_SUBJECT,
        "wrapped-replay-rejected",
        PUBLIC_RETRY_NOW,
    )
    .expect("replay rejection event");
    assert!(run.events.contains(&rejection));
    assert!(run.report.report.evidence_refs.contains(&rejection.evidence_ref));
}
