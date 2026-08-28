use std::collections::BTreeMap;

use molten_core::world_commit::*;
use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use crate::error::Result;

#[derive(Default)]
pub(super) struct Materialization {
    pub members: usize,
}

impl WorldReplayMaterializationPort for Materialization {
    fn materialize(&mut self, member: &WorldReplayCapsuleMember) -> Result<WorldReplayMaterializationObservation> {
        self.members = self.members.checked_add(1).expect("bounded fixture count");
        Ok(WorldReplayMaterializationObservation {
            object_ref: member.object_ref.clone(),
            observation_ref: digest(&format!("materialized-{}", member.object_ref)),
            available: true,
            identity_verified: true,
        })
    }
}

#[derive(Default)]
pub(super) struct Restore {
    pub logical_calls: usize,
    pub opaque_calls: usize,
    pub logical_fallback: bool,
}

impl WorldReplayRestorePort for Restore {
    fn restore_logical(
        &mut self,
        profile: &WorldReplayProfile,
        _initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation> {
        self.logical_calls = self.logical_calls.checked_add(1).expect("bounded fixture count");
        Ok(restore_observation(profile, false))
    }

    fn restore_opaque_exact(
        &mut self,
        profile: &WorldReplayProfile,
        _initial_commit: &crate::world_commit::CanonicalWorldCommit,
    ) -> Result<WorldReplayRestoreObservation> {
        self.opaque_calls = self.opaque_calls.checked_add(1).expect("bounded fixture count");
        Ok(restore_observation(profile, self.logical_fallback))
    }
}

pub(super) struct Admission {
    pub allowed: bool,
}

impl WorldReplayAdmissionPort for Admission {
    fn observe_current(
        &mut self,
        trace: &WorldTransitionTrace,
        capsule: &WorldReplayCapsule,
    ) -> Result<WorldReplayAdmissionObservation> {
        Ok(WorldReplayAdmissionObservation {
            observation_ref: digest("current-admission"),
            trace_ref: trace.trace_ref.clone(),
            capsule_ref: capsule.capsule_ref.clone(),
            profile_ref: trace.profile.profile_ref.as_str().to_string(),
            generation: ADMISSION_GENERATION,
            authority_admitted: self.allowed,
            artifact_admitted: true,
            schema_admitted: true,
            resource_admitted: true,
            runtime_admitted: true,
            effect_admitted: true,
        })
    }
}

#[derive(Default)]
pub(super) struct Transitions {
    pub positions: Vec<u64>,
}

impl WorldReplayTransitionPort for Transitions {
    fn execute_transition(&mut self, step: &WorldTransitionStep) -> Result<WorldReplayExecutionObservation> {
        self.positions.push(step.position);
        Ok(WorldReplayExecutionObservation {
            observation_ref: digest(&format!("execute-{}", step.position)),
            position: step.position,
            input_ref: step.input.input_ref.clone(),
        })
    }
}

pub(super) struct Capture {
    roots: BTreeMap<String, Vec<WorldRootRef>>,
    pub diverge_at: Option<u64>,
}

impl Capture {
    pub fn from_fixture(fixture: &Fixture) -> Self {
        Self {
            roots: fixture
                .request
                .commits
                .iter()
                .map(|commit| (commit.commit_ref.as_str().to_string(), commit.roots.clone()))
                .collect(),
            diverge_at: None,
        }
    }
}

impl WorldReplayCapturePort for Capture {
    fn capture_successor(
        &mut self,
        step: &WorldTransitionStep,
        _execution: &WorldReplayExecutionObservation,
    ) -> Result<WorldReplayCaptureObservation> {
        let is_diverged = self.diverge_at == Some(step.position);
        let actual_ref = if is_diverged {
            WorldCommitRef::new(digest("diverged-successor")).expect("commit ref")
        } else {
            step.expected_successor.clone()
        };
        let mut roots = self.roots.get(step.expected_successor.as_str()).cloned().expect("expected roots");
        let field_differences = if is_diverged {
            replace_artifact_root(&mut roots);
            vec![WorldReplayFieldDifference {
                root_kind: RootKind::Artifact,
                field_path: vec!["state".to_string(), "counter".to_string()],
            }]
        } else {
            Vec::new()
        };
        Ok(WorldReplayCaptureObservation {
            observation_ref: digest(&format!("capture-{}", step.position)),
            transition: WorldReplayTransitionObservation {
                position: step.position,
                observed_parent: step.expected_parent.clone(),
                actual: WorldReplayObservedCommit {
                    commit_ref: actual_ref,
                    roots,
                },
                field_differences,
            },
        })
    }
}

#[derive(Default)]
pub(super) struct Receipts {
    pub kinds: Vec<&'static str>,
}

impl WorldReplayReceiptPort for Receipts {
    fn publish(&mut self, record: &CanonicalWorldReplayRecord) -> Result<String> {
        self.kinds.push(record.kind);
        Ok(record.record_ref.clone())
    }
}

#[derive(Default)]
pub(super) struct ImportValidation {
    pub denied_ref: Option<String>,
    pub sensitive: bool,
    pub bearer: bool,
    pub decryption_available: bool,
    pub fail_canonical: bool,
    pub fail_identity: bool,
}

impl WorldReplayImportValidationPort for ImportValidation {
    fn verify_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        _payload: &WorldReplayMemberPayload,
    ) -> Result<WorldReplayImportVerification> {
        let is_denied = self.denied_ref.as_deref() == Some(member.object_ref.as_str());
        Ok(WorldReplayImportVerification {
            object_ref: member.object_ref.clone(),
            observation_ref: digest(&format!("verified-{}", member.object_ref)),
            byte_length: member.byte_length,
            canonical: !is_denied || !self.fail_canonical,
            identity_verified: !is_denied || !self.fail_identity,
            sensitive_plaintext_found: is_denied && self.sensitive,
            bearer_material_found: is_denied && self.bearer,
            decryption_available: !is_denied || self.decryption_available,
        })
    }
}

