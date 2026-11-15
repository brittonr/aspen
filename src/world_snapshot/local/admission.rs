use ed25519_dalek::Signer;
use ed25519_dalek::Verifier;
use serde::Deserialize;
use serde::Serialize;

use super::SnapshotAdmissionObservation;
use super::local::LocalLogicalSnapshot;
use crate::error::Failure;
use crate::error::Result;
use crate::node_state::RelativePath;

const ADMISSION_SCHEMA: &str = "molten.runtime-spine-admission.v2";

/// Independently provisioned live local authority, never read from the bundle.
/// Updating revocation/generation requires replacing this complete document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCurrentAdmission {
    pub policy_ref: String,
    pub capability_ref: String,
    pub revocation_ref: String,
    pub resource_ref: String,
    pub adapter_ref: String,
    pub generation: u64,
    pub allowed: bool,
    pub expires_unix_seconds: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedAdmission {
    schema: String,
    descriptor_ref: String,
    profile_ref: String,
    cohort_ref: String,
    destination_ref: String,
    current: LocalCurrentAdmission,
    signature: String,
}

pub struct LocalAdmissionInput<'a> {
    pub signer: &'a ed25519_dalek::SigningKey,
    pub descriptor_ref: &'a str,
    pub profile_ref: &'a str,
    pub cohort_ref: &'a str,
    pub destination_ref: &'a str,
    pub current: LocalCurrentAdmission,
}

pub fn sign_local_admission(input: LocalAdmissionInput<'_>) -> Result<Vec<u8>> {
    let mut grant = SignedAdmission {
        schema: ADMISSION_SCHEMA.to_string(),
        descriptor_ref: input.descriptor_ref.to_string(),
        profile_ref: input.profile_ref.to_string(),
        cohort_ref: input.cohort_ref.to_string(),
        destination_ref: input.destination_ref.to_string(),
        current: input.current,
        signature: String::new(),
    };
    grant.signature = hex::encode(input.signer.sign(&message(&grant)?).to_bytes());
    serde_json::to_vec(&grant).map_err(admission_error)
}

impl LocalLogicalSnapshot {
    pub(super) fn admitted(
        &self,
        descriptor: &molten_core::world_snapshot::SnapshotDescriptor,
        descriptor_ref: &str,
    ) -> Result<SnapshotAdmissionObservation> {
        if super::local::destination_identity(&self.root)? != self.destination_ref {
            return Err(Failure::invalid_harness("destination node identity/configuration changed"));
        }
        let storage = self.root.namespace(crate::node_state::NamespaceKind::Storage)?;
        let grant: SignedAdmission =
            serde_json::from_slice(&storage.read(&RelativePath::parse("world-snapshot/admission.json")?, 16_384)?)
                .map_err(admission_error)?;
        let current: LocalCurrentAdmission =
            serde_json::from_slice(&storage.read(&RelativePath::parse("world-snapshot/current.json")?, 16_384)?)
                .map_err(admission_error)?;
        let now = crate::world_witness::WallClock::unix_seconds(&mut crate::world_witness::live_clock()?)?;
        if grant.schema != ADMISSION_SCHEMA
            || grant.destination_ref != self.destination_ref
            || grant.descriptor_ref != descriptor_ref
            || grant.profile_ref != descriptor.profile_ref.as_str()
            || grant.cohort_ref != descriptor.cohort.cohort_ref.as_str()
            || grant.current != current
            || !current.allowed
            || current.generation == 0
            || current.expires_unix_seconds <= now
        {
            return Err(Failure::invalid_harness(
                "snapshot current policy/capability/revocation/resource/generation/adapter admission denied or drifted",
            ));
        }
        let message = message(&grant)?;
        let signature = hex::decode(&grant.signature).map_err(admission_error)?;
        let signature = ed25519_dalek::Signature::from_slice(&signature).map_err(admission_error)?;
        self.trusted_signer.verify(&message, &signature).map_err(admission_error)?;
        let admission_ref = crate::preserves_rail::content_ref_from_bytes(&message);
        Ok(SnapshotAdmissionObservation {
            admission_ref,
            descriptor_ref: grant.descriptor_ref,
            profile_ref: grant.profile_ref,
            cohort_ref: grant.cohort_ref,
            generation: current.generation,
            allowed: true,
        })
    }

    pub(super) fn current_policy_ref(&self) -> Result<String> {
        let storage = self.root.namespace(crate::node_state::NamespaceKind::Storage)?;
        let current: LocalCurrentAdmission =
            serde_json::from_slice(&storage.read(&RelativePath::parse("world-snapshot/current.json")?, 16_384)?)
                .map_err(admission_error)?;
        Ok(current.policy_ref)
    }
}

fn message(grant: &SignedAdmission) -> Result<Vec<u8>> {
    let current = &grant.current;
    let references = [
        &grant.descriptor_ref,
        &grant.profile_ref,
        &grant.cohort_ref,
        &grant.destination_ref,
        &current.policy_ref,
        &current.capability_ref,
        &current.revocation_ref,
        &current.resource_ref,
        &current.adapter_ref,
    ];
    for reference in references {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("runtime-spine-admission-v2", vec![
        crate::preserves_rail::string(&grant.schema),
        crate::preserves_rail::sequence(references.into_iter().map(crate::preserves_rail::string).collect()),
        crate::preserves_rail::u64_value(current.generation),
        crate::preserves_rail::bool_value(current.allowed),
        crate::preserves_rail::u64_value(current.expires_unix_seconds),
    ]))
}

fn admission_error(error: impl std::fmt::Display) -> Failure {
    Failure::invalid_harness(format!("local runtime admission: {error}"))
}
