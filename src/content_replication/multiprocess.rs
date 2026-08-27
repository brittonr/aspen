#![allow(
    tigerstyle::non_trait_imports,
    reason = "the adapter stores explicit payload and manifest bindings in deterministic maps"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use molten_core::content_replication::*;

use super::*;
use crate::cluster_harness::DistinctProcessTransportRunInput;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_durability::*;

const DURABLE_PROFILE_REF: &str = "blake3:7171717171717171717171717171717171717171717171717171717171717171";
const DURABLE_VALUE_SCHEMA_REF: &str = "blake3:7272727272727272727272727272727272727272727272727272727272727272";
const DURABLE_AUTHORITY_REF: &str = "blake3:7373737373737373737373737373737373737373737373737373737373737373";
const DURABLE_PROFILE_LIMIT: u64 = 4_096;
const DURABLE_OPERATION_BYTE_LIMIT: u64 = 1_048_576;
const DURABLE_NAMESPACE_BYTE_LIMIT: u64 = 16_777_216;

pub struct DistinctProcessTransferAdapter {
    run_root: PathBuf,
    process_binary: PathBuf,
    child_timeout_ms: u64,
    manifest_refs: BTreeMap<String, String>,
    payloads: BTreeMap<String, Vec<u8>>,
    generation: u64,
    membership_epoch: u64,
    placement_epoch: u64,
    call_count: usize,
}

impl DistinctProcessTransferAdapter {
    pub fn open(
        manifest: &Manifest,
        run_root: PathBuf,
        process_binary: PathBuf,
        child_timeout_ms: u64,
        payloads: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        if run_root.as_os_str().is_empty() || process_binary.as_os_str().is_empty() {
            return Err(MoltenError::invalid_harness(
                "multiprocess replication requires explicit run root and process binary",
            ));
        }
        for content in &manifest.contents {
            let payload = payloads.get(&content.content_ref).ok_or_else(|| {
                MoltenError::invalid_harness("multiprocess replication lacks a declared content payload")
            })?;
            let actual_ref = crate::preserves_rail::content_ref_from_bytes(payload);
            if actual_ref != content.content_ref {
                return Err(MoltenError::invalid_harness(
                    "multiprocess replication payload does not match its content identity",
                ));
            }
            let payload_bytes = u64::try_from(payload.len())
                .map_err(|_| MoltenError::invalid_harness("multiprocess payload length exceeds u64"))?;
            if payload_bytes != content.encoded_bytes {
                return Err(MoltenError::invalid_harness(
                    "multiprocess replication payload length does not match the content rule",
                ));
            }
        }
        Ok(Self {
            run_root,
            process_binary,
            child_timeout_ms,
            manifest_refs: manifest
                .contents
                .iter()
                .map(|content| (content.content_ref.clone(), content.manifest_ref.clone()))
                .collect(),
            payloads,
            generation: manifest.generation,
            membership_epoch: manifest.membership_epoch,
            placement_epoch: manifest.placement_epoch,
            call_count: 0,
        })
    }

    pub const fn call_count(&self) -> usize {
        self.call_count
    }
}

