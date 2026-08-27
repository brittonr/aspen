use std::collections::BTreeSet;

use preserves::IOValue;

use super::super::*;
use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

const ARTIFACT_INDEX_RECORD: &str = "native-host-artifact-index-v1";
const ARTIFACT_MEMBER_RECORD: &str = "native-host-artifact-member-v1";
const ARTIFACT_INDEX_SCHEMA: &str = "molten.system-extension.native-artifact-index.v1";
const MAX_ARTIFACT_MEMBERS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeArtifactRole {
    Executable,
    CallbackEnvelope,
    CallbackReceipt,
    ExecutionReceipt,
    InstanceState,
    Effect,
    Checkpoint,
    LifecycleEvidence,
}

impl NativeArtifactRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::CallbackEnvelope => "callback-envelope",
            Self::CallbackReceipt => "callback-receipt",
            Self::ExecutionReceipt => "execution-receipt",
            Self::InstanceState => "instance-state",
            Self::Effect => "effect",
            Self::Checkpoint => "checkpoint",
            Self::LifecycleEvidence => "lifecycle-evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArtifactMember {
    pub role: NativeArtifactRole,
    pub artifact_ref: String,
    pub parent_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeArtifactIndex {
    pub index_ref: String,
    pub manifest_ref: String,
    pub members: Vec<NativeArtifactMember>,
    pub non_claims: Vec<NativeHostNonClaim>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeArtifactIndexIssue {
    TooManyMembers { actual: usize, maximum: usize },
    DuplicateMember(String),
    MalformedRef { field: &'static str, value: String },
    MissingRole(NativeArtifactRole),
    ParentNotIndexed { artifact_ref: String, parent_ref: String },
    IndexIdentityMismatch,
    MissingNonClaim(NativeHostNonClaim),
}

// r[impl molten.system_extension.native_host.validation]
pub fn build_native_artifact_index(
    instance: &NativeInstanceRecord,
    observations: &[NativeInvocationObservation],
    callback_receipts: &[CanonicalCallbackReceipt],
    effect_completions: &[CanonicalEffectCompletion],
) -> Result<CanonicalNativeArtifactIndex> {
    let state = canonical_native_instance_record(instance)?;
    let estimated = observations
        .len()
        .checked_mul(2)
        .and_then(|value| value.checked_add(callback_receipts.len()))
        .and_then(|value| value.checked_add(effect_completions.len()))
        .and_then(|value| value.checked_add(instance.unresolved.len()))
        .and_then(|value| value.checked_add(instance.evidence_refs.len()))
        .and_then(|value| value.checked_add(3))
        .ok_or_else(|| MoltenError::invalid_harness("native artifact index member count overflow"))?;
    let mut members = Vec::with_capacity(estimated);
    members.push(NativeArtifactMember {
        role: NativeArtifactRole::Executable,
        artifact_ref: instance.executable_ref.clone(),
        parent_ref: instance.manifest_ref.clone(),
    });
    members.push(NativeArtifactMember {
        role: NativeArtifactRole::InstanceState,
        artifact_ref: state.record_ref.clone(),
        parent_ref: instance.manifest_ref.clone(),
    });
    if let Some(checkpoint_ref) = &instance.checkpoint_ref {
        members.push(NativeArtifactMember {
            role: NativeArtifactRole::Checkpoint,
            artifact_ref: checkpoint_ref.clone(),
            parent_ref: state.record_ref.clone(),
        });
    }
    for observation in observations {
        members.push(NativeArtifactMember {
            role: NativeArtifactRole::CallbackEnvelope,
            artifact_ref: observation.envelope_ref.clone(),
            parent_ref: instance.manifest_ref.clone(),
        });
        if let Some(receipt_ref) = &observation.execution_receipt_ref {
            members.push(NativeArtifactMember {
                role: NativeArtifactRole::ExecutionReceipt,
                artifact_ref: receipt_ref.clone(),
                parent_ref: observation.envelope_ref.clone(),
            });
        }
    }
    for receipt in callback_receipts {
        members.push(NativeArtifactMember {
            role: NativeArtifactRole::CallbackReceipt,
            artifact_ref: receipt.receipt_ref.clone(),
            parent_ref: instance.manifest_ref.clone(),
        });
    }
    for completion in effect_completions {
        members.push(NativeArtifactMember {
            role: NativeArtifactRole::Effect,
            artifact_ref: completion.completion_ref.clone(),
            parent_ref: completion.callback_receipt_ref.clone(),
        });
    }
    for operation in &instance.unresolved {
        if operation.kind == NativeOperationKind::Effect {
            members.push(NativeArtifactMember {
                role: NativeArtifactRole::Effect,
                artifact_ref: operation.operation_ref.clone(),
                parent_ref: operation.parent_ref.clone(),
            });
        }
    }
    let mut represented_refs = members.iter().map(|member| member.artifact_ref.clone()).collect::<BTreeSet<_>>();
    for evidence_ref in &instance.evidence_refs {
        if represented_refs.insert(evidence_ref.clone()) {
            members.push(NativeArtifactMember {
                role: NativeArtifactRole::LifecycleEvidence,
                artifact_ref: evidence_ref.clone(),
                parent_ref: state.record_ref.clone(),
            });
        }
    }
    members.sort_by(|left, right| (left.role, &left.artifact_ref).cmp(&(right.role, &right.artifact_ref)));
    let value = artifact_index_value(&instance.manifest_ref, &members, &REQUIRED_NATIVE_HOST_NON_CLAIMS);
    let index_ref = canonical_hash(&value)?;
    let index = CanonicalNativeArtifactIndex {
        index_ref,
        manifest_ref: instance.manifest_ref.clone(),
        members,
        non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        value,
    };
    verify_native_artifact_index(&index)
        .map_err(|issues| MoltenError::invalid_harness(format!("native artifact index denied: {issues:?}")))?;
    Ok(index)
}

// r[impl molten.system_extension.native_host.validation]
pub fn verify_native_artifact_index(
    index: &CanonicalNativeArtifactIndex,
) -> std::result::Result<(), Vec<NativeArtifactIndexIssue>> {
    let mut issues = Vec::new();
    if index.members.len() > MAX_ARTIFACT_MEMBERS {
        issues.push(NativeArtifactIndexIssue::TooManyMembers {
            actual: index.members.len(),
            maximum: MAX_ARTIFACT_MEMBERS,
        });
    }
    let mut refs = BTreeSet::new();
    for member in &index.members {
        if !refs.insert(member.artifact_ref.clone()) {
            issues.push(NativeArtifactIndexIssue::DuplicateMember(member.artifact_ref.clone()));
        }
        validate_ref("artifact-ref", &member.artifact_ref, &mut issues);
        validate_ref("parent-ref", &member.parent_ref, &mut issues);
    }
    validate_ref("manifest-ref", &index.manifest_ref, &mut issues);
    for role in [NativeArtifactRole::Executable, NativeArtifactRole::InstanceState] {
        if !index.members.iter().any(|member| member.role == role) {
            issues.push(NativeArtifactIndexIssue::MissingRole(role));
        }
    }
    for member in &index.members {
        let parent_is_root = member.parent_ref == index.manifest_ref;
        let parent_is_indexed = refs.contains(&member.parent_ref);
        if !parent_is_root && !parent_is_indexed {
            issues.push(NativeArtifactIndexIssue::ParentNotIndexed {
                artifact_ref: member.artifact_ref.clone(),
                parent_ref: member.parent_ref.clone(),
            });
        }
    }
    for required in REQUIRED_NATIVE_HOST_NON_CLAIMS {
        if !index.non_claims.contains(&required) {
            issues.push(NativeArtifactIndexIssue::MissingNonClaim(required));
        }
    }
    let expected_value = artifact_index_value(&index.manifest_ref, &index.members, &index.non_claims);
    if canonical_hash(&expected_value).ok().as_deref() != Some(&index.index_ref) || expected_value != index.value {
        issues.push(NativeArtifactIndexIssue::IndexIdentityMismatch);
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn artifact_index_value(
    manifest_ref: &str,
    members: &[NativeArtifactMember],
    non_claims: &[NativeHostNonClaim],
) -> IOValue {
    record(ARTIFACT_INDEX_RECORD, vec![
        string(ARTIFACT_INDEX_SCHEMA),
        string(manifest_ref),
        sequence(
            members
                .iter()
                .map(|member| {
                    record(ARTIFACT_MEMBER_RECORD, vec![
                        string(member.role.as_str()),
                        string(&member.artifact_ref),
                        string(&member.parent_ref),
                    ])
                })
                .collect(),
        ),
        sequence(non_claims.iter().map(|claim| string(claim.as_str())).collect()),
    ])
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<NativeArtifactIndexIssue>) {
    if crate::preserves_rail::ContentRef::parse(value).is_err() {
        issues.push(NativeArtifactIndexIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}
