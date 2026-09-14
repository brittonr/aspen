use super::super::canonical::canonical_retry_events;
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
const PUBLIC_FAULT_ADVANCE: u64 = 1;
const PUBLIC_FINAL_TICKS: u64 = PUBLIC_RETRY_DEADLINE + PUBLIC_FAULT_ADVANCE;
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
    assert_eq!(run.report.report.final_time_ticks, PUBLIC_FINAL_TICKS);
    let terminal = canonical_named_event(
        &run.simulation_profile.profile_ref,
        CanonicalTimeEventKind::Conformance,
        GENERATION,
        "simulation-run-state",
        "completed",
        PUBLIC_FINAL_TICKS,
    )
    .expect("expected terminal event");
    assert_eq!(run.report.report.terminal_outcome_ref, terminal.evidence_ref);
    assert!(run.events.contains(&terminal));
    let timer = support::expected_timer_event(&run.simulation_profile, SATURATION_SUBJECT, PUBLIC_RETRY_DEADLINE);
    assert!(run.events.contains(&timer));
    assert!(run.report.report.evidence_refs.contains(&timer.evidence_ref));
}

// r[verify molten.audit_f12.saturation]
// r[verify molten.audit_f12.validation]
#[test]
fn executable_fixture_records_corrected_retry_delay_and_deadline() {
    let run =
        run_executable_fabric_time_fixture(FabricTimeFixtureSelection::DeterministicSimulation).expect("retry fixture");
    let plan =
        support::expected_plan(&run.simulation_profile, SATURATION_SUBJECT, SATURATED_DELAY, PUBLIC_RETRY_DEADLINE);
    let expected = canonical_retry_events(&run.simulation_profile.profile_ref, &plan).expect("expected retry events");
    for event in expected {
        assert!(run.events.contains(&event), "missing canonical retry observation: {}", event.evidence_ref);
        assert!(run.report.report.evidence_refs.contains(&event.evidence_ref));
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
    let wrapped_plan = support::expected_plan(&run.simulation_profile, SATURATION_SUBJECT, 0, PUBLIC_RETRY_NOW);
    let wrapped = canonical_retry_events(&run.simulation_profile.profile_ref, &wrapped_plan).expect("wrapped history");
    for event in wrapped {
        assert!(!run.events.contains(&event), "the fixture must not publish a wrapped retry observation");
    }
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