impl TransportPort for DistinctProcessTransferAdapter {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome> {
        let payload = self
            .payloads
            .get(&action.content_ref)
            .cloned()
            .ok_or_else(|| MoltenError::invalid_harness("multiprocess transfer lacks payload bytes"))?;
        let operation_leaf = action
            .operation_id
            .strip_prefix("blake3:")
            .ok_or_else(|| MoltenError::invalid_harness("multiprocess operation is not a BLAKE3 reference"))?;
        if !operation_leaf.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
            return Err(MoltenError::invalid_harness("multiprocess operation leaf is not canonical lowercase hex"));
        }
        let run = crate::cluster_harness::execute_distinct_process_transport_run(&DistinctProcessTransportRunInput {
            run_directory: self.run_root.join(operation_leaf),
            process_binary: self.process_binary.clone(),
            child_timeout_ms: self.child_timeout_ms,
            force: false,
            request_ref: action.operation_id.clone(),
            payload,
        })?;
        if run.decision != "pass" || !run.diagnostics.is_empty() {
            return Err(MoltenError::invalid_harness(format!(
                "multiprocess replication transport denied: {:?}",
                run.diagnostics
            )));
        }
        self.call_count = self
            .call_count
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("multiprocess transfer call count overflow"))?;
        let manifest_ref = self
            .manifest_refs
            .get(&action.content_ref)
            .cloned()
            .ok_or_else(|| MoltenError::invalid_harness("multiprocess transfer lacks manifest binding"))?;
        Ok(TransferOutcome::Received(TransferEnvelope {
            transfer_ref: run.parent_ref,
            transport_verification_ref: run.verification_ref,
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            manifest_ref,
            source_peer: action.source_peer.clone().unwrap_or_default(),
            target_peer: action.target_peer.clone(),
            generation: self.generation,
            membership_epoch: self.membership_epoch,
            placement_epoch: self.placement_epoch,
            encoded_bytes: action.encoded_bytes,
            protected: action.preserve_protected_form,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct SimulatedDurableReplicationAdapter {
    inner: SimulatedDurableStateAdapter,
    history: Vec<PriorOperation>,
    next_fault: Option<SimulatedDurabilityFault>,
}

impl SimulatedDurableReplicationAdapter {
    pub fn open(manifest: &Manifest, next_fault: Option<SimulatedDurabilityFault>) -> Result<Self> {
        let profile = canonical_durable_profile(&DurableStateProfile {
            schema: DURABLE_STATE_PROFILE_SCHEMA.to_string(),
            profile_id: "content-replication-simulation-v1".to_string(),
            profile_ref: DURABLE_PROFILE_REF.to_string(),
            adapter_kind: DurableAdapterKind::DeterministicSimulation,
            supported_levels: vec![
                DurabilityLevel::Buffered,
                DurabilityLevel::ProcessLoss,
                DurabilityLevel::MachineLoss,
            ],
            max_namespaces: DURABLE_PROFILE_LIMIT,
            max_log_records: DURABLE_PROFILE_LIMIT,
            max_ordered_entries: DURABLE_PROFILE_LIMIT,
            max_operation_bytes: DURABLE_OPERATION_BYTE_LIMIT,
            max_namespace_bytes: DURABLE_NAMESPACE_BYTE_LIMIT,
            max_batch_operations: DURABLE_PROFILE_LIMIT,
            max_snapshots: DURABLE_PROFILE_LIMIT,
            max_effect_transactions: DURABLE_PROFILE_LIMIT,
            non_claims: REQUIRED_DURABILITY_NON_CLAIMS.to_vec(),
        })?;
        let descriptor = DurableNamespaceDescriptor {
            schema: DURABLE_STATE_NAMESPACE_SCHEMA.to_string(),
            profile_ref: DURABLE_PROFILE_REF.to_string(),
            adapter_id: "content-replication-simulation".to_string(),
            namespace_id: manifest.service_id.clone(),
            generation: manifest.generation,
            value_schema_ref: DURABLE_VALUE_SCHEMA_REF.to_string(),
            atomicity_domain: AtomicityDomain {
                domain_id: "content-replication-operations".to_string(),
                adapter_id: "content-replication-simulation".to_string(),
                namespace_id: manifest.service_id.clone(),
                generation: manifest.generation,
                object_classes: vec![DurableObjectClass::OrderedValue],
                max_operations: DURABLE_PROFILE_LIMIT,
                max_bytes: DURABLE_OPERATION_BYTE_LIMIT,
                supported_levels: vec![DurabilityLevel::ProcessLoss],
            },
            retention_authority_ref: Some(DURABLE_AUTHORITY_REF.to_string()),
            quota_bytes: DURABLE_NAMESPACE_BYTE_LIMIT,
        };
        Ok(Self {
            inner: SimulatedDurableStateAdapter::new(profile, descriptor)?,
            history: Vec::new(),
            next_fault,
        })
    }

    pub fn with_history(mut self, history: Vec<PriorOperation>) -> Self {
        self.history = history;
        self
    }

    pub fn history(&self) -> &[PriorOperation] {
        &self.history
    }

    fn store_record(&mut self, key: &[u8], record: &CanonicalReplicationRecord) -> Result<String> {
        let request = AtomicBatchRequest {
            domain: self.inner.state().descriptor.atomicity_domain.clone(),
            generation: self.inner.state().descriptor.generation,
            mutations: vec![OrderedMutation::Put {
                key: key.to_vec(),
                value: record.bytes.clone(),
                value_ref: record.record_ref.clone(),
                precondition: ValuePrecondition::Missing,
            }],
            durability: DurabilityLevel::ProcessLoss,
        };
        let transition = self.inner.apply_batch(&request, self.next_fault.take().as_ref())?;
        if transition.outcome != MutationOutcome::Durable {
            return Err(MoltenError::invalid_harness(format!(
                "simulated replication durability did not commit: {:?}",
                transition.outcome
            )));
        }
        Ok(transition.transition_ref)
    }
}

impl DurablePort for SimulatedDurableReplicationAdapter {
    fn load_history(&mut self, _manifest: &Manifest) -> Result<Vec<PriorOperation>> {
        Ok(self.history.clone())
    }

    fn store_operation(&mut self, operation: &PriorOperation) -> Result<String> {
        let canonical = canonical_operation(operation)?;
        let durable_ref = self.store_record(operation.operation_id.as_bytes(), &canonical)?;
        self.history.push(operation.clone());
        Ok(durable_ref)
    }

    fn store_status(&mut self, status: &CanonicalReplicationRecord) -> Result<String> {
        self.store_record(status.record_ref.as_bytes(), status)
    }
}
