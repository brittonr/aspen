//! Capability-rooted durable orchestration of actual admitted native services.

mod codec;
mod groups;
pub mod native;
pub mod preview;
mod store;
mod work;

pub const MAX_HOST_GROUPS: usize = 32;
pub const MAX_HOST_MEMBERS: usize = 64;
pub const MAX_HOST_WORK: usize = 256;
pub const MAX_HOST_WORK_MIGRATIONS: usize = 8;
pub const MAX_HOST_PAYLOAD_BYTES: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequest {
    pub operation: String,
    pub subject: String,
    pub logical_tick: u64,
    pub state_ref: String,
    pub input_ref: String,
}

/// Implementations must check current authority, not merely parse a reference.
pub trait AuthorityPort {
    fn admit(&mut self, request: &OperationRequest) -> crate::error::Result<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeAction {
    Start,
    Stop,
    Fence,
    Restart,
    Drain,
    Checkpoint,
    Upgrade { admission_ref: String },
}

pub struct NativeEffectRouteInput<'a> {
    pub instance: &'a crate::system_extension::NativeInstanceRecord,
    pub callback: &'a crate::system_extension::CallbackReceipt,
    pub group_id: Option<&'a str>,
}

pub trait NativeEffectRouter: crate::system_extension::FabricEffectPort + Send {
    fn admit(&mut self, input: NativeEffectRouteInput<'_>) -> crate::error::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeEffectDisposition {
    NotRequested,
    Unrouted,
    Completed(Vec<String>),
}

pub struct RuntimeDelivery {
    pub admission: crate::system_extension::NativeIngressAdmission,
    pub dispatch: crate::system_extension::DispatchResult,
    pub effects: NativeEffectDisposition,
}

pub trait RuntimePort {
    fn instance(&self, member: &str) -> crate::error::Result<crate::system_extension::NativeInstanceRecord>;
    fn apply(
        &mut self,
        member: &str,
        action: &RuntimeAction,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord>;
    fn deliver(
        &mut self,
        member: &str,
        ingress: &crate::system_extension::NativeIngressEnvelope,
        logical_tick: u64,
    ) -> crate::error::Result<RuntimeDelivery>;
    fn migration(&mut self, member: &str, migration_ref: &str) -> crate::error::Result<preserves::IOValue>;
}

pub struct Host<A, R> {
    store: store::Database,
    authority: A,
    runtime: R,
    state: State,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct State {
    revision: u64,
    groups: std::collections::BTreeMap<String, Group>,
    members: std::collections::BTreeMap<String, crate::system_extension::NativeInstanceRecord>,
    work: std::collections::BTreeMap<String, Work>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    declaration: crate::system_extension::RecoveryGroupDeclaration,
    history: Vec<crate::system_extension::RecoveryAttempt>,
    last_tick: u64,
    admission_ref: String,
    pending: Vec<Action>,
    is_terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Action {
    member: String,
    kind: RuntimeAction,
    before: crate::system_extension::NativeInstanceRecord,
    is_attempted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Work {
    member: String,
    presentation: crate::system_extension::WorkPresentation,
    payload: crate::system_extension::NativeCallbackValue,
    claim: Option<crate::system_extension::DeliveryClaim>,
    state: WorkState,
    receipt_ref: Option<String>,
    migrations: Vec<WorkMigration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkMigration {
    source: crate::system_extension::WorkPresentation,
    source_payload_ref: String,
    migration_ref: String,
    admission_ref: String,
}

pub struct MigratedWorkInput {
    pub work_ref: String,
    pub migration_ref: String,
    pub payload: crate::system_extension::NativeCallbackValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkState {
    Pending,
    Claimed,
    Unknown,
    Completed,
}

pub struct DeliveryInput {
    pub member: String,
    pub presentation: crate::system_extension::WorkPresentation,
    pub claim_ref: Option<String>,
    pub ingress: crate::system_extension::NativeIngressEnvelope,
    pub logical_tick: u64,
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
