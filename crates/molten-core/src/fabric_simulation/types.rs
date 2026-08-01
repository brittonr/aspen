use crate::fabric::FabricPortClass;
use crate::fabric::ReferenceSystemKind;

pub const FABRIC_SIMULATION_WORLD_SCHEMA: &str = "molten.fabric-simulation.world.v1";
pub const FABRIC_SIMULATION_RUN_SCHEMA: &str = "molten.fabric-simulation.run.v1";
pub const FABRIC_SIMULATION_REPRO_SCHEMA: &str = "molten.fabric-simulation.repro.v1";
pub const FABRIC_SIMULATION_PROFILE_ID: &str = "deterministic-whole-system-v1";
pub const FABRIC_SIMULATION_PORT_VERSION: &str = "v1";
pub const MAX_WORLD_NODES: usize = 64;
pub const MAX_WORLD_PORT_PROFILES: usize = 32;
pub const MAX_WORLD_WORKLOAD_STEPS: usize = 4_096;
pub const MAX_WORLD_FAULTS: usize = 1_024;
pub const MAX_WORLD_INVARIANTS: usize = 128;
pub const MAX_WORLD_NON_CLAIMS: usize = 16;
pub const MAX_WORLD_REF_ITEMS: usize = 256;
pub const MAX_WORLD_IDENTIFIER_BYTES: usize = 256;
pub const MAX_ELIGIBLE_CHOICES: usize = 4_096;
pub const MAX_SHRINK_ATTEMPTS: u64 = 4_096;
pub const FIRST_CHOICE_POSITION: u64 = 0;
pub const FIRST_EVENT_POSITION: u64 = 0;
pub const FIRST_VIRTUAL_TICK: u64 = 0;
pub const FIRST_WORKLOAD_SEQUENCE: u64 = 0;
pub const INITIAL_EXTENSION_GENERATION: u64 = 1;
pub const UNIT_RESOURCE_COST: u64 = 1;
pub const REQUIRED_SIMULATION_PORT_CLASS_COUNT: usize = 13;
pub const REQUIRED_UNIVERSAL_INVARIANT_COUNT: usize = 6;
pub const REQUIRED_SIMULATION_NON_CLAIM_COUNT: usize = 7;

pub const REQUIRED_SIMULATION_PORT_CLASSES: [FabricPortClass; REQUIRED_SIMULATION_PORT_CLASS_COUNT] = [
    FabricPortClass::Authority,
    FabricPortClass::Transport,
    FabricPortClass::DurableState,
    FabricPortClass::Time,
    FabricPortClass::Scheduling,
    FabricPortClass::Membership,
    FabricPortClass::Placement,
    FabricPortClass::Consistency,
    FabricPortClass::Supervision,
    FabricPortClass::Policy,
    FabricPortClass::Resources,
    FabricPortClass::Simulation,
    FabricPortClass::Evidence,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationClaimProfile {
    PureModel,
    DeterministicWholeSystem,
    MultiProcessLive,
    HostChaos,
    VmHardware,
}

impl SimulationClaimProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PureModel => "pure-model",
            Self::DeterministicWholeSystem => "deterministic-whole-system",
            Self::MultiProcessLive => "multi-process-live",
            Self::HostChaos => "host-chaos",
            Self::VmHardware => "vm-hardware",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationNonClaim {
    LiveTransport,
    LiveDisk,
    OperatingSystemTiming,
    ProductionScale,
    ProductionReadiness,
    ExternalProductCompatibility,
    ArbitraryScheduleCorrectness,
}

impl SimulationNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LiveTransport => "does-not-prove-live-transport",
            Self::LiveDisk => "does-not-prove-live-disk",
            Self::OperatingSystemTiming => "does-not-prove-operating-system-timing",
            Self::ProductionScale => "does-not-prove-production-scale",
            Self::ProductionReadiness => "does-not-prove-production-readiness",
            Self::ExternalProductCompatibility => "does-not-prove-external-product-compatibility",
            Self::ArbitraryScheduleCorrectness => "does-not-prove-arbitrary-schedule-correctness",
        }
    }
}

