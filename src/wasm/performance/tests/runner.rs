use super::super::*;

const RAW_SAMPLE_BASE_COUNT: u64 = 1_000;
const RAW_FAST_PROCESSES: u32 = 1;
const RAW_FAST_ITERATIONS: u32 = 3;
const DIAGNOSTIC_PROCESS_ID_OFFSET: u64 = 4_000;
const MUTABLE_RUNNER_BYTES: &[u8] = b"not-the-admitted-runner";

fn raw_measurements(architecture: &str) -> Vec<u8> {
    raw_measurements_with_sampling(architecture, RAW_FAST_PROCESSES, RAW_FAST_ITERATIONS)
}

fn raw_measurements_with_sampling(architecture: &str, processes: u32, iterations: u32) -> Vec<u8> {
    let mut values = Vec::new();
    for phase in ["Compilation", "Instantiation", "Execution"] {
        for process in 0..processes {
            for iteration in 0..iterations {
                values.push(serde_json::json!({
                    "arch": architecture,
                    "engine": "/diagnostic/path/libengine.so",
                    "engine_flags": null,
                    "wasm": "/diagnostic/path/benchmark.wasm",
                    "process": process,
                    "iteration": iteration,
                    "phase": phase,
                    "event": super::support::FIXTURE_EVENT,
                    "count": RAW_SAMPLE_BASE_COUNT + u64::from(iteration),
                }));
            }
        }
    }
    serde_json::to_vec(&values).expect("Sightglass JSON fixture")
}

#[test]
fn sightglass_raw_json_preserves_separate_bounded_phase_samples() {
    // r[verify molten.wasm_performance.phases]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let phases = parse_sightglass_measurements(
        &profile,
        &profile.fast,
        super::support::FIXTURE_ARCHITECTURE,
        &raw_measurements(super::support::FIXTURE_ARCHITECTURE),
    )
    .expect("Sightglass measurements parse");
    assert_eq!(phases.len(), PerformancePhase::ALL.len());
    assert_eq!(phases.iter().map(|phase| phase.phase).collect::<Vec<_>>(), PerformancePhase::ALL);
    assert!(phases.iter().all(|phase| phase.samples.len()
        == usize::try_from(profile.fast.sampling.min_samples_per_phase).expect("sampling count fits usize")));
    assert!(phases.iter().all(|phase| phase.event == super::support::FIXTURE_EVENT));

    let deep = parse_sightglass_measurements(
        &profile,
        &profile.deep,
        super::support::FIXTURE_ARCHITECTURE,
        &raw_measurements_with_sampling(
            super::support::FIXTURE_ARCHITECTURE,
            profile.deep.sampling.processes,
            profile.deep.sampling.iterations_per_process,
        ),
    )
    .expect("deep Sightglass measurements parse");
    assert!(deep.iter().all(|phase| phase.samples.len()
        == usize::try_from(profile.deep.sampling.min_samples_per_phase).expect("sampling count fits usize")));

    let arguments = sightglass_arguments(&profile.fast);
    for required in [
        "benchmark",
        "--processes",
        "--iterations-per-process",
        "--measure",
        "--raw",
        "--output-format",
        "json",
        "--pin",
    ] {
        assert!(arguments.contains(&std::ffi::OsString::from(required)));
    }
    let process_flag =
        arguments.iter().position(|argument| argument == "--processes").expect("Sightglass process flag");
    assert_eq!(arguments[process_flag + 1], "1");
    assert!(!arguments.iter().any(|argument| argument.to_string_lossy().contains("wizer")));
    assert!(!arguments.iter().any(|argument| argument.to_string_lossy().contains("precompile")));
}

