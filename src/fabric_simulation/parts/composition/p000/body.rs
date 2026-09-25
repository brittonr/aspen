use super::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "the simulation shell implements the application-owned typed effect-port boundary"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "the simulation shell implements the application-owned typed effect-port boundary"
)]
use crate::fabric::FabricPortResult;
use crate::system_extension::FabricEffectPort;

const REFERENCE_WORLD_MAX_CHOICES: u64 = 256;
const REFERENCE_WORLD_MAX_EVENTS: u64 = 256;
const REFERENCE_WORLD_MAX_VIRTUAL_TICKS: u64 = 4_096;
const REFERENCE_WORLD_MAX_TRACE_BYTES: u64 = 1_048_576;
const REFERENCE_WORLD_MAX_RESOURCE_UNITS: u64 = 4_096;
const REFERENCE_WORLD_MAX_SHRINK_ATTEMPTS: u64 = 256;
const REFERENCE_REQUEST_BYTES: u64 = 1;
const REFERENCE_FAULT_RESOURCE_COST: u64 = 1;
const REFERENCE_FAULT_ACTIVATION_CHOICE: u64 = 1;
const REFERENCE_FAULT_DURATION_CHOICES: u64 = 6;
const REFERENCE_CHOICE_ID_WIDTH: usize = 4;
const RUN_RESOURCE_INCREMENT: u64 = 1;
const FIRST_HISTORY_MATERIAL: &str = "fabric-simulation-history-v1";
pub const DEFAULT_REFERENCE_SEED: u64 = 0;
pub const DEFAULT_COMPLETION_DELAY_TICKS: u64 = 8;
const SUBMISSION_ACK_MATERIAL: &str = "submitted";
const COMPLETION_ACK_MATERIAL: &str = "completed";
pub const REFERENCE_CRASH_ACTIVATION: u64 = 3;
const DELIVERY_ACK_MATERIAL: &str = "delivered";
const CRASH_ACK_MATERIAL: &str = "crash-recovery";
const FIXTURE_FAILING_INVARIANT_ID: &str = "reference-fixture-failing-invariant";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSimulationFixtureRun {
    pub world: CanonicalSimulatedWorld,
    pub run: CanonicalSimulationRun,
    pub bundle: CanonicalSimulationReproBundle,
    pub observations: Vec<CanonicalSimulationObservation>,
    pub port_events: Vec<CanonicalSimulationPortEvent>,
    pub differential: CanonicalSimulationDifferential,
    pub host_evidence_refs: Vec<String>,
    pub crash_recoveries: Vec<ReferenceCrashRecovery>,
    pub service_states: std::collections::BTreeMap<String, ReferenceServiceState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCrashRecovery {
    pub fault_id: String,
    pub lost_operations: Vec<SimulatedStorageOperation>,
    pub durable_entry_request_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReplayResult {
    pub comparison: ReplayComparison,
    pub replay: ReferenceSimulationFixtureRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceShrinkFixture {
    pub original_world: CanonicalSimulatedWorld,
    pub shrunk_world: CanonicalSimulatedWorld,
    pub shrink: CanonicalSimulationShrink,
}

struct PreparedReferenceWorld {
    world: CanonicalSimulatedWorld,
    hosts: std::collections::BTreeMap<String, crate::system_extension::SystemExtensionHost<ReferenceServiceExecutor>>,
    admissions: std::collections::BTreeMap<String, crate::system_extension::CanonicalAdmittedSystemExtensionManifest>,
    profiles: Vec<SimulatedPortProfile>,
    operations: Vec<(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)>,
}

#[derive(Debug)]
struct DeterministicSimulationPortRouter {
    profiles: std::collections::BTreeMap<String, SimulatedPortProfile>,
    faults: Vec<SimulationFaultAction>,
    current_choice_position: u64,
    current_virtual_tick: u64,
    dispatching_node: String,
    dispatching_request_ref: String,
    next_submission_ordinal: u64,
    resource_units: u64,
    max_resource_units: u64,
    events: Vec<CanonicalSimulationPortEvent>,
    transport: SimulatedTransportState,
    storage: SimulatedStorageState,
    opened_partition_faults: std::collections::BTreeSet<String>,
}
