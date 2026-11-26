/// Builds a live coordination runtime on the built-in single-process control-registry group, sampling time
/// from `clock`, which must be a sealed live clock from [`CoordinationClock::live`].
///
/// The manifest's `control_group_ref` must name that group's manifest; any other group fails closed
/// instead of being silently replaced. Use [`new_coordination_runtime_with_control_group`] to bind an
/// explicit control-registry group manifest.
pub fn new_coordination_runtime(manifest_value: &IoValue, clock: CoordinationClock) -> Result<CoordinationRuntime> {
    let control_group_value = crate::raft_control_plane::control_registry_fixture_manifest_value()?;
    new_coordination_runtime_with_control_group(manifest_value, &control_group_value, clock)
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// Builds a live coordination runtime whose in-process control registry runs the supplied group manifest.
/// The coordination manifest's `control_group_ref` must equal that manifest's canonical ref. `clock` must be a
/// sealed live clock whose basis matches its provenance, whatever crate features are enabled; deterministic
/// clocks are accepted only by [`new_coordination_simulation_runtime`].
pub fn new_coordination_runtime_with_control_group(
    manifest_value: &IoValue,
    control_group_manifest_value: &IoValue,
    clock: CoordinationClock,
) -> Result<CoordinationRuntime> {
    clock.verify_live_provenance()?;
    build_coordination_runtime(manifest_value, control_group_manifest_value, clock)
}

/// Builds a deterministic simulation runtime on the built-in control-registry group. It accepts only a
/// deterministic clock (outside tests, `run-fixture` uses one frozen at tick zero, so its leases never expire),
/// and its clock is never replaced by a live clock transition. It serves simulations, not live coordination.
pub fn new_coordination_simulation_runtime(manifest_value: &IoValue, clock: CoordinationClock) -> Result<CoordinationRuntime> {
    if clock.is_live() {
        return Err(MoltenError::invalid_harness(
            "coordination simulation runtimes take a deterministic clock; live clocks use new_coordination_runtime",
        ));
    }
    let control_group_value = crate::raft_control_plane::control_registry_fixture_manifest_value()?;
    build_coordination_runtime(manifest_value, &control_group_value, clock)
}

fn build_coordination_runtime(
    manifest_value: &IoValue,
    control_group_manifest_value: &IoValue,
    clock: CoordinationClock,
) -> Result<CoordinationRuntime> {
    let manifest = parse_coordination_service_manifest(manifest_value)?;
    let control_group_ref = canonical_hash(control_group_manifest_value)?;
    if manifest.control_group_ref != control_group_ref {
        return Err(MoltenError::invalid_harness(format!(
            "coordination manifest control group {} does not match the bound control-registry group {control_group_ref}",
            manifest.control_group_ref
        )));
    }
    let raft = crate::raft_control_plane::new_control_registry_model_runtime(control_group_manifest_value)?;
    let time_basis_ref = clock.time_basis_ref().to_string();
    Ok(CoordinationRuntime {
        clock,
        manifest,
        raft,
        state: CoordinationState {
            next_fencing_token: 1,
            time_basis_ref,
            ..CoordinationState::default()
        },
        applied_operations: OrderedMap::new(),
        receipts: Vec::new(),
        assertions: Vec::new(),
        tokens: Vec::new(),
        next_sequence: 1,
    })
}

pub fn apply_coordination_request(
    runtime: &mut CoordinationRuntime,
    request_value: &IoValue,
) -> Result<CoordinationApplyResult> {
    let request = parse_coordination_request(request_value)?;
    let current_snapshot = snapshot_from_state(&runtime.state)?;
    if let Some(existing) = runtime.applied_operations.get(&request.operation_id_ref).cloned() {
        return replay_or_conflicting_duplicate(runtime, request, current_snapshot, existing);
    }
    let mut diagnostics = Vec::new();
    collect_admission_diagnostics(runtime, &request, &mut diagnostics)?;
    if diagnostics.is_empty() {
        if request.operation == OP_READ {
            return apply_coordination_read(runtime, request);
        }
        // A clock-port failure is an infrastructure error, not a decision: it returns its fabric failure class
        // before any receipt or operation is recorded, so a retry with the same operation id samples again.
        let sample = runtime.clock.sample().map_err(MoltenError::from)?;
        let transition = primitive_transition(runtime, &request, &sample)?;
        if transition.decision == "pass" {
            return commit_prepared_mutation(runtime, request, current_snapshot, transition);
        }
        return deny_transition_result(runtime, request, current_snapshot, transition, &["semantic-state-transition", "fail"]);
    }
    deny_result(runtime, request, current_snapshot, diagnostics, &["admission-gate", "fail"])
}

pub fn coordination_fixture_manifest_value() -> Result<IoValue> {
    let group_ref = canonical_hash(&crate::raft_control_plane::control_registry_fixture_manifest_value()?)?;
    coordination_service_manifest_value(&CoordinationServiceManifestInput {
        service_id: DEFAULT_COORDINATION_SERVICE_ID.to_string(),
        services: supported_services(),
        control_group_ref: group_ref,
        queue_capacity: DEFAULT_COORDINATION_QUEUE_CAPACITY,
        semaphore_capacity: DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
        rate_limit: DEFAULT_COORDINATION_RATE_LIMIT,
        barrier_parties: DEFAULT_COORDINATION_BARRIER_PARTIES,
        policy_refs: vec![fixture_ref("coordination-policy")],
        resource_refs: vec![fixture_ref("coordination-resource")],
    })
}
