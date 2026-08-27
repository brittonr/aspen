#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::fd::AsRawFd;
use std::path::PathBuf;

use molten_core::content_replication::*;
use molten_core::content_store_adapter::*;

use super::super::*;
use super::support::*;
use crate::content_store_adapter::*;
use crate::error::MoltenError;
use crate::error::Result;

const CONTENT_CHUNK_COUNT: usize = 1;
const CONTENT_OPERATION_LIMIT: usize = 4;
const CONTENT_EVENT_LIMIT: usize = 16;
const CONTENT_DEADLINE_TICKS: u64 = 64;
const CONTENT_RETRY_LIMIT: u32 = 4;
const CONTENT_BYTES_VALUE: u8 = b'x';

pub struct SimulatedContent {
    profile: ContentAdapterProfile,
    descriptor: ContentManifestDescriptor,
    chunks: BTreeMap<String, Vec<u8>>,
    inventory: Inventory,
    fault: Option<SimulationFault>,
    events: Events,
}

impl SimulatedContent {
    pub fn new(manifest: &Manifest, events: &Events, fault: Option<SimulationFault>) -> Result<Self> {
        let bytes = content_bytes()?;
        let chunk_size = usize::try_from(CONTENT_BYTES)
            .map_err(|_| MoltenError::invalid_harness("content fixture size exceeds usize"))?;
        let chunk_ref = crate::chunk_store::hash_chunk(&bytes, chunk_size);
        let descriptor = descriptor(manifest, chunk_ref.clone());
        let profile = profile(manifest, ContentAdapterClass::DeterministicSimulation)?;
        Ok(Self {
            profile,
            descriptor,
            chunks: BTreeMap::from([(chunk_ref, bytes)]),
            inventory: Inventory {
                replicas: vec![source_replica(manifest)],
            },
            fault,
            events: events.clone(),
        })
    }
}

impl ContentPort for SimulatedContent {
    fn inventory(&mut self, _manifest: &Manifest) -> Result<Inventory> {
        self.events.borrow_mut().push("simulated-inventory");
        Ok(self.inventory.clone())
    }

    fn verify(&mut self, action: &Action, _envelope: &TransferEnvelope) -> Result<VerificationObservation> {
        self.events.borrow_mut().push("simulated-content");
        let command = command(&self.profile, &self.descriptor, action)?;
        let execution = execute_simulated_stream(
            &self.profile,
            &self.descriptor,
            &command,
            GENERATION,
            None,
            &self.chunks,
            self.fault,
        )?;
        if !content_is_available(&self.descriptor, &execution.state.artifact) {
            return Err(MoltenError::invalid_harness("simulated replication content did not become verified"));
        }
        verification(action, &self.descriptor, digest('1'))
    }

    fn cleanup(&mut self, _action: &Action, _admission: &CleanupObservation) -> Result<String> {
        Err(MoltenError::invalid_harness("simulation conformance does not execute cleanup"))
    }
}

pub struct LocalContent {
    _temp: cap_tempfile::TempDir,
    root: crate::chunk_store::CapabilityChunkRoot,
    profile: ContentAdapterProfile,
    descriptor: ContentManifestDescriptor,
    bytes: Vec<u8>,
    inventory: Inventory,
    events: Events,
}

impl LocalContent {
    pub fn new(manifest: &mut Manifest, events: &Events) -> Result<Self> {
        let temp = cap_tempfile::tempdir(cap_tempfile::ambient_authority()).map_err(MoltenError::from)?;
        let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", temp.as_raw_fd()));
        let host_path = std::fs::read_link(descriptor_path).map_err(MoltenError::from)?;
        let root = crate::chunk_store::CapabilityChunkRoot::open(&host_path)?;
        let bytes = content_bytes()?;
        let stored = crate::chunk_store::put_bytes_with_root(&root, "replication-fixture", &bytes, CONTENT_BYTES)?;
        let stored_manifest = crate::chunk_store::read_manifest_with_root(&root, &stored.manifest_ref)?;
        let descriptor = manifest_descriptor(&stored_manifest);
        manifest.contents[0].manifest_ref = descriptor.manifest_ref.clone();
        let profile = profile(manifest, ContentAdapterClass::CapabilityLocal)?;
        Ok(Self {
            _temp: temp,
            root,
            profile,
            descriptor,
            bytes,
            inventory: Inventory {
                replicas: vec![source_replica(manifest)],
            },
            events: events.clone(),
        })
    }
}

