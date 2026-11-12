use std::path::Path;

use molten_core::world_snapshot::SnapshotCohort;
use molten_core::world_snapshot::SnapshotComponent;
use molten_core::world_snapshot::SnapshotComponentKind;
use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::SnapshotRestoreStep;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use molten_node_host::node_state::RelativePath;
use molten_node_host::node_state::Scope;
use serde::Deserialize;
use serde::Serialize;

use super::CurrentSnapshotAdmissionPort;
use super::LogicalSnapshotRestorePort;
use super::SnapshotAdmissionObservation;
use super::SnapshotHostHandlePort;
use super::SnapshotMaterializationObservation;
use super::SnapshotMaterializationPort;
use super::SnapshotObservationPort;
use super::SnapshotReceiptPort;
use super::SnapshotStepObservation;
use super::localtree::MAX_LOCAL_FILE_BYTES;
use super::localtree::collect_tree;
use super::localtree::tree_identity;
use crate::error::Failure;
use crate::error::Result;

const HANDLE_DOMAIN: &str = "onixresearch.molten.world-snapshot.local-handle.v1";
const ADMISSION_SCHEMA: &str = "molten.local-snapshot-admission.v1";

/// Content-addressed file-tree bundle and restore target under one node storage namespace.
pub struct LocalLogicalSnapshot {
    storage: Scope,
    trusted_signer: ed25519_dalek::VerifyingKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedAdmission {
    schema: String,
    admission_ref: String,
    descriptor_ref: String,
    profile_ref: String,
    cohort_ref: String,
    generation: u64,
    signature: String,
}

impl LocalLogicalSnapshot {
    pub fn open(state_root: &Path, trusted_signer: ed25519_dalek::VerifyingKey) -> Result<Self> {
        Self::from_root(DirectoryRoot::open(state_root)?, trusted_signer)
    }

    pub fn from_root(root: DirectoryRoot, trusted_signer: ed25519_dalek::VerifyingKey) -> Result<Self> {
        Ok(Self {
            storage: root.namespace(NamespaceKind::Storage)?,
            trusted_signer,
        })
    }

    /// Identifies one component's bundle tree. Bytes live under
    /// `world-snapshot/bundle/<kind>/` before the descriptor is constructed.
    pub fn component_identity(&self, kind: SnapshotComponentKind) -> Result<String> {
        let Some(tree) = self.bundle_tree(kind)? else {
            return Err(Failure::invalid_harness("local snapshot component bundle is absent"));
        };
        Ok(tree_identity(&collect_tree(tree)?))
    }

    fn bundle_tree(&self, kind: SnapshotComponentKind) -> Result<Option<Scope>> {
        self.storage
            .existing_subdir(&RelativePath::parse(&format!("world-snapshot/bundle/{}", kind.as_str()))?)
    }

    fn admitted(&self, descriptor: &SnapshotDescriptor, descriptor_ref: &str) -> Result<SnapshotAdmissionObservation> {
        let bytes = self.storage.read(&RelativePath::parse("world-snapshot/admission.json")?, 16_384)?;
        let signed: SignedAdmission = serde_json::from_slice(&bytes)
            .map_err(|error| Failure::invalid_harness(format!("local snapshot admission: {error}")))?;
        if signed.schema != ADMISSION_SCHEMA
            || signed.descriptor_ref != descriptor_ref
            || signed.profile_ref != descriptor.profile_ref.as_str()
            || signed.cohort_ref != descriptor.cohort.cohort_ref.as_str()
        {
            return Err(Failure::invalid_harness("local snapshot admission is stale or crossed"));
        }
        verify_admission_signature(&self.trusted_signer, &signed)?;
        Ok(SnapshotAdmissionObservation {
            admission_ref: signed.admission_ref,
            descriptor_ref: signed.descriptor_ref,
            profile_ref: signed.profile_ref,
            cohort_ref: signed.cohort_ref,
            generation: signed.generation,
            allowed: true,
        })
    }
}

impl SnapshotMaterializationPort for LocalLogicalSnapshot {
    fn observe_component(&mut self, component: &SnapshotComponent) -> Result<SnapshotMaterializationObservation> {
        let identity = self.component_identity(component.kind)?;
        Ok(SnapshotMaterializationObservation {
            component_identity: component.identity.clone(),
            observation_ref: identity.clone(),
            available: true,
            identity_verified: identity == component.identity,
        })
    }
}

impl CurrentSnapshotAdmissionPort for LocalLogicalSnapshot {
    fn observe_current(
        &mut self,
        descriptor: &SnapshotDescriptor,
        descriptor_ref: &str,
        _destination: &SnapshotCohort,
    ) -> Result<SnapshotAdmissionObservation> {
        self.admitted(descriptor, descriptor_ref)
    }
}

impl SnapshotHostHandlePort for LocalLogicalSnapshot {
    fn recreate_handles(&mut self, descriptor_ref: &str) -> Result<String> {
        let digest = blake3::hash(format!("{HANDLE_DOMAIN}\n{descriptor_ref}").as_bytes()).to_hex().to_string();
        self.storage.open_subdir(&RelativePath::parse(&format!("world-snapshot/handles/{digest}"))?)?;
        Ok(format!("blake3:{digest}"))
    }
}

impl LogicalSnapshotRestorePort for LocalLogicalSnapshot {
    fn restore_component(
        &mut self,
        step: SnapshotRestoreStep,
        component: &SnapshotComponent,
        materialization: &SnapshotMaterializationObservation,
    ) -> Result<SnapshotStepObservation> {
        let source = self
            .bundle_tree(component.kind)?
            .ok_or_else(|| Failure::invalid_harness("local snapshot component bundle is absent"))?;
        let files = collect_tree(source)?;
        if tree_identity(&files) != materialization.observation_ref {
            return Err(Failure::invalid_harness("local snapshot component changed after materialization"));
        }
        let target = self
            .storage
            .open_subdir(&RelativePath::parse(&format!("world-snapshot/restored/{}", component.kind.as_str()))?)?;
        for (path, bytes) in &files {
            let path = RelativePath::parse(path)?;
            if let Some((directory, _)) = path.display().rsplit_once('/') {
                target.create_dir_all(&RelativePath::parse(directory)?)?;
            }
            target.write(&path, bytes)?;
        }
        Ok(SnapshotStepObservation {
            step,
            observation_ref: materialization.observation_ref.clone(),
        })
    }