#[derive(Default)]
pub(super) struct ImportPublication {
    pub staged: usize,
    pub available: usize,
}

impl WorldReplayImportPublicationPort for ImportPublication {
    fn stage_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        _payload: &WorldReplayMemberPayload,
        _verification: &WorldReplayImportVerification,
    ) -> Result<String> {
        self.staged = self.staged.checked_add(1).expect("bounded fixture count");
        Ok(digest(&format!("staged-{}", member.object_ref)))
    }

    fn publish_available(&mut self, capsule_ref: &str, _staged_refs: &[String]) -> Result<String> {
        self.available = self.available.checked_add(1).expect("bounded fixture count");
        Ok(digest(&format!("available-{capsule_ref}")))
    }
}

#[derive(Default)]
pub(super) struct Exchange {
    pub exported: usize,
}

impl WorldReplayExchangePort for Exchange {
    fn export_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        _payload: &WorldReplayMemberPayload,
    ) -> Result<WorldReplayExchangeObservation> {
        self.exported = self.exported.checked_add(1).expect("bounded fixture count");
        Ok(WorldReplayExchangeObservation {
            object_ref: member.object_ref.clone(),
            observation_ref: digest(&format!("exported-{}", member.object_ref)),
            locator_hint: format!("detached:{}", member.object_ref),
        })
    }

    fn import_member(
        &mut self,
        member: &WorldReplayCapsuleMember,
        _locator_hint: &str,
    ) -> Result<WorldReplayMemberPayload> {
        Ok(WorldReplayMemberPayload {
            object_ref: member.object_ref.clone(),
            bytes: vec![0; usize::try_from(member.byte_length).expect("fixture length")],
        })
    }
}

fn restore_observation(profile: &WorldReplayProfile, logical_fallback_used: bool) -> WorldReplayRestoreObservation {
    WorldReplayRestoreObservation {
        observation_ref: digest("restore-observation"),
        profile_ref: profile.profile_ref.as_str().to_string(),
        cohort_ref: profile.cohort_ref.as_ref().map(|reference| reference.as_str().to_string()),
        logical_fallback_used,
    }
}

fn replace_artifact_root(roots: &mut [WorldRootRef]) {
    let artifact = roots.iter_mut().find(|root| root.kind() == RootKind::Artifact).expect("artifact root");
    *artifact = WorldRootRef::parse(RootKind::Artifact, digest("diverged-artifact")).expect("artifact ref");
}
