//! Bounded Hegel properties of the seeded generator and the public history projection.

// r[verify molten.consensus.live_operation_generator]
// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[hegel::test(test_cases = 32)]
fn hegel_seeded_plans_are_deterministic_wave_distinct_and_tamper_evident(tc: hegel::TestCase) {
    let mut profile = super::support::register();
    profile.seed = tc.draw(hegel::generators::integers::<u64>());
    profile.operations = tc.draw(hegel::generators::integers::<u32>().min_value(6).max_value(24));
    profile.control_operations = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(4));
    profile.concurrency = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(2));
    profile.read_weight = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(9));
    profile.write_weight = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(9));
    profile.evidence.max_events = profile.operations * profile.evidence.max_attempts_per_operation * 2;
    let admitted = crate::profile::admit(&profile, super::support::content_ref).expect("bounded profile");
    let plan = crate::generator::generate(&admitted).expect("plan");
    assert_eq!(crate::generator::generate(&admitted), Ok(plan.clone()));
    assert_eq!(crate::generator::validate_plan(&admitted, &plan), Ok(()));
    assert_eq!(plan.operations.len(), usize::try_from(profile.operations).expect("bounded operation count"));

    let final_start = plan.operations.len() - profile.endpoints.len();
    for (operation, endpoint) in plan.operations[final_start..].iter().zip(&profile.endpoints) {
        assert_eq!(operation.function, crate::generator::Function::Read);
        assert_eq!(&operation.endpoint, endpoint);
    }
    for wave in plan.operations.chunks(usize::try_from(profile.concurrency).expect("bounded concurrency")) {
        let processes =
            wave.iter().map(|operation| operation.process.as_str()).collect::<alloc::collections::BTreeSet<_>>();
        assert_eq!(processes.len(), wave.len(), "one wave never reuses a client process");
    }

    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(plan.choices.len() - 1));
    let mut tampered = plan.clone();
    tampered.choices[index].endpoint_draw ^= 1;
    assert!(crate::generator::validate_plan(&admitted, &tampered).is_err());
}

// r[verify molten.consensus.live_operation_history]
#[hegel::test(test_cases = 32)]
fn hegel_projection_never_admits_out_of_bound_attempts_or_reversed_time(tc: hegel::TestCase) {
    let admitted = super::support::admitted();
    let plan = crate::generator::generate(&admitted).expect("plan");
    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(plan.operations.len() - 1));
    let operation = &plan.operations[index];
    let ordinal = tc.draw(hegel::generators::integers::<u32>().min_value(0).max_value(6));
    let started = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000));
    let completed = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000));
    let mut observation = super::support::observation(crate::history::PublicOutcome::Timeout);
    observation.ordinal = ordinal;
    observation.started_ns = started;
    observation.completed_ns = completed;
    let projected = crate::history::project_attempt(&admitted, operation, &observation);
    let is_in_bounds = ordinal < admitted.profile().evidence.max_attempts_per_operation && completed >= started;
    assert_eq!(projected.is_ok(), is_in_bounds, "ordinal {ordinal} started {started} completed {completed}");
    if let Ok(events) = projected {
        assert_eq!(events.len(), 2);
    }
}