#[test]
fn sightglass_process_ids_are_normalized_out_of_canonical_samples() {
    // r[verify molten.wasm_performance.evidence]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let mut raw: Vec<serde_json::Value> = serde_json::from_slice(&raw_measurements_with_sampling(
        super::support::FIXTURE_ARCHITECTURE,
        profile.deep.sampling.processes,
        profile.deep.sampling.iterations_per_process,
    ))
    .expect("raw measurement fixture");
    for measurement in &mut raw {
        let diagnostic_process =
            measurement.get("process").and_then(serde_json::Value::as_u64).expect("diagnostic process id");
        measurement["process"] = serde_json::json!(diagnostic_process + DIAGNOSTIC_PROCESS_ID_OFFSET);
    }
    let bytes = serde_json::to_vec(&raw).expect("diagnostic process fixture");
    let phases = parse_sightglass_measurements(&profile, &profile.deep, super::support::FIXTURE_ARCHITECTURE, &bytes)
        .expect("diagnostic process ids normalize");
    let maximum_process = phases
        .iter()
        .flat_map(|phase| &phase.samples)
        .map(|sample| sample.process)
        .max()
        .expect("normalized process ordinal");
    assert_eq!(maximum_process + 1, profile.deep.sampling.processes);
    assert!(maximum_process < u32::try_from(DIAGNOSTIC_PROCESS_ID_OFFSET).expect("diagnostic process offset"));
}

#[test]
fn process_shell_validates_profile_and_suite_before_spawning() {
    // r[verify molten.wasm_performance.phases]
    // r[verify molten.wasm_performance.functional_core]
    let profile = supported_performance_profile().expect("supported performance profile");
    let mut invalid_suite = profile.fast.clone();
    invalid_suite.sampling.processes = profile.deep.sampling.processes;
    let missing_program = std::path::Path::new("/definitely-missing-sightglass");
    let missing_engine = std::path::Path::new("/diagnostic/mantle-engine.so");
    let missing_benchmark = std::path::Path::new("/diagnostic/mantle-benchmark.wasm");
    let invalid = run_sightglass_process(&SightglassProcessInvocation {
        program: missing_program,
        engine: missing_engine,
        benchmark: missing_benchmark,
        benchmark_ref: &invalid_suite.workload_refs[0],
        profile: &profile,
        suite: &invalid_suite,
        expected_architecture: super::support::FIXTURE_ARCHITECTURE,
        max_output_bytes: profile.comparison.max_sightglass_output_bytes,
    })
    .expect_err("invalid suite denies before process spawn");
    assert!(invalid.blockers.iter().any(|blocker| blocker.contains("lane configuration")));
    assert!(!invalid.blockers.iter().any(|blocker| blocker.contains("could not start")));

    let spawn = run_sightglass_process(&SightglassProcessInvocation {
        program: missing_program,
        engine: missing_engine,
        benchmark: missing_benchmark,
        benchmark_ref: &profile.fast.workload_refs[0],
        profile: &profile,
        suite: &profile.fast,
        expected_architecture: super::support::FIXTURE_ARCHITECTURE,
        max_output_bytes: profile.comparison.max_sightglass_output_bytes,
    })
    .expect_err("missing pinned runner denies diagnostically");
    assert!(spawn.blockers.iter().any(|blocker| blocker.contains("could not open")));
}

