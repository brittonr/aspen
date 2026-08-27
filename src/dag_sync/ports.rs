use molten_core::dag_sync::*;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagAuthorityObservation {
    pub authority_ref: String,
    pub plan_ref: DagPlanRef,
    pub epoch_ref: DagEpochRef,
    pub generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagResourceObservation {
    pub reservation_ref: String,
    pub plan_ref: DagPlanRef,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagTransportEnvelope {
    pub object_ref: DagObjectRef,
    pub assigned_peer: Option<DagPeerId>,
    pub encoded_bytes: u64,
    pub transport_observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagTransferOutcome {
    Received(DagTransportEnvelope),
    Deferred(String),
    Cancelled(String),
}

pub trait DagAuthorityPort {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation>;
}

pub trait DagResourcePort {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation>;
}

pub trait DagTransportPort {
    fn request(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome>;
}

pub trait DagContentVerificationPort {
    fn verify(
        &mut self,
        plan: &DagSyncPlan,
        envelope: &DagTransportEnvelope,
        authority_ref: &str,
    ) -> Result<DagResponseObservation>;
}

pub trait DagProgressPort {
    fn load(&mut self, epoch_ref: &DagEpochRef) -> Result<Option<DagSyncProgress>>;
    fn store(&mut self, progress: &DagSyncProgress) -> Result<String>;
}

pub trait DagObservationPort {
    fn publish_response(&mut self, response: &CanonicalDagRecord) -> Result<()>;
    fn publish_progress(&mut self, progress: &CanonicalDagRecord) -> Result<()>;
}

pub trait DagReceiptPort {
    fn publish_receipt(&mut self, receipt: &CanonicalDagRecord) -> Result<()>;
}

use super::CanonicalDagRecord;
