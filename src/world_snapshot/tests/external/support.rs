use std::collections::VecDeque;

use chaoscontrol_snapshot_descriptor as chaos;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;

use super::super::super::*;
use crate::error::MoltenError;
use crate::error::Result;

const DIGEST_HEX_LENGTH: usize = 64;
const FIRST_GENERATION: u64 = 7;
const SECOND_GENERATION: u64 = 8;
#[cfg(feature = "world-snapshot-vm-cohort")]
const CLONE_COUNT: u32 = 2;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_VCPUS: u32 = 8;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_MEMORY_BYTES: u64 = 1_073_741_824;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_STORAGE_BYTES: u64 = 1_073_741_824;
#[cfg(feature = "world-snapshot-vm-cohort")]
pub(super) const EFFECTIVE_DISK_BYTES: u64 = 16_777_216;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_MAPPINGS: u32 = 16;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_ENDPOINTS: u32 = 8;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_IN_FLIGHT: u32 = 16;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_ATTEMPTS: u32 = 3;
#[cfg(feature = "world-snapshot-vm-cohort")]
const RETRY_DELAY_TICKS: u64 = 2;
#[cfg(feature = "world-snapshot-vm-cohort")]
const MAXIMUM_RETRY_DELAY_TICKS: u64 = 8;
#[cfg(feature = "world-snapshot-vm-cohort")]
const PAGE_BYTES: u64 = 4_096;

pub(super) fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

pub(super) fn fixture() -> (chaos::SnapshotDescriptorEnvelope, SnapshotDescriptor) {
    let descriptor: chaos::SnapshotDescriptor = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/world-snapshot/chaoscontrol-snapshot-descriptor.valid.json"
    ))
    .expect("valid ChaosControl descriptor fixture");
    let descriptor_id = chaos::descriptor_identity(&descriptor).expect("descriptor identity");
    let envelope = chaos::SnapshotDescriptorEnvelope {
        descriptor_id,
        descriptor,
    };
    let mapped = map_chaoscontrol_snapshot(&envelope, binding()).expect("map ChaosControl descriptor");
    (envelope, mapped)
}

pub(super) fn binding() -> ChaosControlSnapshotBinding {
    ChaosControlSnapshotBinding {
        commit_ref: WorldCommitRef::new(digest('a')).expect("commit ref"),
        profile_ref: SnapshotProfileRef::new(digest('b')).expect("profile ref"),
        artifact_root: root(RootKind::Artifact, 'c'),
        schema_root: root(RootKind::Schema, 'd'),
        runtime_profile_root: root(RootKind::RuntimeProfile, 'e'),
        policy_root: root(RootKind::Policy, 'f'),
        runtime_abi_ref: digest('1'),
        memory_format_ref: digest('2'),
        disk_format_ref: digest('3'),
        backend_profile_ref: digest('4'),
    }
}

#[cfg(feature = "world-snapshot-vm-cohort")]
pub(super) fn clone_request(descriptor: &SnapshotDescriptor) -> ClonePlanRequest {
    ClonePlanRequest {
        parent_ref: descriptor.commit_ref.clone(),
        children: (0..CLONE_COUNT)
            .map(|index| CloneChild {
                parent_ref: descriptor.commit_ref.clone(),
                memory_overlay: overlay("memory", index),
                device_overlay: overlay("device", index),
                disk_overlay: overlay("disk", index),
                endpoint_overlay: overlay("endpoint", index),
            })
            .collect(),
    }
}

#[cfg(feature = "world-snapshot-vm-cohort")]
pub(super) fn limits() -> VmCohortLimits {
    VmCohortLimits {
        policy_ref: digest('5'),
        maximum_workers: CLONE_COUNT,
        maximum_vcpus: MAXIMUM_VCPUS,
        maximum_memory_bytes: MAXIMUM_MEMORY_BYTES,
        maximum_storage_bytes: MAXIMUM_STORAGE_BYTES,
        maximum_mappings: MAXIMUM_MAPPINGS,
        maximum_endpoints: MAXIMUM_ENDPOINTS,
        maximum_in_flight: MAXIMUM_IN_FLIGHT,
        max_attempts: MAXIMUM_ATTEMPTS,
        retry_delay_ticks: RETRY_DELAY_TICKS,
        max_retry_delay_ticks: MAXIMUM_RETRY_DELAY_TICKS,
        memory_page_bytes: PAGE_BYTES,
        disk_page_bytes: PAGE_BYTES,
    }
}

