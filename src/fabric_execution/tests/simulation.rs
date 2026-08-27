use super::*;

// r[verify molten.fabric_execution.simulation]
// r[verify molten.fabric_execution.validation]
#[test]
fn simulation_replays_equal_receipts_and_preserves_unknown_without_retry() {
    let (profile, request) =
        canonical_request(ExecutionProfileKind::DeterministicSimulation, script_arguments("ignored-by-simulation"));
    let mut scripts = BTreeMap::new();
    scripts.insert(HASH_B.to_string(), ScriptedExecutionObservation {
        process: scripted_process(ExecutionLifecycleState::Exited, EXPECTED_STDOUT),
    });
    let mut first = SimulatedExecutionAdapter::new(profile.clone(), MemoryPublisher::default(), scripts.clone())
        .expect("first simulation adapter");
    let mut second = SimulatedExecutionAdapter::new(profile.clone(), MemoryPublisher::default(), scripts)
        .expect("second simulation adapter");
    let first_receipt = first.execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None).expect("first simulation");
    let second_receipt =
        second.execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None).expect("second simulation");
    assert_eq!(first_receipt.receipt_ref, second_receipt.receipt_ref);
    assert_eq!(first_receipt.value, second_receipt.value);

    let mut unknown_scripts = BTreeMap::new();
    unknown_scripts.insert(HASH_B.to_string(), ScriptedExecutionObservation {
        process: scripted_process(ExecutionLifecycleState::Unknown, &[]),
    });
    let mut unknown = SimulatedExecutionAdapter::new(profile, MemoryPublisher::default(), unknown_scripts)
        .expect("unknown simulation adapter");
    let failure = unknown
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect_err("unknown observation must not become success");
    assert_eq!(failure.kind, ExecutionPortFailureKind::UnknownAfterStart);
    assert_eq!(unknown.reconcile(HASH_B, GENERATION), ExecutionReconciliationStatus::UnknownRequiresReconciliation);
}

// r[verify molten.fabric_execution.simulation]
// r[verify molten.fabric_execution.validation]
#[test]
fn live_and_simulation_compositions_share_command_and_outcome_shape() {
    let arguments = script_arguments("printf 'bounded:input'");
    let (live_profile, live_request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, arguments.clone());
    let mut live = crate::system_extension::compose_system_extension_execution_fabric(
        live_profile,
        MemoryPublisher::default(),
        BTreeMap::new(),
    )
    .expect("live composition");
    let live_receipt = live
        .execute(&live_request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("live composed execution");

    let (simulation_profile, simulation_request) =
        canonical_request(ExecutionProfileKind::DeterministicSimulation, arguments);
    let mut scripts = BTreeMap::new();
    scripts.insert(HASH_B.to_string(), ScriptedExecutionObservation {
        process: scripted_process(ExecutionLifecycleState::Exited, EXPECTED_STDOUT),
    });
    let mut simulation = crate::system_extension::compose_system_extension_execution_fabric(
        simulation_profile,
        MemoryPublisher::default(),
        scripts,
    )
    .expect("simulation composition");
    let simulation_receipt = simulation
        .execute(&simulation_request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("simulated composed execution");

    assert_eq!(live_request.plan.request.arguments, simulation_request.plan.request.arguments);
    assert_eq!(live_receipt.process.lifecycle, simulation_receipt.process.lifecycle);
    assert_eq!(live_receipt.process.disposition, simulation_receipt.process.disposition);
    assert_eq!(live_receipt.process.stdout.retained_bytes, simulation_receipt.process.stdout.retained_bytes);
}

// r[verify molten.fabric_execution.port_contract]
#[test]
fn system_extension_composition_selects_only_the_exact_profile() {
    let live =
        canonical_admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess)).expect("live profile");
    let selected = crate::system_extension::compose_system_extension_execution_fabric(
        live.clone(),
        MemoryPublisher::default(),
        BTreeMap::new(),
    )
    .expect("live composition");
    assert!(matches!(selected, crate::system_extension::SystemExtensionExecutionFabric::Live(_)));

    let mut scripts = BTreeMap::new();
    scripts.insert(HASH_B.to_string(), ScriptedExecutionObservation {
        process: scripted_process(ExecutionLifecycleState::Exited, &[]),
    });
    assert_eq!(
        crate::system_extension::compose_system_extension_execution_fabric(live, MemoryPublisher::default(), scripts,)
            .err(),
        Some(crate::system_extension::SystemExtensionExecutionFabricSelectionError::LiveProfileHasSimulationScripts)
    );
}

// r[verify molten.fabric_execution.port_contract]
#[test]
fn unavailable_profile_has_no_hidden_fallback() {
    let failure = unavailable_execution_port_failure(HASH_B);
    assert_eq!(failure.kind, ExecutionPortFailureKind::ProfileUnavailable);
    assert!(failure.detail.contains("no fallback"));
    let live_profile =
        canonical_admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess)).expect("live profile");
    assert_eq!(
        SimulatedExecutionAdapter::new(live_profile, MemoryPublisher::default(), BTreeMap::new()).err(),
        Some(SimulatedExecutionAdapterBuildError::WrongProfileKind)
    );
}
