type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const DISTRIBUTED_TOPOLOGY_SCHEMA: &str = "molten.testing.distributed-simulation.topology.v1";
const DISTRIBUTED_SCHEDULER_SCHEMA: &str = "molten.testing.distributed-simulation.scheduler-profile.v1";
const DISTRIBUTED_SEED_SCHEMA: &str = "molten.testing.distributed-simulation.seed.v1";
const DISTRIBUTED_FAULT_PLAN_SCHEMA: &str = "molten.testing.distributed-simulation.fault-plan.v1";
const DISTRIBUTED_EVENT_SCHEMA: &str = "molten.testing.distributed-simulation.event.v1";
const DISTRIBUTED_FINAL_STATE_SCHEMA: &str = "molten.testing.distributed-simulation.final-state.v1";
const DISTRIBUTED_RUN_SCHEMA: &str = "molten.testing.distributed-test-run.v1";
const DISTRIBUTED_CI_MATRIX_SCHEMA: &str = "molten.testing.distributed-ci.matrix.v1";
const DISTRIBUTED_TEST_METADATA_SCHEMA: &str = "molten.testing.distributed-ci.metadata.v1";
const DISTRIBUTED_CI_GATE_SCHEMA: &str = "molten.testing.distributed-ci.gate.v1";

const MAX_DISTRIBUTED_PEERS: usize = 128;
const MAX_DISTRIBUTED_CHANNELS: usize = 512;
const MAX_DISTRIBUTED_ROLES: usize = 32;
const MAX_DISTRIBUTED_FAULTS: usize = 512;
const MAX_DISTRIBUTED_COMMANDS: usize = 1024;
const MAX_DISTRIBUTED_REFS: usize = 1024;
const MAX_DISTRIBUTED_TEXT: usize = 256;
const MAX_DISTRIBUTED_PROFILES: usize = 32;
const REQUIRED_DISTRIBUTED_PROFILE_COUNT: usize = 6;
const DISTRIBUTED_RUN_ARITY: usize = 15;
const DEFAULT_REPLAY_STATUS: &str = "deterministic-simulation-replayable";
const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const DUPLICATE_SUPPRESSED_DECISION: &str = "duplicate-suppressed";
const COMMIT_EVENT_KIND: &str = "semantic-commit";
const DENY_EVENT_KIND: &str = "deny-before-side-effects";
const REPLAY_EVENT_KIND: &str = "restart-replay-stable";
const DUPLICATE_EVENT_KIND: &str = "duplicate-operation-suppressed";
const FAULT_DELAY: &str = "delay";
const FAULT_DROP: &str = "drop";
const FAULT_DUPLICATE: &str = "duplicate";
const FAULT_REORDER: &str = "reorder";
const FAULT_PARTITION: &str = "partition";
const FAULT_REJOIN: &str = "rejoin";
const FAULT_CRASH: &str = "crash";
const FAULT_RESTART: &str = "restart";
const FAULT_RESOURCE_PRESSURE: &str = "resource-pressure";
const FAULT_STALE_EVIDENCE: &str = "stale-evidence";
const FAULT_AMBIENT_STATE_DRIFT: &str = "ambient-state-drift";
const FAULT_CORRUPTED_RECEIPT: &str = "corrupted-receipt";
const FAULT_UNAUTHORIZED_TRANSPORT: &str = "unauthorized-transport";
const PROFILE_FAST: &str = "fast";
const PROFILE_PROTOCOL: &str = "protocol";
const PROFILE_CLI: &str = "cli";
const PROFILE_VM_SMOKE: &str = "vm-smoke";
const PROFILE_VM_FAULT: &str = "vm-fault";
const PROFILE_SOAK: &str = "soak";
const RELEASE_REQUIRED: &str = "required";
const RELEASE_REQUIRED_WHEN_SUPPORTED: &str = "required-when-supported";
const RELEASE_PILOT_SCOPE: &str = "pilot-scope";
const COST_FAST: &str = "fast";
const COST_MEDIUM: &str = "medium";
const COST_HEAVY: &str = "heavy";
const METADATA_REQUIRED_FIELDS: usize = 10;