impl ContentPort for LocalContent {
    fn inventory(&mut self, _manifest: &Manifest) -> Result<Inventory> {
        self.events.borrow_mut().push("local-inventory");
        Ok(self.inventory.clone())
    }

    fn verify(&mut self, action: &Action, _envelope: &TransferEnvelope) -> Result<VerificationObservation> {
        self.events.borrow_mut().push("local-content");
        let command = command(&self.profile, &self.descriptor, action)?;
        let execution = execute_local_stream_get(&self.profile, &self.root, &command, GENERATION, None)?;
        let assembled =
            assemble_verified_content(&execution.manifest, &execution.state.artifact, &execution.verified_chunks)?;
        if assembled != self.bytes {
            return Err(MoltenError::invalid_harness("local replication content readback drifted"));
        }
        verification(action, &self.descriptor, digest('2'))
    }

    fn cleanup(&mut self, _action: &Action, _admission: &CleanupObservation) -> Result<String> {
        Err(MoltenError::invalid_harness("local conformance does not execute cleanup"))
    }
}

fn profile(manifest: &Manifest, class: ContentAdapterClass) -> Result<ContentAdapterProfile> {
    content_adapter_profile(
        "content-replication-conformance-v1",
        manifest.content_profile_ref.clone(),
        class,
        ContentResourceBounds {
            max_total_bytes: CONTENT_BYTES,
            max_chunk_count: CONTENT_CHUNK_COUNT,
            max_chunk_bytes: CONTENT_BYTES,
            max_range_bytes: CONTENT_BYTES,
            max_concurrent_operations: CONTENT_OPERATION_LIMIT,
            max_queued_bytes: CONTENT_BYTES,
            max_memory_bytes: CONTENT_BYTES,
            max_deadline_ticks: CONTENT_DEADLINE_TICKS,
            max_retries: CONTENT_RETRY_LIMIT,
            max_events: CONTENT_EVENT_LIMIT,
            max_status_entries: CONTENT_EVENT_LIMIT,
        },
        vec![manifest.evidence_profile_ref.clone()],
    )
}

fn descriptor(manifest: &Manifest, chunk_ref: String) -> ContentManifestDescriptor {
    ContentManifestDescriptor {
        manifest_ref: manifest.contents[0].manifest_ref.clone(),
        total_length: CONTENT_BYTES,
        chunker: "fixed-v1".to_string(),
        chunk_size: CONTENT_BYTES,
        metadata_ref: digest('0'),
        policy_refs: vec![manifest.retention_policy_ref.clone()],
        evidence_refs: vec![manifest.evidence_profile_ref.clone()],
        chunks: vec![ContentChunkDescriptor {
            chunk_ref,
            length: CONTENT_BYTES,
            position: 0,
            transform: "identity".to_string(),
        }],
    }
}

fn command(
    profile: &ContentAdapterProfile,
    descriptor: &ContentManifestDescriptor,
    action: &Action,
) -> Result<ContentCommand> {
    content_command(profile, ContentCommandInput {
        operation_ref: action.operation_id.clone(),
        operation: ContentOperation::Get,
        manifest: descriptor,
        range: None,
        submitted_tick: 1,
        deadline_tick: CONTENT_DEADLINE_TICKS,
        retry_count: action.attempt.saturating_sub(1),
        cancelled: false,
        policy_refs: vec![digest('5')],
    })
}

fn verification(
    action: &Action,
    descriptor: &ContentManifestDescriptor,
    verification_ref: String,
) -> Result<VerificationObservation> {
    Ok(VerificationObservation {
        verification_ref,
        operation_id: action.operation_id.clone(),
        replica: Replica {
            content_ref: action.content_ref.clone(),
            peer_id: action.target_peer.clone(),
            fault_domain: action.fault_domain.clone(),
            generation: GENERATION,
            membership_epoch: MEMBERSHIP_EPOCH,
            placement_epoch: PLACEMENT_EPOCH,
            present: true,
            identity_verified: true,
            pinned: true,
            protected: action.preserve_protected_form,
            manifest_ref: descriptor.manifest_ref.clone(),
            cleanup_clearance_ref: None,
        },
        identity_verified: true,
        authorization_admitted: true,
    })
}

fn content_bytes() -> Result<Vec<u8>> {
    let length = usize::try_from(CONTENT_BYTES)
        .map_err(|_| MoltenError::invalid_harness("content fixture size exceeds usize"))?;
    Ok(vec![CONTENT_BYTES_VALUE; length])
}