pub const REQUIRED_SIMULATION_NON_CLAIMS: [SimulationNonClaim; REQUIRED_SIMULATION_NON_CLAIM_COUNT] = [
    SimulationNonClaim::LiveTransport,
    SimulationNonClaim::LiveDisk,
    SimulationNonClaim::OperatingSystemTiming,
    SimulationNonClaim::ProductionScale,
    SimulationNonClaim::ProductionReadiness,
    SimulationNonClaim::ExternalProductCompatibility,
    SimulationNonClaim::ArbitraryScheduleCorrectness,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCoreIdentity {
    pub implementation_ref: String,
    pub manifest_ref: String,
    pub callback_dispatcher_ref: String,
    pub protocol_core_ref: String,
    pub state_machine_ref: String,
    pub schema_set_ref: String,
    pub port_contract_set_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameCoreWitness {
    pub simulation: ExtensionCoreIdentity,
    pub live: ExtensionCoreIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedNode {
    pub node_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub initial_state_ref: String,
    pub membership_view_ref: String,
    pub placement_ref: String,
    pub consistency_profile_ref: String,
    pub same_core: SameCoreWitness,
    pub required_port_classes: Vec<FabricPortClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedPortProfile {
    pub class: FabricPortClass,
    pub port_id: String,
    pub version: String,
    pub implementation_profile: String,
    pub descriptor_ref: String,
    pub command_schema_ref: String,
    pub event_schema_ref: String,
    pub deterministic: bool,
    pub declared_faults: Vec<SimulationFaultKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulerChoiceKind {
    Runnable,
    MessageDelivery,
    TimerFire,
    StorageCompletion,
    ProcessLifecycle,
    FaultActivation,
}

impl SchedulerChoiceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Runnable => "runnable",
            Self::MessageDelivery => "message-delivery",
            Self::TimerFire => "timer-fire",
            Self::StorageCompletion => "storage-completion",
            Self::ProcessLifecycle => "process-lifecycle",
            Self::FaultActivation => "fault-activation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EligibleChoice {
    pub kind: SchedulerChoiceKind,
    pub choice_id: String,
    pub node_id: String,
    pub generation: u64,
    pub ready_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerChoiceRecord {
    pub position: u64,
    pub virtual_tick: u64,
    pub eligible: Vec<EligibleChoice>,
    pub selected: EligibleChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationSchedulerState {
    pub next_choice_position: u64,
    pub event_count: u64,
    pub virtual_tick: u64,
    pub terminal: bool,
}

impl Default for SimulationSchedulerState {
    fn default() -> Self {
        Self {
            next_choice_position: FIRST_CHOICE_POSITION,
            event_count: FIRST_EVENT_POSITION,
            virtual_tick: FIRST_VIRTUAL_TICK,
            terminal: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationFaultKind {
    Delay,
    Drop,
    Duplicate,
    Reorder,
    Partition,
    Reset,
    BoundedCorruption,
    CapacityExhaustion,
    Pause,
    Crash,
    Restart,
    ClockSkew,
    ClockJump,
    AuthorityRevocation,
    MembershipChange,
    PlacementReplacement,
    ConsistencyQuorumLoss,
}

impl SimulationFaultKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delay => "delay",
            Self::Drop => "drop",
            Self::Duplicate => "duplicate",
            Self::Reorder => "reorder",
            Self::Partition => "partition",
            Self::Reset => "reset",
            Self::BoundedCorruption => "bounded-corruption",
            Self::CapacityExhaustion => "capacity-exhaustion",
            Self::Pause => "pause",
            Self::Crash => "crash",
            Self::Restart => "restart",
            Self::ClockSkew => "clock-skew",
            Self::ClockJump => "clock-jump",
            Self::AuthorityRevocation => "authority-revocation",
            Self::MembershipChange => "membership-change",
            Self::PlacementReplacement => "placement-replacement",
            Self::ConsistencyQuorumLoss => "consistency-quorum-loss",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationFaultAction {
    pub fault_id: String,
    pub kind: SimulationFaultKind,
    pub target: String,
    pub boundary: FabricPortClass,
    pub activate_at_choice: u64,
    pub duration_choices: Option<u64>,
    pub resource_cost: u64,
    pub expected_observation: String,
    pub direct_extension_state_mutation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UniversalInvariantKind {
    NoAmbientEffect,
    NoStaleGenerationMutation,
    NoResourceBoundBypass,
    NoPortStateMachineViolation,
    ValidCanonicalRefs,
    CompleteTerminalCleanup,
}

impl UniversalInvariantKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoAmbientEffect => "no-ambient-effect",
            Self::NoStaleGenerationMutation => "no-stale-generation-mutation",
            Self::NoResourceBoundBypass => "no-resource-bound-bypass",
            Self::NoPortStateMachineViolation => "no-port-state-machine-violation",
            Self::ValidCanonicalRefs => "valid-canonical-refs",
            Self::CompleteTerminalCleanup => "complete-terminal-cleanup",
        }
    }
}

pub const REQUIRED_UNIVERSAL_INVARIANTS: [UniversalInvariantKind; REQUIRED_UNIVERSAL_INVARIANT_COUNT] = [
    UniversalInvariantKind::NoAmbientEffect,
    UniversalInvariantKind::NoStaleGenerationMutation,
    UniversalInvariantKind::NoResourceBoundBypass,
    UniversalInvariantKind::NoPortStateMachineViolation,
    UniversalInvariantKind::ValidCanonicalRefs,
    UniversalInvariantKind::CompleteTerminalCleanup,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationInvariant {
    Universal(UniversalInvariantKind),
    ExtensionSemantic {
        service: ReferenceSystemKind,
        invariant_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationObservation {
    pub sequence: u64,
    pub node_id: String,
    pub service: Option<ReferenceSystemKind>,
    pub generation: u64,
    pub state_ref: String,
    pub history_ref: String,
    pub port_event_ref: String,
    pub ambient_effect: bool,
    pub stale_generation_mutation: bool,
    pub resource_bound_bypass: bool,
    pub port_state_machine_violation: bool,
    pub terminal_cleanup_complete: bool,
    pub semantic_invariants_passed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantResult {
    pub invariant: SimulationInvariant,
    pub passed: bool,
    pub first_failure_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationWorkloadStep {
    pub sequence: u64,
    pub node_id: String,
    pub request_ref: String,
    pub service: ReferenceSystemKind,
    pub expected_failure_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationBounds {
    pub max_choices: u64,
    pub max_events: u64,
    pub max_virtual_ticks: u64,
    pub max_trace_bytes: u64,
    pub max_resource_units: u64,
    pub max_shrink_attempts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedWorldManifest {
    pub schema: String,
    pub runtime_ref: String,
    pub scheduler_input_ref: String,
    pub entropy_input_ref: String,
    pub authority_ref: String,
    pub policy_ref: String,
    pub initial_durable_state_ref: String,
    pub resource_profile_ref: String,
    pub workload_ref: String,
    pub fault_plan_ref: String,
    pub invariant_set_ref: String,
    pub nodes: Vec<SimulatedNode>,
    pub port_profiles: Vec<SimulatedPortProfile>,
    pub workload: Vec<SimulationWorkloadStep>,
    pub faults: Vec<SimulationFaultAction>,
    pub invariants: Vec<SimulationInvariant>,
    pub bounds: SimulationBounds,
    pub claim_profile: SimulationClaimProfile,
    pub non_claims: Vec<SimulationNonClaim>,
    pub ambient_inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedSimulatedWorld {
    pub manifest: SimulatedWorldManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationRunSummary {
    pub decision: SimulationDecision,
    pub choice_records: Vec<SchedulerChoiceRecord>,
    pub invariant_results: Vec<InvariantResult>,
    pub final_state_refs: Vec<String>,
    pub first_divergence: Option<ReplayDivergence>,
    pub resource_units: u64,
    pub virtual_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationDecision {
    Pass,
    InvariantFailed,
    Diverged,
    BoundExceeded,
    Denied,
}

impl SimulationDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::InvariantFailed => "invariant-failed",
            Self::Diverged => "diverged",
            Self::BoundExceeded => "bound-exceeded",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDivergence {
    pub position: u64,
    pub expected_choice_id: String,
    pub eligible_choice_ids: Vec<String>,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayComparison {
    pub matches: bool,
    pub first_divergence: Option<ReplayDivergence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrinkResult {
    pub world: SimulatedWorldManifest,
    pub attempts: u64,
    pub removed_workload_steps: u64,
    pub failure_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimEvidence {
    pub profile: SimulationClaimProfile,
    pub implementation_ref: String,
    pub environment_ref: Option<String>,
    pub adapter_refs: Vec<String>,
    pub lifecycle_ref: Option<String>,
    pub fault_ref: Option<String>,
    pub operator_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimPromotionDecision {
    pub admitted: bool,
    pub target: SimulationClaimProfile,
    pub missing_evidence: Vec<&'static str>,
}
