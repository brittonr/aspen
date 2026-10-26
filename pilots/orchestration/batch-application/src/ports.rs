// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Arc;

use async_trait::async_trait;
use molten_batch_core::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    Denied,
    Conflict,
    Unknown,
    Invalid(String),
    Bound,
    Unsupported,
}
impl From<Violation> for Error {
    fn from(v: Violation) -> Self {
        Self::Invalid(v.to_string())
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commit {
    Applied,
    Conflict,
    Unknown,
}
/// CAS compares the complete expected record, with immediate durable commit.
/// Unknown is not a negative acknowledgement and must never trigger execution.
pub trait StateStore: Send + Sync {
    fn load(&self) -> Result<State, Error>;
    fn compare_commit(&self, expected: &State, next: &State) -> Result<Commit, Error>;
}
pub trait WorkerJournal: Send + Sync {
    fn load(&self) -> Result<WorkerState, Error>;
    fn compare_commit(&self, expected: &WorkerState, next: &WorkerState) -> Result<Commit, Error>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub manifest: ContentManifest,
    pub bytes: Vec<u8>,
}
pub trait ContentPort: Send + Sync {
    fn get(&self, id: Id) -> Result<Content, Error>;
    fn put(&self, content: &Content) -> Result<Id, Error>;
    fn make_data(&self, bytes: Vec<u8>) -> Result<Content, Error>;
}
pub trait IdentityPort: Send + Sync {
    fn spec(&self, spec: &JobSpec) -> Result<Id, Error>;
    fn run(&self, cluster: Id, key: &str) -> Result<Id, Error>;
    fn assignment(&self, cluster: Id, generation: u64, p: &Placement, spec: Id) -> Result<Id, Error>;
    fn request(&self, r: &ExecutionRequest) -> Result<Id, Error>;
    fn result(&self, r: &ExecutionResult) -> Result<Id, Error>;
}
pub trait PlacementPort: Send + Sync {
    fn next(&self, s: &State) -> Result<Option<Placement>, Error>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionAction {
    Reserve,
    Execute,
    Complete,
    Cancel,
}
pub trait AdmissionPort: Send + Sync {
    fn submit(&self, spec: &JobSpec, policy: Id) -> Result<(), Error>;
    fn assignment(&self, r: &ExecutionRequest, action: AdmissionAction) -> Result<(), Error>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerInfo {
    pub node: Id,
    pub cluster: Id,
    pub generation: u64,
    pub policy_ref: Id,
    pub capacity: Resources,
    pub boot: u64,
    pub draining: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerObservation {
    Missing,
    Accepted { request_ref: Id },
    Running { request_ref: Id },
    Terminal { request_ref: Id, result: ExecutionResult },
}
#[async_trait]
pub trait WorkerClient: Send + Sync {
    async fn info(&self, node: Id) -> Result<WorkerInfo, Error>;
    async fn put(&self, node: Id, id: Id, content: Content) -> Result<(), Error>;
    async fn get(&self, node: Id, id: Id) -> Result<Content, Error>;
    async fn start(&self, r: ExecutionRequest) -> Result<WorkerObservation, Error>;
    async fn observe(&self, r: &ExecutionRequest) -> Result<WorkerObservation, Error>;
    async fn cancel(&self, r: &ExecutionRequest) -> Result<WorkerObservation, Error>;
}
/// Only a stopped observation may become an authoritative terminal record.
/// An error does not prove teardown. The application leaves it for reconciliation.
#[derive(Debug)]
pub struct ExecutionOutput {
    pub stopped: bool,
    pub output: Result<Vec<u8>, FailureCode>,
}
#[async_trait]
pub trait ExecutionPort: Send + Sync {
    async fn run(
        &self,
        r: ExecutionRequest,
        component: Content,
        inputs: Vec<Vec<u8>>,
    ) -> Result<ExecutionOutput, Error>;
    fn cancel(&self, id: Id) -> Result<(), Error>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    Transport { node: Id },
    CommitUnknown,
    Admission { attempt: Id },
    PendingReconciliation { attempt: Id },
    ServiceError,
}
pub trait ObservabilityPort: Send + Sync {
    fn record(&self, event: Diagnostic);
}
pub struct ControllerPorts {
    pub store: Arc<dyn StateStore>,
    pub content: Arc<dyn ContentPort>,
    pub identity: Arc<dyn IdentityPort>,
    pub placement: Arc<dyn PlacementPort>,
    pub admission: Arc<dyn AdmissionPort>,
    pub workers: Arc<dyn WorkerClient>,
    pub observations: Arc<dyn ObservabilityPort>,
}
pub struct WorkerPorts {
    pub journal: Arc<dyn WorkerJournal>,
    pub content: Arc<dyn ContentPort>,
    pub identity: Arc<dyn IdentityPort>,
    pub admission: Arc<dyn AdmissionPort>,
    pub execution: Arc<dyn ExecutionPort>,
    pub observations: Arc<dyn ObservabilityPort>,
}
