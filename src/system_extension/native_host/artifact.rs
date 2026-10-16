use super::super::*;
use super::*;

const ARTIFACT_INDEX_RECORD: &str = "native-host-artifact-index-v2";
const ARTIFACT_MEMBER_RECORD: &str = "native-host-artifact-member-v2";
const ARTIFACT_INDEX_SCHEMA: &str = "molten.system-extension.native-artifact-index.v2";
const MAX_ARTIFACT_MEMBERS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Executable,
    CallbackEnvelope,
    CallbackReceipt,
    ExecutionReceipt,
    InstanceState,
    SemanticState,
    ValuePublication,
    Effect,
    Checkpoint,
    LifecycleEvidence,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::CallbackEnvelope => "callback-envelope",
            Self::CallbackReceipt => "callback-receipt",
            Self::ExecutionReceipt => "execution-receipt",
            Self::InstanceState => "instance-state",
            Self::SemanticState => "semantic-state",
            Self::ValuePublication => "value-publication",
            Self::Effect => "effect",
            Self::Checkpoint => "checkpoint",
            Self::LifecycleEvidence => "lifecycle-evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub role: Role,
    pub artifact_ref: String,
    pub parent_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalIndex {
    pub index_ref: String,
    pub manifest_ref: String,
    pub members: Vec<Member>,
    pub non_claims: Vec<NativeHostNonClaim>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexIssue {
    TooManyMembers { actual: usize, maximum: usize },
    DuplicateMember(String),
    MalformedRef { field: &'static str, value: String },
    MissingRole(Role),
    ParentNotIndexed { artifact_ref: String, parent_ref: String },
    IndexIdentityMismatch,
    MissingNonClaim(NativeHostNonClaim),
}

// r[impl molten.system_extension.native_host.validation]
pub fn build_index(
    instance: &NativeInstanceRecord,
    observations: &[InvocationObservation],
    callback_receipts: &[CallbackReceipt],
    effect_completions: &[EffectCompletion],
) -> crate::error::Result<CanonicalIndex> {
    let state = canonical_instance_record(instance)?;
    let estimated = estimated_member_count(instance, observations, callback_receipts, effect_completions)?;
    let mut members = Vec::with_capacity(estimated);
    members.push(member(Role::Executable, &instance.executable_ref, &instance.manifest_ref));
    members.push(member(Role::InstanceState, &state.record_ref, &instance.manifest_ref));
    if let Some(state_ref) = &instance.state_ref {
        members.push(member(Role::SemanticState, state_ref, &state.record_ref));
    }
    if let Some(checkpoint_ref) = &instance.checkpoint_ref {
        members.push(member(Role::Checkpoint, checkpoint_ref, &state.record_ref));
    }
    for observation in observations {
        members.push(member(Role::CallbackEnvelope, &observation.envelope_ref, &instance.manifest_ref));
        if let Some(receipt_ref) = &observation.execution_receipt_ref {
            members.push(member(Role::ExecutionReceipt, receipt_ref, &observation.envelope_ref));
        }
    }
    for receipt in callback_receipts {
        members.push(member(Role::CallbackReceipt, &receipt.receipt_ref, &instance.manifest_ref));
    }
    for completion in effect_completions {
        members.push(member(Role::Effect, &completion.completion_ref, &completion.callback_receipt_ref));
    }
    for operation in &instance.unresolved {
        let role = match operation.kind {
            NativeOperationKind::Effect => Some(Role::Effect),
            NativeOperationKind::ValuePublication => Some(Role::ValuePublication),
            NativeOperationKind::Callback | NativeOperationKind::Ingress => None,
        };
        if let Some(role) = role {
            members.push(Member {
                role,
                artifact_ref: operation.operation_ref.clone(),
                parent_ref: state.record_ref.clone(),
            });
        }
    }
    for operation in &instance.completed_operations {
        if operation.kind == NativeOperationKind::ValuePublication
            && let Some(terminal_ref) = &operation.terminal_ref
        {
            members.push(member(Role::ValuePublication, terminal_ref, &state.record_ref));
        }
    }
    let mut represented_refs =
        members.iter().map(|member| member.artifact_ref.clone()).collect::<std::collections::BTreeSet<_>>();
    for evidence_ref in &instance.evidence_refs {
        if represented_refs.insert(evidence_ref.clone()) {
            members.push(member(Role::LifecycleEvidence, evidence_ref, &state.record_ref));
        }
    }
    members.sort_by(|left, right| (left.role, &left.artifact_ref).cmp(&(right.role, &right.artifact_ref)));
    let value = index_value(&instance.manifest_ref, &members, &REQUIRED_NATIVE_HOST_NON_CLAIMS);
    let index_ref = crate::preserves_rail::canonical_hash(&value)?;
    let index = CanonicalIndex {
        index_ref,
        manifest_ref: instance.manifest_ref.clone(),
        members,
        non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        value,
    };
    verify_index(&index).map_err(|issues| {
        crate::error::Failure::invalid_harness(format!("native artifact index denied: {issues:?}"))
    })?;
    Ok(index)
}

fn member(role: Role, artifact_ref: &str, parent_ref: &str) -> Member {
    Member {
        role,
        artifact_ref: artifact_ref.to_string(),
        parent_ref: parent_ref.to_string(),
    }
}

/// An upper bound on index members: two per invocation, one per receipt, completion, operation, and
/// evidence ref, and four for the executable, instance state, semantic state, and checkpoint.
fn estimated_member_count(
    instance: &NativeInstanceRecord,
    observations: &[InvocationObservation],
    callback_receipts: &[CallbackReceipt],
    effect_completions: &[EffectCompletion],
) -> crate::error::Result<usize> {
    observations
        .len()
        .checked_mul(2)
        .and_then(|value| value.checked_add(callback_receipts.len()))
        .and_then(|value| value.checked_add(effect_completions.len()))
        .and_then(|value| value.checked_add(instance.unresolved.len()))
        .and_then(|value| value.checked_add(instance.completed_operations.len()))
        .and_then(|value| value.checked_add(instance.evidence_refs.len()))
        .and_then(|value| value.checked_add(4))
        .ok_or_else(|| crate::error::Failure::invalid_harness("native artifact index member count overflow"))
}

// r[impl molten.system_extension.native_host.validation]
pub fn verify_index(index: &CanonicalIndex) -> std::result::Result<(), Vec<IndexIssue>> {
    let mut issues = Vec::new();
    if index.members.len() > MAX_ARTIFACT_MEMBERS {
        issues.push(IndexIssue::TooManyMembers {
            actual: index.members.len(),
            maximum: MAX_ARTIFACT_MEMBERS,
        });
    }
    let mut members = std::collections::BTreeSet::new();
    let mut refs = std::collections::BTreeSet::new();
    issues.reserve(index.members.len());
    for member in &index.members {
        if !members.insert((member.role, member.artifact_ref.clone())) {
            issues.push(IndexIssue::DuplicateMember(member.artifact_ref.clone()));
        }
        refs.insert(member.artifact_ref.clone());
        validate_ref("artifact-ref", &member.artifact_ref, &mut issues);
        validate_ref("parent-ref", &member.parent_ref, &mut issues);
    }
    validate_ref("manifest-ref", &index.manifest_ref, &mut issues);
    issues.extend(
        [Role::Executable, Role::InstanceState]
            .into_iter()
            .filter(|role| !index.members.iter().any(|member| member.role == *role))
            .map(IndexIssue::MissingRole),
    );
    issues.extend(
        index
            .members
            .iter()
            .filter(|member| member.parent_ref != index.manifest_ref && !refs.contains(&member.parent_ref))
            .map(|member| IndexIssue::ParentNotIndexed {
                artifact_ref: member.artifact_ref.clone(),
                parent_ref: member.parent_ref.clone(),
            }),
    );
    issues.extend(
        REQUIRED_NATIVE_HOST_NON_CLAIMS
            .into_iter()
            .filter(|required| !index.non_claims.contains(required))
            .map(IndexIssue::MissingNonClaim),
    );
    let expected_value = index_value(&index.manifest_ref, &index.members, &index.non_claims);
    if crate::preserves_rail::canonical_hash(&expected_value).ok().as_deref() != Some(&index.index_ref)
        || expected_value != index.value
    {
        issues.push(IndexIssue::IndexIdentityMismatch);
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn index_value(manifest_ref: &str, members: &[Member], non_claims: &[NativeHostNonClaim]) -> preserves::IOValue {
    crate::preserves_rail::record(ARTIFACT_INDEX_RECORD, vec![
        crate::preserves_rail::string(ARTIFACT_INDEX_SCHEMA),
        crate::preserves_rail::string(manifest_ref),
        crate::preserves_rail::sequence(
            members
                .iter()
                .map(|member| {
                    crate::preserves_rail::record(ARTIFACT_MEMBER_RECORD, vec![
                        crate::preserves_rail::string(member.role.as_str()),
                        crate::preserves_rail::string(&member.artifact_ref),
                        crate::preserves_rail::string(&member.parent_ref),
                    ])
                })
                .collect(),
        ),
        crate::preserves_rail::sequence(
            non_claims.iter().map(|claim| crate::preserves_rail::string(claim.as_str())).collect(),
        ),
    ])
}

fn validate_ref(field: &'static str, value: &str, issues: &mut impl crate::bounded::VecSink<IndexIssue>) {
    if crate::preserves_rail::ContentRef::parse(value).is_err() {
        issues.push_item(IndexIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}
