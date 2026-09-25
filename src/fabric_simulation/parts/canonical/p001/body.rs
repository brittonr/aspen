
// r[impl molten.fabric_simulation.live_sim_differential]
pub fn canonical_simulation_differential(
    input: DifferentialInput<'_>,
) -> crate::error::Result<CanonicalSimulationDifferential> {
    let DifferentialInput {
        simulation_profile_ref,
        live_profile_ref,
        shared_contract_ref,
        simulation_trace_refs,
        live_trace_refs,
        normalized_difference_refs,
    } = input;
    for reference in [simulation_profile_ref, live_profile_ref, shared_contract_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    validate_refs("simulation differential", simulation_trace_refs)?;
    validate_refs("live differential", live_trace_refs)?;
    validate_refs("normalized difference", &normalized_difference_refs)?;
    let is_equivalent = simulation_trace_refs == live_trace_refs && normalized_difference_refs.is_empty();
    let value = crate::preserves_rail::record("fabric-simulation-differential-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_DIFFERENTIAL_SCHEMA),
        field("simulation-profile-ref", crate::preserves_rail::string(simulation_profile_ref)),
        field("live-profile-ref", crate::preserves_rail::string(live_profile_ref)),
        field("shared-contract-ref", crate::preserves_rail::string(shared_contract_ref)),
        field("simulation-trace-refs", strings_value(simulation_trace_refs.iter().map(String::as_str))),
        field("live-trace-refs", strings_value(live_trace_refs.iter().map(String::as_str))),
        field("normalized-difference-refs", strings_value(normalized_difference_refs.iter().map(String::as_str))),
        field("equivalent", crate::preserves_rail::bool_value(is_equivalent)),
        checks(&[
            "shared-port-contract",
            "declared-capability-differences-visible",
            "no-live-production-equivalence-claim",
        ]),
    ]);
    let report_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulationDifferential {
        report_ref,
        simulation_profile_ref: simulation_profile_ref.to_string(),
        live_profile_ref: live_profile_ref.to_string(),
        shared_contract_ref: shared_contract_ref.to_string(),
        equivalent: is_equivalent,
        normalized_difference_refs,
        value,
    })
}

// r[impl molten.fabric_simulation.claim_ladder]
pub fn canonical_claim_promotion(
    current: SimulationClaimProfile,
    target: SimulationClaimProfile,
    evidence: &ClaimEvidence,
) -> crate::error::Result<CanonicalClaimPromotion> {
    let decision = evaluate_claim_promotion(current, target, evidence);
    let value = crate::preserves_rail::record("fabric-simulation-claim-profile-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_CLAIM_SCHEMA),
        field("current-profile", crate::preserves_rail::string(current.as_str())),
        field("target-profile", crate::preserves_rail::string(target.as_str())),
        field("evidence-profile", crate::preserves_rail::string(evidence.profile.as_str())),
        field("admitted", crate::preserves_rail::bool_value(decision.admitted)),
        field("missing-evidence", strings_value(decision.missing_evidence.iter().copied())),
        checks(&[
            "profile-specific-evidence-required",
            "stronger-profile-cannot-use-simulation-label",
            "decision-does-not-grant-runtime-authority",
        ]),
    ]);
    let decision_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalClaimPromotion {
        decision_ref,
        current,
        decision,
        value,
    })
}

// r[impl molten.fabric_simulation.replay_shrink]
pub fn canonical_simulation_shrink(
    original_world_ref: &str,
    shrunk_world: &CanonicalSimulatedWorld,
    result: ShrinkResult,
) -> crate::error::Result<CanonicalSimulationShrink> {
    crate::preserves_rail::validate_content_ref(original_world_ref)?;
    let value = crate::preserves_rail::record("fabric-simulation-shrink-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_SHRINK_SCHEMA),
        field("original-world-ref", crate::preserves_rail::string(original_world_ref)),
        field("shrunk-world-ref", crate::preserves_rail::string(&shrunk_world.world_ref)),
        field("attempts", crate::preserves_rail::u64_value(result.attempts)),
        field("removed-workload-steps", crate::preserves_rail::u64_value(result.removed_workload_steps)),
        field("failure-preserved", crate::preserves_rail::bool_value(result.failure_preserved)),
        checks(&[
            "candidate-replayed-from-initial-world",
            "invalid-candidates-rejected",
            "failure-class-preserved",
        ]),
    ]);
    let shrink_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulationShrink {
        shrink_ref,
        original_world_ref: original_world_ref.to_string(),
        shrunk_world_ref: shrunk_world.world_ref.clone(),
        result,
        value,
    })
}