pub(super) fn machine_descriptor_ref(descriptor: &SnapshotDescriptor) -> String {
    descriptor
        .components
        .iter()
        .find(|component| component.kind == SnapshotComponentKind::MachineDescriptor)
        .expect("machine descriptor component")
        .identity
        .clone()
}

pub(super) struct Materializer;

impl SnapshotMaterializationPort for Materializer {
    fn observe_component(&mut self, component: &SnapshotComponent) -> Result<SnapshotMaterializationObservation> {
        Ok(SnapshotMaterializationObservation {
            component_identity: component.identity.clone(),
            observation_ref: digest('6'),
            available: true,
            identity_verified: true,
        })
    }
}

pub(super) struct ChaosObserver;

impl ChaosControlSnapshotDescriptorPort for ChaosObserver {
    fn observe_descriptor(&mut self, descriptor: &SnapshotDescriptor) -> Result<ChaosControlDescriptorObservation> {
        Ok(ChaosControlDescriptorObservation {
            descriptor_ref: machine_descriptor_ref(descriptor),
            cohort_ref: descriptor.cohort.cohort_ref.as_str().to_string(),
            available: true,
            identity_verified: true,
        })
    }
}

pub(super) struct Admission {
    generations: VecDeque<u64>,
}

impl Admission {
    pub(super) fn current() -> Self {
        Self {
            generations: VecDeque::from([FIRST_GENERATION, SECOND_GENERATION]),
        }
    }
}

impl CurrentSnapshotAdmissionPort for Admission {
    fn observe_current(
        &mut self,
        descriptor: &SnapshotDescriptor,
        descriptor_ref: &str,
        destination: &SnapshotCohort,
    ) -> Result<SnapshotAdmissionObservation> {
        Ok(SnapshotAdmissionObservation {
            admission_ref: digest('7'),
            descriptor_ref: descriptor_ref.to_string(),
            profile_ref: descriptor.profile_ref.as_str().to_string(),
            cohort_ref: destination.cohort_ref.as_str().to_string(),
            generation: self
                .generations
                .pop_front()
                .ok_or_else(|| MoltenError::invalid_harness("admission fixture exhausted"))?,
            allowed: true,
        })
    }
}

pub(super) struct Handles;

impl SnapshotHostHandlePort for Handles {
    fn recreate_handles(&mut self, _descriptor_ref: &str) -> Result<String> {
        Ok(digest('8'))
    }
}

pub(super) struct OpaqueRuntime;

impl OpaqueSnapshotRestorePort for OpaqueRuntime {
    fn restore_exact(
        &mut self,
        _descriptor: &SnapshotDescriptor,
        _destination: &SnapshotCohort,
    ) -> Result<Vec<SnapshotStepObservation>> {
        Ok(vec![SnapshotStepObservation {
            step: SnapshotRestoreStep::RestoreOpaqueMachine,
            observation_ref: digest('9'),
        }])
    }

    fn activate(&mut self, _descriptor_ref: &str) -> Result<String> {
        Ok(digest('a'))
    }
}

#[derive(Default)]
pub(super) struct Receipts {
    pub(super) published: usize,
}

impl SnapshotReceiptPort for Receipts {
    fn publish_receipt(&mut self, _receipt_ref: &str, _canonical_bytes: &[u8]) -> Result<()> {
        self.published = self
            .published
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("receipt count overflow"))?;
        Ok(())
    }
}

fn root(kind: RootKind, byte: char) -> WorldRootRef {
    WorldRootRef::parse(kind, digest(byte)).expect("typed root")
}

#[cfg(feature = "world-snapshot-vm-cohort")]
fn overlay(label: &str, index: u32) -> OverlayIdentity {
    OverlayIdentity(format!("{label}-{index}"))
}