const _: () = assert!(MAX_DISTRIBUTED_PEERS > 0);
const _: () = assert!(MAX_DISTRIBUTED_CHANNELS >= MAX_DISTRIBUTED_PEERS);
const _: () = assert!(MAX_DISTRIBUTED_FAULTS > 0);
const _: () = assert!(MAX_DISTRIBUTED_COMMANDS > 0);
const _: () = assert!(MAX_DISTRIBUTED_REFS >= MAX_DISTRIBUTED_COMMANDS);
const _: () = assert!(MAX_DISTRIBUTED_TEXT > 0);
const _: () = assert!(MAX_DISTRIBUTED_PROFILES >= REQUIRED_DISTRIBUTED_PROFILE_COUNT);
const _: () = assert!(METADATA_REQUIRED_FIELDS >= REQUIRED_DISTRIBUTED_PROFILE_COUNT);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedPeer {
    pub id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedChannel {
    pub id: String,
    pub from_peer: String,
    pub to_peer: String,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedTopology {
    pub peers: Vec<DistributedPeer>,
    pub channels: Vec<DistributedChannel>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerProfile {
    pub id: String,
    pub policy: String,
    pub max_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationSeed {
    pub id: String,
    pub entropy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEvent {
    pub kind: String,
    pub target_kind: String,
    pub target: String,
    pub operation_id: Option<String>,
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultPlan {
    pub events: Vec<FaultEvent>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationCommand {
    pub operation_id: String,
    pub from_peer: String,
    pub to_peer: String,
    pub payload_ref: String,
    pub commit_ref: String,
    pub authority_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub resource_ref: Option<String>,
    pub transport_ref: Option<String>,
    pub requires_authority: bool,
    pub requires_quorum: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedSimulationInput {
    pub topology: DistributedTopology,
    pub scheduler: SchedulerProfile,
    pub seed: SimulationSeed,
    pub fault_plan: FaultPlan,
    pub source_ref: String,
    pub test_binary_ref: String,
    pub commands: Vec<SimulationCommand>,
    pub child_workflow_refs: Vec<String>,
    pub allowed_variance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationEventOutcome {
    pub tick: u64,
    pub operation_id: String,
    pub kind: String,
    pub decision: String,
    pub diagnostic: String,
    pub event_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedSimulationRun {
    pub decision: String,
    pub topology_ref: String,
    pub seed_ref: String,
    pub scheduler_ref: String,
    pub fault_plan_ref: String,
    pub source_ref: String,
    pub test_binary_ref: String,
    pub child_workflow_refs: Vec<String>,
    pub event_refs: Vec<String>,
    pub committed_operation_ids: Vec<String>,
    pub denied_operation_ids: Vec<String>,
    pub final_state_ref: String,
    pub replay_status: String,
    pub allowed_variance_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDistributedTestRun {
    pub decision: String,
    pub topology_ref: String,
    pub seed_ref: String,
    pub scheduler_ref: String,
    pub fault_plan_ref: String,
    pub event_refs: Vec<String>,
    pub final_state_ref: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedCiProfile {
    pub id: String,
    pub purpose: String,
    pub command: String,
    pub expected_artifact_kinds: Vec<String>,
    pub evidence_scope: String,
    pub cost_class: String,
    pub release_review_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedCiMatrix {
    pub decision: String,
    pub profiles: Vec<DistributedCiProfile>,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedTestMetadataInput {
    pub source_ref: String,
    pub nix_input_refs: Vec<String>,
    pub test_binary_ref: String,
    pub profile_id: String,
    pub shard_id: String,
    pub seed_ref: String,
    pub topology_ref: String,
    pub fault_plan_ref: String,
    pub receipt_refs: Vec<String>,
    pub variance_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedTestMetadata {
    pub profile_id: String,
    pub shard_id: String,
    pub metadata_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedProfileRun {
    pub profile_id: String,
    pub decision: String,
    pub metadata_ref: String,
    pub traceability_ref: String,
    pub positive_coverage: bool,
    pub negative_coverage: bool,
    pub retry_attempts: u64,
    pub unavailable: bool,
    pub unsupported_reason: Option<String>,
    pub variance_declared: bool,
    pub required_for_release: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedCiGateInput<'a> {
    pub matrix: &'a DistributedCiMatrix,
    pub metadata: &'a [DistributedTestMetadata],
    pub traceability_manifest: &'a crate::trace_core::TraceabilityManifest,
    pub runs: &'a [DistributedProfileRun],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedCiGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub traceability_ref: String,
    pub metadata_refs: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

pub fn distributed_topology_value(topology: &DistributedTopology) -> Result<IoValue> {
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

pub fn run_distributed_simulation(input: &DistributedSimulationInput) -> Result<DistributedSimulationRun> {
    validate_simulation_input(input)?;
    let topology_ref = canonical_ref(&distributed_topology_value(&input.topology)?)?;
    let seed_ref = canonical_ref(&seed_value(&input.seed)?)?;
    let scheduler_ref = canonical_ref(&scheduler_profile_value(&input.scheduler)?)?;
    let fault_plan_ref = canonical_ref(&fault_plan_value(&input.fault_plan)?)?;
    let mut events = Vec::with_capacity(input.commands.len());
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
        events.push(simulation_event_value(SimulationEventValueInput {
            tick,
            operation_id: &command.operation_id,
            kind: &evaluation.kind,
            decision: &evaluation.decision,
            diagnostic: &evaluation.diagnostic,
            payload_ref: &command.payload_ref,
            commit_ref: &command.commit_ref,
        })?);
    }

    let event_refs = canonical_refs(&events)?;
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
    let value = distributed_test_run_value(DistributedTestRunValueInput {
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
    Ok(DistributedSimulationRun {
        decision,
        topology_ref,
        seed_ref,
        scheduler_ref,
        fault_plan_ref,
        source_ref: input.source_ref.clone(),
        test_binary_ref: input.test_binary_ref.clone(),
        child_workflow_refs: input.child_workflow_refs.clone(),
        event_refs,
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

pub fn parse_distributed_test_run(value: &IoValue) -> Result<ParsedDistributedTestRun> {
    let run = value
        .collect_simple_record("distributed-test-run-v1", Some(DISTRIBUTED_RUN_ARITY))
        .ok_or_else(|| MoltenError::invalid_harness("expected distributed-test-run-v1 receipt"))?;
    require_schema(&run[0], DISTRIBUTED_RUN_SCHEMA, "distributed test run")?;
    let decision = record_string(&run[1], "decision", "distributed test run decision")?;
    validate_decision(&decision)?;
    Ok(ParsedDistributedTestRun {
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

struct SimulationEventValueInput<'a> {
    tick: u64,
    operation_id: &'a str,
    kind: &'a str,
    decision: &'a str,
    diagnostic: &'a str,
    payload_ref: &'a str,
    commit_ref: &'a str,
}

struct DistributedTestRunValueInput<'a> {
    decision: &'a str,
    source_ref: &'a str,
    test_binary_ref: &'a str,
    topology_ref: &'a str,
    seed_ref: &'a str,
    scheduler_ref: &'a str,
    fault_plan_ref: &'a str,
    child_workflow_refs: &'a [String],
    event_refs: &'a [String],
    final_state_ref: &'a str,
    replay_status: &'a str,
    allowed_variance_refs: &'a [String],
    diagnostics: &'a [String],
}

fn evaluate_command(
    command: &SimulationCommand,
    faults: &[FaultEvent],
    committed: &OrderedSet<String>,
) -> Result<CommandEvaluation> {
    let active = active_faults(command, faults);
    if committed.contains(&command.operation_id) || has_fault_kind(&active, FAULT_DUPLICATE) {
        return Ok(CommandEvaluation {
            decision: PASS_DECISION.to_string(),
            kind: DUPLICATE_EVENT_KIND.to_string(),
            diagnostic: DUPLICATE_SUPPRESSED_DECISION.to_string(),
            counts_as_commit: false,
            diagnostics: vec![DUPLICATE_SUPPRESSED_DECISION.to_string()],
        });
    }
    if has_fault_kind(&active, FAULT_AMBIENT_STATE_DRIFT) {
        return deny_evaluation("undeclared-ambient-state");
    }
    if has_fault_kind(&active, FAULT_UNAUTHORIZED_TRANSPORT)
        || (command.transport_ref.is_some() && command.authority_ref.is_none())
    {
        return deny_evaluation("transport-evidence-does-not-grant-authority");
    }
    if command.requires_authority && command.authority_ref.is_none() {
        return deny_evaluation("missing-authority");
    }
    if command.requires_authority && command.policy_ref.is_none() {
        return deny_evaluation("missing-policy");
    }
    if command.requires_authority && command.resource_ref.is_none() {
        return deny_evaluation("missing-resource");
    }
    if command.requires_quorum && has_fault_kind(&active, FAULT_PARTITION) {
        return deny_evaluation("partitioned-quorum-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_STALE_EVIDENCE) {
        return deny_evaluation("stale-evidence-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_CORRUPTED_RECEIPT) {
        return deny_evaluation("corrupted-receipt-denied-before-side-effects");
    }
    if has_fault_kind(&active, FAULT_RESOURCE_PRESSURE) {
        return deny_evaluation("resource-pressure-denied-before-side-effects");
    }
    let diagnostics = benign_fault_diagnostics(&active);
    let kind = if has_fault_kind(&active, FAULT_CRASH) || has_fault_kind(&active, FAULT_RESTART) {
        REPLAY_EVENT_KIND
    } else {
        COMMIT_EVENT_KIND
    };
    Ok(CommandEvaluation {
        decision: PASS_DECISION.to_string(),
        kind: kind.to_string(),
        diagnostic: diagnostics.first().cloned().unwrap_or_else(|| "semantic-commit-accepted".to_string()),
        counts_as_commit: true,
        diagnostics,
    })
}

fn deny_evaluation(diagnostic: &str) -> Result<CommandEvaluation> {
    validate_text("deny diagnostic", diagnostic)?;
    Ok(CommandEvaluation {
        decision: DENY_DECISION.to_string(),
        kind: DENY_EVENT_KIND.to_string(),
        diagnostic: diagnostic.to_string(),
        counts_as_commit: false,
        diagnostics: vec![diagnostic.to_string()],
    })
}

fn active_faults<'a>(command: &SimulationCommand, faults: &'a [FaultEvent]) -> Vec<&'a FaultEvent> {
    faults
        .iter()
        .filter(|fault| {
            fault.operation_id.as_deref() == Some(command.operation_id.as_str())
                || fault.target == command.from_peer
                || fault.target == command.to_peer
                || fault.target == command.operation_id
        })
        .collect()
}

fn has_fault_kind(faults: &[&FaultEvent], kind: &str) -> bool {
    faults.iter().any(|fault| fault.kind == kind)
}

fn benign_fault_diagnostics(faults: &[&FaultEvent]) -> Vec<String> {
    faults
        .iter()
        .filter(|fault| {
            matches!(
                fault.kind.as_str(),
                FAULT_DELAY | FAULT_DROP | FAULT_REORDER | FAULT_REJOIN | FAULT_CRASH | FAULT_RESTART
            )
        })
        .map(|fault| format!("{}:{}", fault.kind, fault.diagnostic))
        .collect()
}

fn distributed_test_run_value(input: DistributedTestRunValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_ref(input.source_ref, "distributed source")?;
    validate_ref(input.test_binary_ref, "distributed test binary")?;
    validate_ref(input.topology_ref, "distributed topology")?;
    validate_ref(input.seed_ref, "distributed seed")?;
    validate_ref(input.scheduler_ref, "distributed scheduler")?;
    validate_ref(input.fault_plan_ref, "distributed fault plan")?;
    validate_ref_slice("distributed child workflow", input.child_workflow_refs)?;
    validate_ref_slice("distributed event", input.event_refs)?;
    validate_ref(input.final_state_ref, "distributed final state")?;
    validate_text("distributed replay status", input.replay_status)?;
    validate_ref_slice("distributed allowed variance", input.allowed_variance_refs)?;
    validate_strings("distributed diagnostic", input.diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-test-run-v1", vec![
        string(DISTRIBUTED_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("source", vec![string(input.source_ref)]),
        record("test-binary", vec![string(input.test_binary_ref)]),
        record("topology", vec![string(input.topology_ref)]),
        record("seed", vec![string(input.seed_ref)]),
        record("scheduler", vec![string(input.scheduler_ref)]),
        record("fault-plan", vec![string(input.fault_plan_ref)]),
        record("child-workflows", vec![refs_sequence(input.child_workflow_refs)]),
        record("events", vec![refs_sequence(input.event_refs)]),
        record("final-state", vec![string(input.final_state_ref)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("allowed-variance", vec![refs_sequence(input.allowed_variance_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("topology-bound", PASS_DECISION),
            ("fault-plan-bound", PASS_DECISION),
            ("simulation-does-not-grant-authority", PASS_DECISION),
            ("diagnostic-logs-not-authority", PASS_DECISION),
        ]),
    ]))
}

fn simulation_event_value(input: SimulationEventValueInput<'_>) -> Result<IoValue> {
    validate_text("simulation operation", input.operation_id)?;
    validate_text("simulation event kind", input.kind)?;
    validate_event_decision(input.decision)?;
    validate_text("simulation diagnostic", input.diagnostic)?;
    validate_ref(input.payload_ref, "simulation payload")?;
    validate_ref(input.commit_ref, "simulation commit")?;
    Ok(record("distributed-simulation-event-v1", vec![
        string(DISTRIBUTED_EVENT_SCHEMA),
        record("tick", vec![u64_value(input.tick)]),
        record("operation", vec![string(input.operation_id)]),
        record("kind", vec![string(input.kind)]),
        record("decision", vec![string(input.decision)]),
        record("diagnostic", vec![string(input.diagnostic)]),
        record("payload", vec![string(input.payload_ref)]),
        record("commit", vec![string(input.commit_ref)]),
    ]))
}

fn final_state_value(committed: &[String], denied: &[String], event_refs: &[String]) -> Result<IoValue> {
    validate_strings("committed operation", committed, MAX_DISTRIBUTED_COMMANDS)?;
    validate_strings("denied operation", denied, MAX_DISTRIBUTED_COMMANDS)?;
    validate_ref_slice("final-state event", event_refs)?;
    Ok(record("distributed-simulation-final-state-v1", vec![
        string(DISTRIBUTED_FINAL_STATE_SCHEMA),
        record("committed-operations", vec![sequence(committed.iter().map(string).collect())]),
        record("denied-operations", vec![sequence(denied.iter().map(string).collect())]),
        record("events", vec![refs_sequence(event_refs)]),
    ]))
}

fn validate_simulation_input(input: &DistributedSimulationInput) -> Result<()> {
    validate_topology(&input.topology)?;
    scheduler_profile_value(&input.scheduler)?;
    seed_value(&input.seed)?;
    validate_fault_plan(&input.fault_plan)?;
    validate_ref(&input.source_ref, "distributed simulation source")?;
    validate_ref(&input.test_binary_ref, "distributed simulation test binary")?;
    validate_ref_slice("distributed child workflow", &input.child_workflow_refs)?;
    validate_ref_slice("distributed allowed variance", &input.allowed_variance_refs)?;
    validate_commands(&input.topology, &input.commands)
}

fn validate_topology(topology: &DistributedTopology) -> Result<()> {
    if topology.peers.is_empty() {
        return Err(MoltenError::invalid_harness("distributed topology requires peers"));
    }
    ensure_count_at_most(topology.peers.len(), MAX_DISTRIBUTED_PEERS, "distributed peers")?;
    ensure_count_at_most(topology.channels.len(), MAX_DISTRIBUTED_CHANNELS, "distributed channels")?;
    validate_strings("topology caveat", &topology.caveats, MAX_DISTRIBUTED_TEXT)?;
    let mut peers = OrderedSet::new();
    for peer in &topology.peers {
        validate_text("distributed peer", &peer.id)?;
        ensure_count_at_most(peer.roles.len(), MAX_DISTRIBUTED_ROLES, "distributed peer roles")?;
        validate_strings("distributed peer role", &peer.roles, MAX_DISTRIBUTED_TEXT)?;
        if !peers.insert(peer.id.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate distributed peer {}", peer.id)));
        }
    }
    let mut channels = OrderedSet::new();
    for channel in &topology.channels {
        validate_text("distributed channel", &channel.id)?;
        validate_text("distributed channel topic", &channel.topic)?;
        if !peers.contains(channel.from_peer.as_str()) || !peers.contains(channel.to_peer.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "distributed channel {} references peer outside topology",
                channel.id
            )));
        }
        if !channels.insert(channel.id.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate distributed channel {}", channel.id)));
        }
    }
    Ok(())
}

fn validate_fault_plan(plan: &FaultPlan) -> Result<()> {
    ensure_count_at_most(plan.events.len(), MAX_DISTRIBUTED_FAULTS, "distributed faults")?;
    validate_strings("fault-plan caveat", &plan.caveats, MAX_DISTRIBUTED_TEXT)?;
    for event in &plan.events {
        validate_fault_kind(&event.kind)?;
        validate_text("fault target kind", &event.target_kind)?;
        validate_text("fault target", &event.target)?;
        validate_text("fault diagnostic", &event.diagnostic)?;
        if let Some(operation_id) = &event.operation_id {
            validate_text("fault operation", operation_id)?;
        }
    }
    Ok(())
}

fn validate_commands(topology: &DistributedTopology, commands: &[SimulationCommand]) -> Result<()> {
    if commands.is_empty() {
        return Err(MoltenError::invalid_harness("distributed simulation requires commands"));
    }
    ensure_count_at_most(commands.len(), MAX_DISTRIBUTED_COMMANDS, "distributed commands")?;
    let peers = topology.peers.iter().map(|peer| peer.id.as_str()).collect::<OrderedSet<_>>();
    for command in commands {
        validate_text("simulation operation", &command.operation_id)?;
        if !peers.contains(command.from_peer.as_str()) || !peers.contains(command.to_peer.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "simulation command {} references peer outside topology",
                command.operation_id
            )));
        }
        validate_ref(&command.payload_ref, "simulation payload")?;
        validate_ref(&command.commit_ref, "simulation commit")?;
        validate_optional_ref(command.authority_ref.as_deref(), "simulation authority")?;
        validate_optional_ref(command.policy_ref.as_deref(), "simulation policy")?;
        validate_optional_ref(command.resource_ref.as_deref(), "simulation resource")?;
        validate_optional_ref(command.transport_ref.as_deref(), "simulation transport")?;
    }
    Ok(())
}

fn validate_fault_kind(kind: &str) -> Result<()> {
    match kind {
        FAULT_DELAY
        | FAULT_DROP
        | FAULT_DUPLICATE
        | FAULT_REORDER
        | FAULT_PARTITION
        | FAULT_REJOIN
        | FAULT_CRASH
        | FAULT_RESTART
        | FAULT_RESOURCE_PRESSURE
        | FAULT_STALE_EVIDENCE
        | FAULT_AMBIENT_STATE_DRIFT
        | FAULT_CORRUPTED_RECEIPT
        | FAULT_UNAUTHORIZED_TRANSPORT => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed fault kind {other}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        PASS_DECISION | DENY_DECISION => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed decision {other}"))),
    }
}

fn validate_event_decision(decision: &str) -> Result<()> {
    match decision {
        PASS_DECISION | DENY_DECISION => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported simulation event decision {other}"))),
    }
}

fn command_tick(index: usize) -> Result<u64> {
    u64::try_from(index).map_err(|_| MoltenError::invalid_harness("simulation command index exceeds u64"))
}

fn peer_values(peers: &[DistributedPeer]) -> Result<Vec<IoValue>> {
    peers
        .iter()
        .map(|peer| {
            Ok(record("peer", vec![
                record("id", vec![string(&peer.id)]),
                record("roles", vec![sequence(peer.roles.iter().map(string).collect())]),
            ]))
        })
        .collect()
}

fn channel_values(channels: &[DistributedChannel]) -> Result<Vec<IoValue>> {
    channels
        .iter()
        .map(|channel| {
            Ok(record("channel", vec![
                record("id", vec![string(&channel.id)]),
                record("from", vec![string(&channel.from_peer)]),
                record("to", vec![string(&channel.to_peer)]),
                record("topic", vec![string(&channel.topic)]),
            ]))
        })
        .collect()
}

fn fault_event_values(events: &[FaultEvent]) -> Result<Vec<IoValue>> {
    events
        .iter()
        .map(|event| {
            Ok(record("fault-event", vec![
                record("kind", vec![string(&event.kind)]),
                record("target-kind", vec![string(&event.target_kind)]),
                record("target", vec![string(&event.target)]),
                record("operation", vec![optional_string_value(event.operation_id.as_deref())]),
                record("start-tick", vec![u64_value(event.start_tick)]),
                record("duration-ticks", vec![u64_value(event.duration_ticks)]),
                record("diagnostic", vec![string(&event.diagnostic)]),
            ]))
        })
        .collect()
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_optional_ref(reference: Option<&str>, label: &str) -> Result<()> {
    if let Some(reference) = reference {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_DISTRIBUTED_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    Ok(())
}

fn validate_strings(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds bound {maximum}")))
    }
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IoValue>> {
    validate_strings(label, values, maximum)?;
    Ok(values.iter().map(string).collect())
}

fn record_string(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<String> {
    let field = value_to_iovalue(value);
    let record = field
        .collect_simple_record(record_name, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {record_name} for {context}")))?;
    record[0]
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {context}")))
}

fn record_ref(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<String> {
    let reference = record_string(value, record_name, context)?;
    validate_ref(&reference, context)?;
    Ok(reference)
}

fn record_string_sequence(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<Vec<String>> {
    let field = value_to_iovalue(value);
    let record = field
        .collect_simple_record(record_name, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {record_name} for {context}")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {context}")))?;
    let mut output = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        output.push(
            item.as_string()
                .map(|value| value.to_string())
                .ok_or_else(|| MoltenError::invalid_harness(format!("expected string sequence item for {context}")))?,
        );
    }
    Ok(output)
}

fn record_ref_sequence(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<Vec<String>> {
    let refs = record_string_sequence(value, record_name, context)?;
    validate_ref_slice(context, &refs)?;
    Ok(refs)
}

fn require_schema(value: &preserves::Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected schema string for {context}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn value_to_iovalue(value: &preserves::Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn canonical_ref(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn canonical_refs(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(canonical_ref(value)?);
    }
    Ok(refs)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

pub fn default_distributed_ci_profiles() -> Vec<DistributedCiProfile> {
    vec![
        DistributedCiProfile {
            id: PROFILE_FAST.to_string(),
            purpose: "pure core, unit, parser, and receipt validation checks".to_string(),
            command: "cargo nextest run --profile deterministic".to_string(),
            expected_artifact_kinds: vec!["libtest".to_string(), "junit".to_string()],
            evidence_scope: "no platform or transport claims".to_string(),
            cost_class: COST_FAST.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        DistributedCiProfile {
            id: PROFILE_PROTOCOL.to_string(),
            purpose: "deterministic simulation, model fixtures, and drift checks".to_string(),
            command: "cargo test --lib distributed".to_string(),
            expected_artifact_kinds: vec![
                "distributed-test-run-v1".to_string(),
                "distributed-fault-plan-v1".to_string(),
            ],
            evidence_scope: "simulated distributed invariants".to_string(),
            cost_class: COST_FAST.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        DistributedCiProfile {
            id: PROFILE_CLI.to_string(),
            purpose: "CLI receipt and traceability workflow checks".to_string(),
            command: "nix build .#checks.x86_64-linux.requirement-traceability-gate".to_string(),
            expected_artifact_kinds: vec!["requirement-traceability-gate-v1".to_string()],
            evidence_scope: "local process and receipt behavior".to_string(),
            cost_class: COST_MEDIUM.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        DistributedCiProfile {
            id: PROFILE_VM_SMOKE.to_string(),
            purpose: "two-node NixOS platform smoke topology".to_string(),
            command: "nix build .#checks.x86_64-linux.nixos-vm-multinode".to_string(),
            expected_artifact_kinds: vec![
                "nixos-vm-test-run-v1".to_string(),
                "nixos-vm-evidence-validation-v1".to_string(),
            ],
            evidence_scope: "platform integration smoke evidence".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_REQUIRED_WHEN_SUPPORTED.to_string(),
        },
        DistributedCiProfile {
            id: PROFILE_VM_FAULT.to_string(),
            purpose: "executable VM network, restart, and state-root fault checks".to_string(),
            command: "nix build .#checks.x86_64-linux.nixos-vm-multinode".to_string(),
            expected_artifact_kinds: vec!["nixos-vm-fault-receipt-v1".to_string()],
            evidence_scope: "bounded executable platform fault evidence".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_REQUIRED_WHEN_SUPPORTED.to_string(),
        },
        DistributedCiProfile {
            id: PROFILE_SOAK.to_string(),
            purpose: "dogfood, pilot, and production-shaped evidence review".to_string(),
            command: "nix build .#checks.x86_64-linux.dogfood-local-node".to_string(),
            expected_artifact_kinds: vec![
                "prod-soak-run-v1".to_string(),
                "operator-release-gate-receipt-v1".to_string(),
            ],
            evidence_scope: "constrained pilot/readiness review only".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_PILOT_SCOPE.to_string(),
        },
    ]
}

pub fn build_distributed_ci_matrix(profiles: Vec<DistributedCiProfile>) -> Result<DistributedCiMatrix> {
    ensure_count_at_most(profiles.len(), MAX_DISTRIBUTED_PROFILES, "distributed CI profiles")?;
    let diagnostics = matrix_diagnostics(&profiles)?;
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = distributed_ci_matrix_value(&decision, &profiles, &diagnostics)?;
    let matrix_ref = canonical_ref(&value)?;
    Ok(DistributedCiMatrix {
        decision,
        profiles,
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_distributed_test_metadata(input: &DistributedTestMetadataInput) -> Result<DistributedTestMetadata> {
    validate_metadata_input(input)?;
    let value = distributed_test_metadata_value(input)?;
    let metadata_ref = canonical_ref(&value)?;
    Ok(DistributedTestMetadata {
        profile_id: input.profile_id.clone(),
        shard_id: input.shard_id.clone(),
        metadata_ref,
        value,
    })
}

pub fn evaluate_distributed_ci_gate(input: &DistributedCiGateInput<'_>) -> Result<DistributedCiGate> {
    let mut diagnostics = gate_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let metadata_refs = input.metadata.iter().map(|metadata| metadata.metadata_ref.clone()).collect::<Vec<_>>();
    let value = distributed_ci_gate_value(
        &decision,
        &input.matrix.matrix_ref,
        &input.traceability_manifest.manifest_ref,
        &metadata_refs,
        &diagnostics,
    )?;
    let gate_ref = canonical_ref(&value)?;
    Ok(DistributedCiGate {
        decision,
        diagnostics,
        matrix_ref: input.matrix.matrix_ref.clone(),
        traceability_ref: input.traceability_manifest.manifest_ref.clone(),
        metadata_refs,
        gate_ref,
        value,
    })
}

fn matrix_diagnostics(profiles: &[DistributedCiProfile]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let mut ids = OrderedSet::new();
    for profile in profiles {
        validate_profile(profile, &mut diagnostics)?;
        if !ids.insert(profile.id.as_str()) {
            diagnostics.push(format!("duplicate-profile:{}", profile.id));
        }
    }
    for required in required_profile_ids() {
        if !ids.contains(required) {
            diagnostics.push(format!("missing-profile:{required}"));
        }
    }
    Ok(diagnostics)
}

fn validate_profile(profile: &DistributedCiProfile, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_text("distributed profile id", &profile.id)?;
    validate_text("distributed profile purpose", &profile.purpose)?;
    validate_text("distributed profile command", &profile.command)?;
    validate_text("distributed profile evidence scope", &profile.evidence_scope)?;
    validate_cost_class(&profile.cost_class)?;
    validate_release_status(&profile.release_review_status)?;
    if profile.expected_artifact_kinds.is_empty() {
        diagnostics.push(format!("profile-missing-artifact-kind:{}", profile.id));
    }
    validate_strings("distributed profile artifact kind", &profile.expected_artifact_kinds, MAX_DISTRIBUTED_TEXT)
}

fn validate_cost_class(value: &str) -> Result<()> {
    match value {
        COST_FAST | COST_MEDIUM | COST_HEAVY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed profile cost class {other}"))),
    }
}

fn validate_release_status(value: &str) -> Result<()> {
    match value {
        RELEASE_REQUIRED | RELEASE_REQUIRED_WHEN_SUPPORTED | RELEASE_PILOT_SCOPE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed profile release status {other}"))),
    }
}

fn required_profile_ids() -> [&'static str; REQUIRED_DISTRIBUTED_PROFILE_COUNT] {
    [
        PROFILE_FAST,
        PROFILE_PROTOCOL,
        PROFILE_CLI,
        PROFILE_VM_SMOKE,
        PROFILE_VM_FAULT,
        PROFILE_SOAK,
    ]
}

fn distributed_ci_matrix_value(
    decision: &str,
    profiles: &[DistributedCiProfile],
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_strings("distributed matrix diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-ci-matrix-v1", vec![
        string(DISTRIBUTED_CI_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profiles", vec![sequence(profile_values(profiles)?)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("profiles-explicit", status(profiles.len() == REQUIRED_DISTRIBUTED_PROFILE_COUNT)),
            ("artifact-kinds-declared", status(diagnostics.iter().all(|item| !item.contains("artifact-kind")))),
            ("retry-success-not-pass-evidence", PASS_DECISION),
            ("unavailable-is-not-pass", PASS_DECISION),
        ]),
    ]))
}

fn profile_values(profiles: &[DistributedCiProfile]) -> Result<Vec<IoValue>> {
    profiles
        .iter()
        .map(|profile| {
            Ok(record("profile", vec![
                record("id", vec![string(&profile.id)]),
                record("purpose", vec![string(&profile.purpose)]),
                record("command", vec![string(&profile.command)]),
                record("artifact-kinds", vec![sequence(profile.expected_artifact_kinds.iter().map(string).collect())]),
                record("evidence-scope", vec![string(&profile.evidence_scope)]),
                record("cost-class", vec![string(&profile.cost_class)]),
                record("release-review-status", vec![string(&profile.release_review_status)]),
            ]))
        })
        .collect()
}

fn validate_metadata_input(input: &DistributedTestMetadataInput) -> Result<()> {
    validate_ref(&input.source_ref, "distributed metadata source")?;
    validate_ref_slice("distributed metadata nix input", &input.nix_input_refs)?;
    validate_ref(&input.test_binary_ref, "distributed metadata test binary")?;
    validate_text("distributed metadata profile", &input.profile_id)?;
    validate_text("distributed metadata shard", &input.shard_id)?;
    validate_ref(&input.seed_ref, "distributed metadata seed")?;
    validate_ref(&input.topology_ref, "distributed metadata topology")?;
    validate_ref(&input.fault_plan_ref, "distributed metadata fault plan")?;
    validate_ref_slice("distributed metadata receipt", &input.receipt_refs)?;
    validate_ref_slice("distributed metadata variance", &input.variance_refs)?;
    validate_ref_slice("distributed metadata diagnostic log", &input.diagnostic_log_refs)?;
    if input.receipt_refs.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires receipt refs"));
    }
    if input.variance_refs.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires variance refs"));
    }
    Ok(())
}

fn distributed_test_metadata_value(input: &DistributedTestMetadataInput) -> Result<IoValue> {
    Ok(record("distributed-test-metadata-v1", vec![
        string(DISTRIBUTED_TEST_METADATA_SCHEMA),
        record("source", vec![string(&input.source_ref)]),
        record("nix-inputs", vec![refs_sequence(&input.nix_input_refs)]),
        record("test-binary", vec![string(&input.test_binary_ref)]),
        record("profile", vec![string(&input.profile_id)]),
        record("shard", vec![string(&input.shard_id)]),
        record("seed", vec![string(&input.seed_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("receipts", vec![refs_sequence(&input.receipt_refs)]),
        record("variance", vec![refs_sequence(&input.variance_refs)]),
        record("diagnostic-logs", vec![refs_sequence(&input.diagnostic_log_refs)]),
        checks_value(&[
            ("source-bound", PASS_DECISION),
            ("profile-and-shard-bound", PASS_DECISION),
            ("variance-declared", PASS_DECISION),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

fn gate_diagnostics(input: &DistributedCiGateInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.matrix.decision != PASS_DECISION {
        diagnostics.push("distributed-matrix-denied".to_string());
    }
    if input.traceability_manifest.decision != PASS_DECISION {
        diagnostics.push("distributed-traceability-denied".to_string());
    }
    let metadata_refs = input.metadata.iter().map(|metadata| metadata.metadata_ref.as_str()).collect::<OrderedSet<_>>();
    let profile_ids = input.matrix.profiles.iter().map(|profile| profile.id.as_str()).collect::<OrderedSet<_>>();
    for run in input.runs {
        validate_profile_run(run)?;
        if !profile_ids.contains(run.profile_id.as_str()) {
            diagnostics.push(format!("run-profile-not-in-matrix:{}", run.profile_id));
        }
        if !metadata_refs.contains(run.metadata_ref.as_str()) {
            diagnostics.push(format!("run-metadata-ref-missing:{}", run.profile_id));
        }
        if run.traceability_ref != input.traceability_manifest.manifest_ref {
            diagnostics.push(format!("run-traceability-ref-mismatch:{}", run.profile_id));
        }
        if !run.positive_coverage {
            diagnostics.push(format!("missing-positive-coverage:{}", run.profile_id));
        }
        if !run.negative_coverage {
            diagnostics.push(format!("missing-negative-coverage:{}", run.profile_id));
        }
        if run.retry_attempts > 0 && run.decision == PASS_DECISION {
            diagnostics.push(format!("retry-only-success-denied:{}", run.profile_id));
        }
        if run.unavailable && run.decision == PASS_DECISION {
            diagnostics.push(format!("unavailable-profile-cannot-pass:{}", run.profile_id));
        }
        if run.unavailable && run.required_for_release {
            diagnostics.push(format!("required-profile-unavailable:{}", run.profile_id));
        }
        if !run.variance_declared {
            diagnostics.push(format!("undeclared-variance:{}", run.profile_id));
        }
    }
    for profile in &input.matrix.profiles {
        if profile.release_review_status == RELEASE_REQUIRED
            && !input.runs.iter().any(|run| run.profile_id == profile.id)
        {
            diagnostics.push(format!("missing-required-profile-run:{}", profile.id));
        }
    }
    Ok(diagnostics)
}

fn validate_profile_run(run: &DistributedProfileRun) -> Result<()> {
    validate_text("profile run id", &run.profile_id)?;
    validate_decision(&run.decision)?;
    validate_ref(&run.metadata_ref, "profile run metadata")?;
    validate_ref(&run.traceability_ref, "profile run traceability")?;
    if let Some(reason) = &run.unsupported_reason {
        validate_text("profile run unsupported reason", reason)?;
    }
    Ok(())
}

fn distributed_ci_gate_value(
    decision: &str,
    matrix_ref: &str,
    traceability_ref: &str,
    metadata_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_ref(matrix_ref, "distributed CI matrix")?;
    validate_ref(traceability_ref, "distributed traceability")?;
    validate_ref_slice("distributed metadata", metadata_refs)?;
    validate_strings("distributed CI gate diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-ci-gate-v1", vec![
        string(DISTRIBUTED_CI_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("matrix", vec![string(matrix_ref)]),
        record("traceability", vec![string(traceability_ref)]),
        record("metadata", vec![refs_sequence(metadata_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("positive-coverage-required", status(!diagnostics.iter().any(|item| item.contains("positive")))),
            ("negative-coverage-required", status(!diagnostics.iter().any(|item| item.contains("negative")))),
            ("zero-retry-pass-required", status(!diagnostics.iter().any(|item| item.contains("retry")))),
            ("unavailable-not-pass", status(!diagnostics.iter().any(|item| item.contains("unavailable")))),
        ]),
    ]))
}

fn status(condition: bool) -> &'static str {
    if condition { PASS_DECISION } else { "fail" }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMULATION_MAX_TICKS: u64 = 32;
    const FAULT_START_TICK: u64 = 1;
    const FAULT_DURATION_TICKS: u64 = 2;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn topology() -> DistributedTopology {
        DistributedTopology {
            peers: vec![
                DistributedPeer {
                    id: "peer-a".to_string(),
                    roles: vec!["sender".to_string()],
                },
                DistributedPeer {
                    id: "peer-b".to_string(),
                    roles: vec!["receiver".to_string()],
                },
            ],
            channels: vec![DistributedChannel {
                id: "a-to-b".to_string(),
                from_peer: "peer-a".to_string(),
                to_peer: "peer-b".to_string(),
                topic: "node-control".to_string(),
            }],
            caveats: vec!["simulation evidence is review evidence only".to_string()],
        }
    }

    fn scheduler() -> SchedulerProfile {
        SchedulerProfile {
            id: "round-robin".to_string(),
            policy: "deterministic-virtual-clock".to_string(),
            max_ticks: SIMULATION_MAX_TICKS,
        }
    }

    fn seed() -> SimulationSeed {
        SimulationSeed {
            id: "seed-1".to_string(),
            entropy_ref: local_ref("seed-1"),
        }
    }

    fn command(operation_id: &str) -> SimulationCommand {
        SimulationCommand {
            operation_id: operation_id.to_string(),
            from_peer: "peer-a".to_string(),
            to_peer: "peer-b".to_string(),
            payload_ref: local_ref(&format!("payload:{operation_id}")),
            commit_ref: local_ref(&format!("commit:{operation_id}")),
            authority_ref: Some(local_ref("authority")),
            policy_ref: Some(local_ref("policy")),
            resource_ref: Some(local_ref("resource")),
            transport_ref: Some(local_ref("transport")),
            requires_authority: true,
            requires_quorum: false,
        }
    }

    fn input_with(plan: FaultPlan, commands: Vec<SimulationCommand>) -> DistributedSimulationInput {
        DistributedSimulationInput {
            topology: topology(),
            scheduler: scheduler(),
            seed: seed(),
            fault_plan: plan,
            source_ref: local_ref("source-tree"),
            test_binary_ref: local_ref("test-binary"),
            commands,
            child_workflow_refs: vec![local_ref("child-workflow")],
            allowed_variance_refs: vec![local_ref("variance:none")],
        }
    }

    fn fault(kind: &str, operation_id: &str, diagnostic: &str) -> FaultEvent {
        FaultEvent {
            kind: kind.to_string(),
            target_kind: "operation".to_string(),
            target: operation_id.to_string(),
            operation_id: Some(operation_id.to_string()),
            start_tick: FAULT_START_TICK,
            duration_ticks: FAULT_DURATION_TICKS,
            diagnostic: diagnostic.to_string(),
        }
    }

    #[test]
    fn distributed_simulation_is_deterministic_and_parseable() {
        // r[verify molten.testing.distributed_simulation.fault_plan_schema]
        // r[verify molten.testing.distributed_simulation.simulator_core]
        // r[verify molten.testing.distributed_simulation.run_receipts]
        let input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_DELAY, "op-1", "bounded-delay")],
                caveats: vec!["delay is virtual".to_string()],
            },
            vec![command("op-1")],
        );

        let first = run_distributed_simulation(&input).expect("first run");
        let second = run_distributed_simulation(&input).expect("second run");
        let parsed = parse_distributed_test_run(&first.value).expect("parse run");

        assert_eq!(first.decision, PASS_DECISION);
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.final_state_ref, second.final_state_ref);
        assert_eq!(parsed.topology_ref, first.topology_ref);
        assert_eq!(parsed.event_refs, first.event_refs);
    }

    #[test]
    fn changing_fault_plan_changes_identity() {
        // r[verify molten.testing.distributed_simulation.fault_plan_schema]
        let base = FaultPlan {
            events: vec![fault(FAULT_DROP, "op-1", "drop-one")],
            caveats: Vec::new(),
        };
        let changed = FaultPlan {
            events: vec![fault(FAULT_REORDER, "op-1", "reorder-one")],
            caveats: Vec::new(),
        };
        let base_ref = canonical_ref(&fault_plan_value(&base).expect("base plan")).expect("base ref");
        let changed_ref = canonical_ref(&fault_plan_value(&changed).expect("changed plan")).expect("changed ref");

        assert_ne!(base_ref, changed_ref);
    }

    #[test]
    fn unauthorized_transport_denies_before_side_effects() {
        // r[verify molten.testing.distributed_simulation.fixtures]
        // r[verify molten.testing.distributed_simulation.property_invariants]
        let mut unauthorized = command("op-transport");
        unauthorized.authority_ref = None;
        let input = input_with(
            FaultPlan {
                events: vec![fault(FAULT_UNAUTHORIZED_TRANSPORT, "op-transport", "transport-only")],
                caveats: Vec::new(),
            },
            vec![unauthorized],
        );

        let run = run_distributed_simulation(&input).expect("run");

        assert_eq!(run.decision, DENY_DECISION);
        assert!(run.committed_operation_ids.is_empty());
        assert_eq!(run.denied_operation_ids, vec!["op-transport".to_string()]);
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "transport-evidence-does-not-grant-authority"));
    }

    #[test]
    fn duplicate_delivery_does_not_double_commit_and_restart_replays_stably() {
        // r[verify molten.testing.distributed_simulation.property_invariants]
        let input = input_with(
            FaultPlan {
                events: vec![
                    fault(FAULT_RESTART, "op-restart", "restart-window"),
                    fault(FAULT_DUPLICATE, "op-duplicate", "duplicate-delivery"),
                ],
                caveats: Vec::new(),
            },
            vec![command("op-restart"), command("op-duplicate")],
        );

        let run = run_distributed_simulation(&input).expect("run");

        assert_eq!(run.decision, PASS_DECISION);
        assert_eq!(run.committed_operation_ids, vec!["op-restart".to_string()]);
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == DUPLICATE_SUPPRESSED_DECISION));
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic.contains("restart:restart-window")));
    }

    #[test]
    fn partitioned_quorum_and_ambient_state_deny() {
        // r[verify molten.testing.distributed_simulation.fixtures]
        let mut quorum = command("op-quorum");
        quorum.requires_quorum = true;
        let ambient = command("op-ambient");
        let input = input_with(
            FaultPlan {
                events: vec![
                    fault(FAULT_PARTITION, "op-quorum", "partition-window"),
                    fault(FAULT_AMBIENT_STATE_DRIFT, "op-ambient", "host-path-drift"),
                ],
                caveats: Vec::new(),
            },
            vec![quorum, ambient],
        );

        let run = run_distributed_simulation(&input).expect("run");

        assert_eq!(run.decision, DENY_DECISION);
        assert_eq!(run.denied_operation_ids, vec!["op-ambient".to_string(), "op-quorum".to_string()]);
        assert!(
            run.diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "partitioned-quorum-denied-before-side-effects")
        );
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "undeclared-ambient-state"));
    }

    fn traceability_manifest() -> crate::trace_core::TraceabilityManifest {
        let requirement = crate::trace_core::RequirementInput {
            id: "molten.testing.distributed_ci.fixture".to_string(),
            source: "cairn/changes/distributed-test-ci-risk-matrix/specs/testing-harness/spec.md".to_string(),
            kind: "evidence".to_string(),
            changed: true,
        };
        let positive = verification_evidence("positive");
        let negative = verification_evidence("negative");
        crate::trace_core::build_traceability_manifest(&crate::trace_core::TraceabilityInput {
            requirements: vec![requirement],
            coverage: vec![crate::trace_core::CoverageInput {
                requirement_id: "molten.testing.distributed_ci.fixture".to_string(),
                positive: vec![positive],
                negative: vec![negative],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest")
    }

    fn verification_evidence(kind: &str) -> crate::trace_core::VerificationEvidence {
        crate::trace_core::VerificationEvidence {
            target: format!("tests/distributed-{kind}.rs"),
            command: format!("cargo test distributed_{kind}"),
            artifact_ref: local_ref(&format!("traceability:{kind}")),
            artifact_refs: vec![local_ref(&format!("traceability:{kind}"))],
            target_exists: true,
            artifact_present: true,
            source: "compatibility".to_string(),
            receipt_ref: None,
            expected_decision: "compatibility".to_string(),
        }
    }

    fn metadata(profile_id: &str) -> DistributedTestMetadata {
        build_distributed_test_metadata(&DistributedTestMetadataInput {
            source_ref: local_ref("source-tree"),
            nix_input_refs: vec![local_ref("nix-inputs")],
            test_binary_ref: local_ref("test-binary"),
            profile_id: profile_id.to_string(),
            shard_id: format!("{profile_id}-shard"),
            seed_ref: local_ref("seed"),
            topology_ref: local_ref("topology"),
            fault_plan_ref: local_ref("fault-plan"),
            receipt_refs: vec![local_ref(&format!("receipt:{profile_id}"))],
            variance_refs: vec![local_ref("variance:none")],
            diagnostic_log_refs: vec![local_ref(&format!("log:{profile_id}"))],
        })
        .expect("metadata")
    }

    fn profile_run(profile_id: &str, metadata_ref: String, traceability_ref: String) -> DistributedProfileRun {
        DistributedProfileRun {
            profile_id: profile_id.to_string(),
            decision: PASS_DECISION.to_string(),
            metadata_ref,
            traceability_ref,
            positive_coverage: true,
            negative_coverage: true,
            retry_attempts: 0,
            unavailable: false,
            unsupported_reason: None,
            variance_declared: true,
            required_for_release: false,
        }
    }

    #[test]
    fn distributed_ci_matrix_declares_profiles_and_metadata() {
        // r[verify molten.testing.distributed_ci.profile_matrix]
        // r[verify molten.testing.distributed_ci.metadata_binding]
        let matrix = build_distributed_ci_matrix(default_distributed_ci_profiles()).expect("matrix");
        let protocol_metadata = metadata(PROFILE_PROTOCOL);
        let rendered = crate::preserves_rail::to_text(&matrix.value).expect("render matrix");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert_eq!(matrix.profiles.len(), REQUIRED_DISTRIBUTED_PROFILE_COUNT);
        assert!(rendered.contains("vm-fault"));
        assert!(rendered.contains("nix build .#checks.x86_64-linux.nixos-vm-multinode"));
        assert_eq!(protocol_metadata.profile_id, PROFILE_PROTOCOL);
        assert!(
            crate::preserves_rail::to_text(&protocol_metadata.value)
                .expect("render metadata")
                .contains("diagnostic-logs")
        );
    }

    #[test]
    fn distributed_ci_gate_requires_traceability_and_zero_retry_pass() {
        // r[verify molten.testing.distributed_ci.traceability_required_gate]
        // r[verify molten.testing.distributed_ci.retry_policy]
        let matrix = build_distributed_ci_matrix(default_distributed_ci_profiles()).expect("matrix");
        let traceability = traceability_manifest();
        let metadata = metadata(PROFILE_FAST);
        let mut run = profile_run(PROFILE_FAST, metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        run.retry_attempts = 1;
        let gate = evaluate_distributed_ci_gate(&DistributedCiGateInput {
            matrix: &matrix,
            metadata: &[metadata],
            traceability_manifest: &traceability,
            runs: &[run],
        })
        .expect("gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "retry-only-success-denied:fast"));
    }

    #[test]
    fn distributed_ci_gate_rejects_missing_negative_coverage_and_unavailable_pass() {
        // r[verify molten.testing.distributed_ci.unavailable_handling]
        // r[verify molten.testing.distributed_ci.negative_fixtures]
        let matrix = build_distributed_ci_matrix(default_distributed_ci_profiles()).expect("matrix");
        let traceability = traceability_manifest();
        let metadata = metadata(PROFILE_VM_FAULT);
        let mut run = profile_run(PROFILE_VM_FAULT, metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        run.negative_coverage = false;
        run.unavailable = true;
        run.required_for_release = true;
        run.unsupported_reason = Some("no-kvm".to_string());
        let gate = evaluate_distributed_ci_gate(&DistributedCiGateInput {
            matrix: &matrix,
            metadata: &[metadata],
            traceability_manifest: &traceability,
            runs: &[run],
        })
        .expect("gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-negative-coverage:vm-fault"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "unavailable-profile-cannot-pass:vm-fault"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "required-profile-unavailable:vm-fault"));
    }

    #[test]
    fn distributed_ci_matrix_negative_fixture_rejects_missing_profile() {
        // r[verify molten.testing.distributed_ci.negative_fixtures]
        let mut profiles = default_distributed_ci_profiles();
        profiles.retain(|profile| profile.id != PROFILE_PROTOCOL);
        let matrix = build_distributed_ci_matrix(profiles).expect("matrix");

        assert_eq!(matrix.decision, DENY_DECISION);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "missing-profile:protocol"));
    }
}