// r[impl molten.fabric_simulation.evidence]
pub fn canonical_simulation_repro_bundle(
    world: &CanonicalSimulatedWorld,
    run: &CanonicalSimulationRun,
    shrink: Option<&CanonicalSimulationShrink>,
) -> crate::error::Result<CanonicalSimulationReproBundle> {
    if run.world_ref != world.world_ref {
        return Err(crate::error::MoltenError::invalid_harness(
            "simulation repro run does not bind the supplied world",
        ));
    }
    let value = crate::preserves_rail::record("fabric-simulation-repro-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_REPRO_SCHEMA),
        field("world-ref", crate::preserves_rail::string(&world.world_ref)),
        field("run-ref", crate::preserves_rail::string(&run.run_ref)),
        field("shrink-ref", optional_string(shrink.map(|item| item.shrink_ref.as_str()))),
        field("profile", crate::preserves_rail::string(run.profile.as_str())),
        field("non-claims", strings_value(world.admitted.manifest.non_claims.iter().map(|item| item.as_str()))),
        checks(&[
            "offline-verifiable-input-closure",
            "bounded-evidence-members",
            "secret-payload-excluded",
            "simulation-not-relabeled-live",
        ]),
    ]);
    let bundle_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalSimulationReproBundle {
        bundle_ref,
        world_ref: world.world_ref.clone(),
        run_ref: run.run_ref.clone(),
        shrink_ref: shrink.map(|item| item.shrink_ref.clone()),
        value,
    })
}

// r[impl molten.fabric_simulation.operator_workflow]
pub fn parse_simulation_run_readback(value: &preserves::IOValue) -> crate::error::Result<SimulationRunReadback> {
    let fields = value
        .collect_simple_record("fabric-simulation-run-v1", Some(RUN_READBACK_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected canonical fabric-simulation run"))?;
    let schema = required_string(&fields[0], "simulation run schema")?;
    if schema != FABRIC_SIMULATION_RUN_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric-simulation run schema mismatch: {schema}"
        )));
    }
    let decision = record_string_field(&fields[RUN_DECISION_FIELD_INDEX], "decision")?;
    let profile = record_string_field(&fields[RUN_PROFILE_FIELD_INDEX], "profile")?;
    if !matches!(decision.as_str(), "pass" | "invariant-failed" | "diverged" | "bound-exceeded" | "denied") {
        return Err(crate::error::MoltenError::invalid_harness(format!("unsupported simulation decision: {decision}")));
    }
    if !matches!(
        profile.as_str(),
        "pure-model" | "deterministic-whole-system" | "multi-process-live" | "host-chaos" | "vm-hardware"
    ) {
        return Err(crate::error::MoltenError::invalid_harness(format!("unsupported simulation profile: {profile}")));
    }
    let world_ref = record_string_field(&fields[RUN_WORLD_REF_FIELD_INDEX], "world-ref")?;
    crate::preserves_rail::validate_content_ref(&world_ref)?;
    let final_state_refs = record_string_sequence_field(&fields[RUN_FINAL_STATE_REFS_FIELD_INDEX], "final-state-refs")?;
    validate_refs("readback final state", &final_state_refs)?;
    Ok(SimulationRunReadback {
        decision,
        profile,
        choice_count: record_u64_field(&fields[RUN_CHOICE_COUNT_FIELD_INDEX], "choice-count")?,
        event_count: record_u64_field(&fields[RUN_EVENT_COUNT_FIELD_INDEX], "event-count")?,
        invariant_count: record_u64_field(&fields[RUN_INVARIANT_COUNT_FIELD_INDEX], "invariant-count")?,
        resource_units: record_u64_field(&fields[RUN_RESOURCE_UNITS_FIELD_INDEX], "resource-units")?,
        virtual_ticks: record_u64_field(&fields[RUN_VIRTUAL_TICKS_FIELD_INDEX], "virtual-ticks")?,
        world_ref,
        final_state_refs,
        first_divergence: record_optional_divergence(&fields[RUN_FIRST_DIVERGENCE_FIELD_INDEX])?,
        run_ref: crate::preserves_rail::canonical_hash(value)?,
    })
}

