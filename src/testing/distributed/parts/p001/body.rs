#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiGateInput<'a> {
    pub matrix: &'a CiMatrix,
    pub metadata: &'a [TestMetadata],
    pub traceability_manifest: &'a crate::trace_core::TraceabilityManifest,
    pub runs: &'a [ProfileRun],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub traceability_ref: String,
    pub metadata_refs: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeFaultCase {
    pub case_id: String,
    pub invariant_name: String,
    pub simulation: SimulationInput,
    pub expected_decision: String,
    pub profile_eligibility: Vec<String>,
    pub cost_class: String,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeFaultSuite {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub case_refs: Vec<String>,
    pub run_refs: Vec<String>,
    pub suite_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedCasePromotionInput {
    pub case_id: String,
    pub invariant_name: String,
    pub seed_ref: String,
    pub topology_ref: String,
    pub scheduler_ref: String,
    pub fault_plan_ref: String,
    pub command_refs: Vec<String>,
    pub replay_ref: String,
    pub diagnostic_refs: Vec<String>,
    pub profile_eligibility: Vec<String>,
    pub traceability_refs: Vec<String>,
    pub retry_attempts: u64,
    pub variance_refs: Vec<String>,
    pub cost_class: String,
    pub release_review_status: String,
    pub diagnostic_only: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedCasePromotion {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub promotion_ref: String,
    pub value: IoValue,
}

pub fn topology_value(topology: &Topology) -> Result<IoValue> {
    validate_topology(topology)?;
    Ok(record("distributed-topology-v1", vec![
        string(DISTRIBUTED_TOPOLOGY_SCHEMA),
        record("peers", vec![sequence(peer_values(&topology.peers)?)]),
        record("channels", vec![sequence(channel_values(&topology.channels)?)]),
        record("caveats", vec![sequence(string_values(
            "topology caveat",
            &topology.caveats,
            MAX_DISTRIBUTED_TEXT,
        )?)]),
        checks_value(&[
            ("explicit-peer-set", PASS_DECISION),
            ("explicit-channel-set", PASS_DECISION),
            ("simulation-does-not-grant-transport-trust", PASS_DECISION),
        ]),
    ]))
}

pub fn scheduler_profile_value(profile: &SchedulerProfile) -> Result<IoValue> {
    validate_text("scheduler profile id", &profile.id)?;
    validate_text("scheduler policy", &profile.policy)?;
    if profile.max_ticks == 0 {
        return Err(MoltenError::invalid_harness("scheduler profile max_ticks must be positive"));
    }
    Ok(record("distributed-scheduler-profile-v1", vec![
        string(DISTRIBUTED_SCHEDULER_SCHEMA),
        record("id", vec![string(&profile.id)]),
        record("policy", vec![string(&profile.policy)]),
        record("max-ticks", vec![u64_value(profile.max_ticks)]),
        checks_value(&[("virtual-clock-only", PASS_DECISION)]),
    ]))
}

pub fn seed_value(seed: &SimulationSeed) -> Result<IoValue> {
    validate_text("simulation seed id", &seed.id)?;
    validate_ref(&seed.entropy_ref, "seed entropy")?;
    Ok(record("distributed-simulation-seed-v1", vec![
        string(DISTRIBUTED_SEED_SCHEMA),
        record("id", vec![string(&seed.id)]),
        record("entropy", vec![string(&seed.entropy_ref)]),
        checks_value(&[("declared-entropy-only", PASS_DECISION)]),
    ]))
}

pub fn fault_plan_value(plan: &FaultPlan) -> Result<IoValue> {
    validate_fault_plan(plan)?;
    Ok(record("distributed-fault-plan-v1", vec![
        string(DISTRIBUTED_FAULT_PLAN_SCHEMA),
        record("events", vec![sequence(fault_event_values(&plan.events)?)]),
        record("caveats", vec![sequence(string_values(
            "fault-plan caveat",
            &plan.caveats,
            MAX_DISTRIBUTED_TEXT,
        )?)]),
        checks_value(&[
            ("explicit-fault-targets", PASS_DECISION),
            ("ambient-runtime-state-excluded", PASS_DECISION),
        ]),
    ]))
}

pub fn run_simulation(input: &SimulationInput) -> Result<SimulationRun> {
    validate_simulation_input(input)?;
    let topology_ref = canonical_ref(&topology_value(&input.topology)?)?;
    let seed_ref = canonical_ref(&seed_value(&input.seed)?)?;
    let scheduler_ref = canonical_ref(&scheduler_profile_value(&input.scheduler)?)?;
    let fault_plan_ref = canonical_ref(&fault_plan_value(&input.fault_plan)?)?;
    let mut event_outcomes = Vec::with_capacity(input.commands.len());
    let mut committed = OrderedSet::new();
    let mut denied = OrderedSet::new();
    let mut diagnostics = OrderedSet::new();

    for (index, command) in input.commands.iter().enumerate() {
        let tick = command_tick(index)?;
        let evaluation = evaluate_command(command, &input.fault_plan.events, &committed)?;
        for diagnostic in &evaluation.diagnostics {
            diagnostics.insert(diagnostic.clone());
        }
        if evaluation.counts_as_commit {
            committed.insert(command.operation_id.clone());
        }
        if evaluation.decision == DENY_DECISION {
            denied.insert(command.operation_id.clone());
        }
        let event_value = simulation_event_value(SimulationEventValueInput {
            tick,
            operation_id: &command.operation_id,
            kind: &evaluation.kind,
            decision: &evaluation.decision,
            diagnostic: &evaluation.diagnostic,
            payload_ref: &command.payload_ref,
            commit_ref: &command.commit_ref,
        })?;
        let event_ref = canonical_ref(&event_value)?;
        event_outcomes.push(SimulationEventOutcome {
            tick,
            operation_id: command.operation_id.clone(),
            kind: evaluation.kind,
            decision: evaluation.decision,
            diagnostic: evaluation.diagnostic,
            event_ref,
            value: event_value,
        });
    }

    let event_refs = event_outcomes.iter().map(|outcome| outcome.event_ref.clone()).collect::<Vec<_>>();
    let committed_operation_ids = committed.into_iter().collect::<Vec<_>>();
    let denied_operation_ids = denied.into_iter().collect::<Vec<_>>();
    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let decision = if denied_operation_ids.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let final_state = final_state_value(&committed_operation_ids, &denied_operation_ids, &event_refs)?;
    let final_state_ref = canonical_ref(&final_state)?;
    let value = test_run_value(TestRunValueInput {
        decision: &decision,
        source_ref: &input.source_ref,
        test_binary_ref: &input.test_binary_ref,
        topology_ref: &topology_ref,
        seed_ref: &seed_ref,
        scheduler_ref: &scheduler_ref,
        fault_plan_ref: &fault_plan_ref,
        child_workflow_refs: &input.child_workflow_refs,
        event_refs: &event_refs,
        final_state_ref: &final_state_ref,
        replay_status: DEFAULT_REPLAY_STATUS,
        allowed_variance_refs: &input.allowed_variance_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_ref(&value)?;
    Ok(SimulationRun {
        decision,
        topology_ref,
        seed_ref,
        scheduler_ref,
        fault_plan_ref,
        source_ref: input.source_ref.clone(),
        test_binary_ref: input.test_binary_ref.clone(),
        child_workflow_refs: input.child_workflow_refs.clone(),
        event_refs,
        event_outcomes,
        committed_operation_ids,
        denied_operation_ids,
        final_state_ref,
        replay_status: DEFAULT_REPLAY_STATUS.to_string(),
        allowed_variance_refs: input.allowed_variance_refs.clone(),
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn parse_test_run(value: &IoValue) -> Result<ParsedTestRun> {
    let run = value
        .collect_simple_record("distributed-test-run-v1", Some(DISTRIBUTED_RUN_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("expected distributed-test-run-v1 receipt"))?;
    require_schema(&run[0], DISTRIBUTED_RUN_SCHEMA, "distributed test run")?;
    let decision = record_string(&run[1], "decision", "distributed test run decision")?;
    validate_decision(&decision)?;
    Ok(ParsedTestRun {
        decision,
        topology_ref: record_ref(&run[4], "topology", "distributed test run topology")?,
        seed_ref: record_ref(&run[5], "seed", "distributed test run seed")?,
        scheduler_ref: record_ref(&run[6], "scheduler", "distributed test run scheduler")?,
        fault_plan_ref: record_ref(&run[7], "fault-plan", "distributed test run fault plan")?,
        event_refs: record_ref_sequence(&run[9], "events", "distributed test run events")?,
        final_state_ref: record_ref(&run[10], "final-state", "distributed test run final state")?,
        diagnostics: record_string_sequence(&run[13], "diagnostics", "distributed test run diagnostics")?,
    })
}

struct CommandEvaluation {
    decision: String,
    kind: String,
    diagnostic: String,
    counts_as_commit: bool,
    diagnostics: Vec<String>,
}