    fn activate(&mut self, descriptor_ref: &str) -> Result<String> {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "molten.local-snapshot-active.v1",
            "descriptor_ref": descriptor_ref,
        }))
        .map_err(|error| Failure::invalid_harness(format!("local snapshot activation: {error}")))?;
        self.storage.create_dir_all(&RelativePath::parse("world-snapshot")?)?;
        self.storage.write(&RelativePath::parse("world-snapshot/active.json")?, &bytes)?;
        Ok(format!("blake3:{}", blake3::hash(&bytes).to_hex()))
    }
}

impl SnapshotObservationPort for LocalLogicalSnapshot {
    fn publish_observation(&mut self, observation: &SnapshotStepObservation) -> Result<()> {
        let bytes = format!("{}\n{}\n", observation.step.as_str(), observation.observation_ref).into_bytes();
        let path =
            RelativePath::parse(&format!("world-snapshot/observations/blake3:{}.json", blake3::hash(&bytes).to_hex()))?;
        self.storage.create_dir_all(&parent(&path)?)?;
        if self.storage.try_exists(&path)? {
            return Ok(());
        }
        self.storage.write(&path, &bytes)?;
        Ok(())
    }
}

impl SnapshotReceiptPort for LocalLogicalSnapshot {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_bytes: &[u8]) -> Result<()> {
        if !receipt_ref.starts_with("blake3:") || canonical_bytes.is_empty() {
            return Err(Failure::invalid_harness("local snapshot receipt is malformed"));
        }
        let path = RelativePath::parse(&format!(
            "world-snapshot/receipts/{}.preserves",
            receipt_ref.trim_start_matches("blake3:")
        ))?;
        self.storage.create_dir_all(&parent(&path)?)?;
        if self.storage.try_exists(&path)? {
            let existing = self.storage.read(&path, MAX_LOCAL_FILE_BYTES)?;
            return if existing == canonical_bytes {
                Ok(())
            } else {
                Err(Failure::invalid_harness("stored snapshot receipt does not match its content address"))
            };
        }
        self.storage.write(&path, canonical_bytes)?;
        Ok(())
    }
}

fn parent(path: &RelativePath) -> Result<RelativePath> {
    match path.display().rsplit_once('/') {
        Some((parent, _)) => RelativePath::parse(parent),
        None => RelativePath::parse(""),
    }
}

/// Signs one operator admission grant for a local snapshot descriptor.
pub fn sign_local_admission(
    signer: &ed25519_dalek::SigningKey,
    descriptor_ref: &str,
    profile_ref: &str,
    cohort_ref: &str,
    generation: u64,
) -> Result<Vec<u8>> {
    use ed25519_dalek::Signer;
    let mut fields = SignedAdmission {
        schema: ADMISSION_SCHEMA.to_string(),
        admission_ref: String::new(),
        descriptor_ref: descriptor_ref.to_string(),
        profile_ref: profile_ref.to_string(),
        cohort_ref: cohort_ref.to_string(),
        generation,
        signature: String::new(),
    };
    let message = admission_message(&fields);
    fields.admission_ref = format!("blake3:{}", blake3::hash(&message).to_hex());
    let mut signed_message = message;
    signed_message.extend_from_slice(fields.admission_ref.as_bytes());
    fields.signature = hex::encode(signer.sign(&signed_message).to_bytes());
    serde_json::to_vec(&fields).map_err(|error| Failure::invalid_harness(format!("local snapshot admission: {error}")))
}

fn admission_message(fields: &SignedAdmission) -> Vec<u8> {
    format!(
        "{ADMISSION_SCHEMA}\n{}\n{}\n{}\n{}",
        fields.descriptor_ref, fields.profile_ref, fields.cohort_ref, fields.generation
    )
    .into_bytes()
}

fn verify_admission_signature(signer: &ed25519_dalek::VerifyingKey, signed: &SignedAdmission) -> Result<()> {
    use ed25519_dalek::Verifier;
    let message = admission_message(signed);
    if signed.admission_ref != format!("blake3:{}", blake3::hash(&message).to_hex()) {
        return Err(Failure::invalid_harness("local snapshot admission reference is not canonical"));
    }
    let signature = hex::decode(&signed.signature)
        .map_err(|_| Failure::invalid_harness("local snapshot admission signature is not hex"))?;
    let signature = ed25519_dalek::Signature::from_slice(&signature)
        .map_err(|_| Failure::invalid_harness("local snapshot admission signature is malformed"))?;
    let mut signed_message = message;
    signed_message.extend_from_slice(signed.admission_ref.as_bytes());
    signer
        .verify(&signed_message, &signature)
        .map_err(|_| Failure::invalid_harness("local snapshot admission signature is not trusted"))
}