fn world_value(world: &SimulatedWorldManifest) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-world-v1", vec![
        crate::preserves_rail::string(FABRIC_SIMULATION_WORLD_SCHEMA),
        field("runtime-ref", crate::preserves_rail::string(&world.runtime_ref)),
        field("scheduler-input-ref", crate::preserves_rail::string(&world.scheduler_input_ref)),
        field("entropy-input-ref", crate::preserves_rail::string(&world.entropy_input_ref)),
        field("authority-ref", crate::preserves_rail::string(&world.authority_ref)),
        field("policy-ref", crate::preserves_rail::string(&world.policy_ref)),
        field("initial-durable-state-ref", crate::preserves_rail::string(&world.initial_durable_state_ref)),
        field("resource-profile-ref", crate::preserves_rail::string(&world.resource_profile_ref)),
        field("workload-ref", crate::preserves_rail::string(&world.workload_ref)),
        field("fault-plan-ref", crate::preserves_rail::string(&world.fault_plan_ref)),
        field("invariant-set-ref", crate::preserves_rail::string(&world.invariant_set_ref)),
        field("nodes", crate::preserves_rail::sequence(world.nodes.iter().map(node_value).collect())),
        field(
            "port-profiles",
            crate::preserves_rail::sequence(world.port_profiles.iter().map(port_profile_value).collect()),
        ),
        field(
            "workload",
            crate::preserves_rail::sequence(world.workload.iter().map(workload_step_value).collect()),
        ),
        field("faults", crate::preserves_rail::sequence(world.faults.iter().map(fault_value).collect())),
        field(
            "invariants",
            crate::preserves_rail::sequence(world.invariants.iter().map(invariant_value).collect()),
        ),
        field("bounds", bounds_value(&world.bounds)),
        field("claim-profile", crate::preserves_rail::string(world.claim_profile.as_str())),
        field("non-claims", strings_value(world.non_claims.iter().map(|claim| claim.as_str()))),
        field("ambient-inputs", strings_value(world.ambient_inputs.iter().map(String::as_str))),
        checks(&[
            "behavior-inputs-closed",
            "same-extension-core-bound",
            "ports-explicit",
            "exploration-bounded",
            "ambient-inputs-denied",
            "claim-profile-explicit",
        ]),
    ])
}

fn node_value(node: &SimulatedNode) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-node-v1", vec![
        field("node-id", crate::preserves_rail::string(&node.node_id)),
        field("extension-id", crate::preserves_rail::string(&node.extension_id)),
        field("service-id", crate::preserves_rail::string(&node.service_id)),
        field("generation", crate::preserves_rail::u64_value(node.generation)),
        field("initial-state-ref", crate::preserves_rail::string(&node.initial_state_ref)),
        field("membership-view-ref", crate::preserves_rail::string(&node.membership_view_ref)),
        field("placement-ref", crate::preserves_rail::string(&node.placement_ref)),
        field("consistency-profile-ref", crate::preserves_rail::string(&node.consistency_profile_ref)),
        field("simulation-core", core_identity_value(&node.same_core.simulation)),
        field("live-core", core_identity_value(&node.same_core.live)),
        field(
            "required-port-classes",
            strings_value(node.required_port_classes.iter().map(|class| class.as_str())),
        ),
    ])
}

fn core_identity_value(identity: &ExtensionCoreIdentity) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-core-identity-v1", vec![
        field("implementation-ref", crate::preserves_rail::string(&identity.implementation_ref)),
        field("manifest-ref", crate::preserves_rail::string(&identity.manifest_ref)),
        field("callback-dispatcher-ref", crate::preserves_rail::string(&identity.callback_dispatcher_ref)),
        field("protocol-core-ref", crate::preserves_rail::string(&identity.protocol_core_ref)),
        field("state-machine-ref", crate::preserves_rail::string(&identity.state_machine_ref)),
        field("schema-set-ref", crate::preserves_rail::string(&identity.schema_set_ref)),
        field("port-contract-set-ref", crate::preserves_rail::string(&identity.port_contract_set_ref)),
    ])
}

fn port_profile_value(profile: &SimulatedPortProfile) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-port-profile-v1", vec![
        field("class", crate::preserves_rail::string(profile.class.as_str())),
        field("port-id", crate::preserves_rail::string(&profile.port_id)),
        field("version", crate::preserves_rail::string(&profile.version)),
        field("implementation-profile", crate::preserves_rail::string(&profile.implementation_profile)),
        field("descriptor-ref", crate::preserves_rail::string(&profile.descriptor_ref)),
        field("command-schema-ref", crate::preserves_rail::string(&profile.command_schema_ref)),
        field("event-schema-ref", crate::preserves_rail::string(&profile.event_schema_ref)),
        field("deterministic", crate::preserves_rail::bool_value(profile.deterministic)),
        field("declared-faults", strings_value(profile.declared_faults.iter().map(|fault| fault.as_str()))),
    ])
}

fn workload_step_value(step: &SimulationWorkloadStep) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-workload-step-v1", vec![
        field("sequence", crate::preserves_rail::u64_value(step.sequence)),
        field("node-id", crate::preserves_rail::string(&step.node_id)),
        field("request-ref", crate::preserves_rail::string(&step.request_ref)),
        field("service", crate::preserves_rail::string(step.service.as_str())),
        field("expected-failure-class", optional_string(step.expected_failure_class.as_deref())),
    ])
}
