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

// r[impl molten.dag_sync.content_adapter_boundary]
pub trait DagContentVerificationPort {
    fn verify(
        &mut self,
        plan: &DagSyncPlan,
        envelope: &DagTransportEnvelope,
        authority_ref: &str,
    ) -> Result<DagResponseObservation>;
}

// r[impl molten.dag_sync.resume_fencing]
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

pub const DAG_SYNC_PORT_ID: &str = "molten.extension.dag-sync";
pub const DAG_SYNC_PORT_VERSION: &str = "v1";
pub const DAG_SYNC_OPERATION: &str = "sync";

// r[impl molten.dag_sync.fabric_admission]
pub fn dag_sync_port_descriptor(
    profile_ref: &str,
    conformance_refs: &[String],
) -> crate::error::Result<crate::fabric::FabricPortDescriptor> {
    crate::preserves_rail::validate_content_ref(profile_ref)?;
    if conformance_refs.is_empty() || conformance_refs.len() >= 128 {
        return Err(crate::error::Failure::invalid_harness(
            "DAG fabric binding requires bounded reviewed conformance refs",
        ));
    }
    for reference in conformance_refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    let mut reviewed = conformance_refs.to_vec();
    reviewed.push(profile_ref.to_string());
    reviewed.sort();
    reviewed.dedup();
    Ok(crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: DAG_SYNC_PORT_ID.to_string(),
        version: DAG_SYNC_PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Transport,
        operation_classes: vec![DAG_SYNC_OPERATION.to_string()],
        input_schema_refs: vec![dag_sync_request_schema_ref()?],
        output_schema_refs: vec![dag_sync_receipt_schema_ref()?],
        authority_requirements: vec![
            crate::fabric::FabricAuthority::Transport,
            crate::fabric::FabricAuthority::Policy,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Evidence,
        ],
        resource_requirements: vec![
            crate::fabric::FabricResource::Memory,
            crate::fabric::FabricResource::NetworkBytes,
            crate::fabric::FabricResource::Concurrency,
            crate::fabric::FabricResource::QueueDepth,
            crate::fabric::FabricResource::LogicalTime,
            crate::fabric::FabricResource::Diagnostics,
        ],
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: "bounded-admitted-dag-sync-v1".to_string(),
        conformance_refs: reviewed,
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    })
}

pub fn dag_sync_request_schema_ref() -> crate::error::Result<String> {
    schema_ref("molten-dag-sync-request-v1", &[
        "roots",
        "strategy",
        "inventory",
        "progress",
        "peers",
        "epoch",
        "generation",
        "policy",
        "bounds",
    ])
}
pub fn dag_sync_receipt_schema_ref() -> crate::error::Result<String> {
    schema_ref("molten-dag-sync-receipt-v1", &[
        "decision",
        "plan",
        "epoch",
        "generation",
        "strategy",
        "requested",
        "verified",
        "missing",
        "issues",
        "non-claims",
    ])
}
fn schema_ref(label: &str, fields: &[&str]) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("dag-sync-fabric-schema-v1", vec![
        crate::preserves_rail::string(label),
        crate::preserves_rail::sequence(fields.iter().map(crate::preserves_rail::string).collect()),
    ]))
}
