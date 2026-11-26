#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationServiceManifestInput {
    pub service_id: String,
    pub services: Vec<String>,
    pub control_group_ref: String,
    pub queue_capacity: u64,
    pub semaphore_capacity: u64,
    pub rate_limit: u64,
    pub barrier_parties: u64,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationServiceManifest {
    pub manifest_ref: String,
    pub service_id: String,
    pub services: Vec<String>,
    pub control_group_ref: String,
    pub queue_capacity: u64,
    pub semaphore_capacity: u64,
    pub rate_limit: u64,
    pub barrier_parties: u64,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationRequestInput {
    pub service: String,
    pub operation: String,
    pub key: String,
    pub client_session: String,
    pub operation_id_ref: String,
    pub read_consistency_mode: String,
    pub payload: Option<IoValue>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationRequest {
    pub request_ref: String,
    pub service: String,
    pub operation: String,
    pub key: String,
    pub client_session: String,
    pub operation_id_ref: String,
    pub read_consistency_mode: String,
    pub payload: Option<IoValue>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencingToken {
    pub token_ref: String,
    pub key: String,
    pub owner: String,
    pub token: u64,
    pub lease_epoch: u64,
    pub commit_receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationStatusAssertion {
    pub assertion_ref: String,
    pub service: String,
    pub key: String,
    pub read_consistency_mode: String,
    pub state_ref: String,
    /// Control-plane receipt the observation is bound to: the Raft commit receipt of a mutation or the
    /// registry read receipt of a read. The coordination receipt in turn lists this assertion's ref.
    pub control_plane_receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationStateSnapshot {
    pub state_ref: String,
    pub retention_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub service: String,
    pub operation: String,
    pub read_consistency_mode: String,
    pub request_ref: String,
    pub raft_receipt_ref: Option<String>,
    pub token_ref: Option<String>,
    pub state_ref: String,
    pub transition_kind: String,
    pub before_state_ref: String,
    pub after_state_ref: Option<String>,
    pub preserved_state_ref: Option<String>,
    pub output_refs: Vec<String>,
    pub control_plane_intent_ref: Option<String>,
    pub prior_receipt_ref: Option<String>,
    pub dataspace_assertion_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationApplyResult {
    pub receipt: CoordinationReceipt,
    pub request: CoordinationRequest,
    pub token: Option<FencingToken>,
    pub state_snapshot: CoordinationStateSnapshot,
    pub assertions: Vec<CoordinationStatusAssertion>,
    pub raft_commit_ref: Option<String>,
    pub raft_read_receipt: Option<RaftReadReceipt>,
    pub evidence_values: Vec<IoValue>,
    /// The clock sample a committed mutation recorded; `None` for reads, denials, and replays. Replaying the
    /// committed requests with these samples reproduces the committed state.
    pub time_sample: Option<CoordinationTimeSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationFixtureRun {
    pub decision: String,
    pub manifest_ref: String,
    pub final_state_ref: String,
    pub receipt_refs: Vec<String>,
    pub assertion_refs: Vec<String>,
    pub evidence_values: Vec<IoValue>,
    pub report_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationApplyReport {
    pub report_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub final_state_ref: String,
    pub receipt_refs: Vec<String>,
    pub assertion_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

/// The imperative coordination shell: committed state, its in-process control registry, and the
/// shell-owned clock port that samples coordination time. The clock is private: a clock on another basis
/// is installed only through [`recreate_coordination_live_clock`], which commits the basis change.
#[derive(Debug)]
pub struct CoordinationRuntime {
    clock: CoordinationClock,
    pub manifest: CoordinationServiceManifest,
    pub raft: ControlRegistryRuntime,
    pub state: CoordinationState,
    pub applied_operations: OrderedMap<String, CoordinationApplyResult>,
    pub receipts: Vec<CoordinationReceipt>,
    pub assertions: Vec<CoordinationStatusAssertion>,
    pub tokens: Vec<FencingToken>,
    pub next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoordinationState {
    pub locks: OrderedMap<String, LockState>,
    pub queues: OrderedMap<String, Vec<String>>,
    pub semaphores: OrderedMap<String, OrderedSet<String>>,
    pub rates: OrderedMap<String, u64>,
    pub elections: OrderedMap<String, ElectionState>,
    pub barriers: OrderedMap<String, BarrierState>,
    pub registry: OrderedMap<String, RegistryEntry>,
    pub next_fencing_token: u64,
    /// Time basis every committed clock sample must come from; bound when the runtime is built.
    pub time_basis_ref: String,
    /// Clock sample (ticks of `time_basis_ref`) recorded by the last committed mutation. Samples may never
    /// regress below it.
    pub time_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockState {
    pub owner: String,
    pub token: u64,
    /// `token_ref` of the issued [`FencingToken`], which is bound to the lock's control-plane commit receipt.
    pub token_ref: String,
    pub lease_epoch: u64,
    /// Coordination clock tick at which the lease becomes expirable through [`OP_EXPIRE`].
    pub lease_expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionState {
    pub leader: String,
    pub token: u64,
    /// `token_ref` of the issued [`FencingToken`], which is bound to the election's control-plane commit receipt.
    pub token_ref: String,
    /// Coordination clock tick at which the leader term becomes expirable through [`OP_EXPIRE`].
    pub lease_expires_at: u64,
}

/// A fencing-token grant prepared by a pure transition core. The token itself, and the lock or election
/// entry carrying its `token_ref`, exist only after the control plane commits the transition, because the
/// token binds that commit receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationTokenClaim {
    pub service: String,
    pub key: String,
    pub owner: String,
    pub token: u64,
    pub lease_epoch: u64,
    pub lease_expires_at: u64,
}
