use molten_core::world_replay::*;

use super::CanonicalWorldReplayRecord;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayMemberPayload {
    pub object_ref: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayMaterializationObservation {
    pub object_ref: String,
    pub observation_ref: String,
    pub available: bool,
    pub identity_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayRestoreObservation {
    pub observation_ref: String,
    pub profile_ref: String,
    pub cohort_ref: Option<String>,
    pub logical_fallback_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayAdmissionObservation {
    pub observation_ref: String,
    pub trace_ref: String,
    pub capsule_ref: String,
    pub profile_ref: String,
    pub generation: u64,
    pub authority_admitted: bool,
    pub artifact_admitted: bool,
    pub schema_admitted: bool,
    pub resource_admitted: bool,
    pub runtime_admitted: bool,
    pub effect_admitted: bool,
}

impl WorldReplayAdmissionObservation {
    pub const fn admitted(&self) -> bool {
        self.authority_admitted
            && self.artifact_admitted
            && self.schema_admitted
            && self.resource_admitted
            && self.runtime_admitted
            && self.effect_admitted
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayExecutionObservation {
    pub observation_ref: String,
    pub position: u64,
    pub input_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayCaptureObservation {
    pub observation_ref: String,
    pub transition: WorldReplayTransitionObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayImportVerification {
    pub object_ref: String,
    pub observation_ref: String,
    pub byte_length: u64,
    pub canonical: bool,
    pub identity_verified: bool,
    pub sensitive_plaintext_found: bool,
    pub bearer_material_found: bool,
    pub decryption_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayExchangeObservation {
    pub object_ref: String,
    pub observation_ref: String,
    pub locator_hint: String,
}

pub trait WorldReplayMaterializationPort {
    fn materialize(&mut self, member: &WorldReplayCapsuleMember) -> Result<WorldReplayMaterializationObservation>;
}

pub trait WorldReplayRestorePort {
    fn restore_logical(
        &mut self,
        profile: &WorldReplayProfile,
        initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation>;

    fn restore_opaque_exact(
        &mut self,
        profile: &WorldReplayProfile,
        initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation>;
}

pub trait WorldReplayAdmissionPort {
    fn observe_current(
        &mut self,
        trace: &WorldTransitionTrace,
        capsule: &WorldReplayCapsule,
    ) -> Result<WorldReplayAdmissionObservation>;
}

pub trait WorldReplayTransitionPort {
    fn execute_transition(&mut self, step: &WorldTransitionStep) -> Result<WorldReplayExecutionObservation>;
}

pub trait WorldReplayCapturePort {
    fn capture_successor(
        &mut self,
        step: &WorldTransitionStep,
        execution: &WorldReplayExecutionObservation,
    ) -> Result<WorldReplayCaptureObservation>;
}

pub trait WorldReplayImportValidationPort {
    fn verify_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        payload: &WorldReplayMemberPayload,
    ) -> Result<WorldReplayImportVerification>;
}

pub trait WorldReplayImportPublicationPort {
    fn stage_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        payload: &WorldReplayMemberPayload,
        verification: &WorldReplayImportVerification,
    ) -> Result<String>;

    fn publish_available(&mut self, capsule_ref: &str, staged_refs: &[String]) -> Result<String>;
}

pub trait WorldReplayExchangePort {
    fn export_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        payload: &WorldReplayMemberPayload,
    ) -> Result<WorldReplayExchangeObservation>;

    fn import_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        locator_hint: &str,
    ) -> Result<WorldReplayMemberPayload>;
}

pub trait WorldReplayReceiptPort {
    fn publish(&mut self, record: &CanonicalWorldReplayRecord) -> Result<String>;
}
