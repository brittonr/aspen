use molten_core::world_snapshot::SnapshotCohort;
use molten_core::world_snapshot::SnapshotComponent;
use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::SnapshotRestoreStep;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMaterializationObservation {
    pub component_identity: String,
    pub observation_ref: String,
    pub available: bool,
    pub identity_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotAdmissionObservation {
    pub admission_ref: String,
    pub descriptor_ref: String,
    pub profile_ref: String,
    pub cohort_ref: String,
    pub generation: u64,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotStepObservation {
    pub step: SnapshotRestoreStep,
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlDescriptorObservation {
    pub descriptor_ref: String,
    pub cohort_ref: String,
    pub available: bool,
    pub identity_verified: bool,
}

pub trait SnapshotMaterializationPort {
    fn observe_component(&mut self, component: &SnapshotComponent) -> Result<SnapshotMaterializationObservation>;
}

pub trait CurrentSnapshotAdmissionPort {
    fn observe_current(
        &mut self,
        descriptor: &SnapshotDescriptor,
        descriptor_ref: &str,
        destination: &SnapshotCohort,
    ) -> Result<SnapshotAdmissionObservation>;
}

pub trait ChaosControlSnapshotDescriptorPort {
    fn observe_descriptor(&mut self, descriptor: &SnapshotDescriptor) -> Result<ChaosControlDescriptorObservation>;
}

pub trait SnapshotHostHandlePort {
    fn recreate_handles(&mut self, descriptor_ref: &str) -> Result<String>;
}

pub trait LogicalSnapshotRestorePort {
    fn restore_component(
        &mut self,
        step: SnapshotRestoreStep,
        component: &SnapshotComponent,
        materialization: &SnapshotMaterializationObservation,
    ) -> Result<SnapshotStepObservation>;

    fn activate(&mut self, descriptor_ref: &str) -> Result<String>;
}

pub trait OpaqueSnapshotRestorePort {
    fn restore_exact(
        &mut self,
        descriptor: &SnapshotDescriptor,
        destination: &SnapshotCohort,
    ) -> Result<Vec<SnapshotStepObservation>>;

    fn activate(&mut self, descriptor_ref: &str) -> Result<String>;
}

pub trait SnapshotObservationPort {
    fn publish_observation(&mut self, observation: &SnapshotStepObservation) -> Result<()>;
}

pub trait SnapshotReceiptPort {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_bytes: &[u8]) -> Result<()>;
}