#[test]
fn process_shell_remeasures_runner_bytes_before_any_execution() {
    // r[verify molten.wasm_performance.materialization]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let workspace =
        crate::test_support::TestWorkspace::new("wasm-performance-runner-identity").expect("capability test workspace");
    let state = workspace.state().expect("state capability");
    let runner_locator = crate::test_support::WorkspacePath::parse("runner").expect("runner locator");
    state.write(&runner_locator, MUTABLE_RUNNER_BYTES).expect("write runner fixture");
    let process_root = workspace.process_bridge().plan(&state).expect("diagnostic process bridge");
    let runner_path = process_root.path().join("runner");
    let mut suite = profile.fast.clone();
    suite.runner_artifact_ref = super::support::fixture_ref("different-runner-bytes");
    let denial = run_sightglass_process(&SightglassProcessInvocation {
        program: &runner_path,
        engine: std::path::Path::new("/unused-engine"),
        benchmark: std::path::Path::new("/unused-benchmark"),
        benchmark_ref: &suite.workload_refs[0],
        profile: &profile,
        suite: &suite,
        expected_architecture: super::support::FIXTURE_ARCHITECTURE,
        max_output_bytes: profile.comparison.max_sightglass_output_bytes,
    })
    .expect_err("runner byte mismatch denies before execution");
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("differs from its admitted content identity")));

    suite.runner_artifact_ref = super::support::fixture_bytes_ref(MUTABLE_RUNNER_BYTES);
    let mutable_denial = run_sightglass_process(&SightglassProcessInvocation {
        program: &runner_path,
        engine: std::path::Path::new("/unused-engine"),
        benchmark: std::path::Path::new("/unused-benchmark"),
        benchmark_ref: &suite.workload_refs[0],
        profile: &profile,
        suite: &suite,
        expected_architecture: super::support::FIXTURE_ARCHITECTURE,
        max_output_bytes: profile.comparison.max_sightglass_output_bytes,
    })
    .expect_err("mutable runner denies after content admission");
    assert!(mutable_denial.blockers.iter().any(|blocker| blocker.contains("mutable after content admission")));
}

#[test]
fn malformed_cross_architecture_missing_phase_and_wrong_event_outputs_deny() {
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, b"not-json")
            .is_err()
    );
    assert!(
        parse_sightglass_measurements(
            &profile,
            &profile.fast,
            super::support::FIXTURE_ARCHITECTURE,
            &raw_measurements("aarch64"),
        )
        .is_err()
    );

    let mut missing =
        serde_json::from_slice::<Vec<serde_json::Value>>(&raw_measurements(super::support::FIXTURE_ARCHITECTURE))
            .expect("Sightglass fixture values");
    missing.retain(|value| value["phase"] != "Execution");
    let missing = serde_json::to_vec(&missing).expect("missing phase fixture");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, &missing).is_err()
    );

    let mut wrong_event =
        serde_json::from_slice::<Vec<serde_json::Value>>(&raw_measurements(super::support::FIXTURE_ARCHITECTURE))
            .expect("Sightglass fixture values");
    for value in &mut wrong_event {
        value["event"] = serde_json::json!("nanoseconds");
    }
    let wrong_event = serde_json::to_vec(&wrong_event).expect("wrong event fixture");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, &wrong_event)
            .is_err()
    );

    let mut mixed_identity =
        serde_json::from_slice::<Vec<serde_json::Value>>(&raw_measurements(super::support::FIXTURE_ARCHITECTURE))
            .expect("Sightglass fixture values");
    mixed_identity[0]["engine"] = serde_json::json!("/diagnostic/path/other-engine.so");
    let mixed_identity = serde_json::to_vec(&mixed_identity).expect("mixed identity fixture");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, &mixed_identity)
            .is_err()
    );

    let mut duplicate =
        serde_json::from_slice::<Vec<serde_json::Value>>(&raw_measurements(super::support::FIXTURE_ARCHITECTURE))
            .expect("Sightglass fixture values");
    duplicate.push(duplicate[0].clone());
    let duplicate = serde_json::to_vec(&duplicate).expect("duplicate sample fixture");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, &duplicate)
            .is_err()
    );

    let mut zero =
        serde_json::from_slice::<Vec<serde_json::Value>>(&raw_measurements(super::support::FIXTURE_ARCHITECTURE))
            .expect("Sightglass fixture values");
    zero[0]["count"] = serde_json::json!(0);
    let zero = serde_json::to_vec(&zero).expect("zero sample fixture");
    assert!(
        parse_sightglass_measurements(&profile, &profile.fast, super::support::FIXTURE_ARCHITECTURE, &zero).is_err()
    );
}
