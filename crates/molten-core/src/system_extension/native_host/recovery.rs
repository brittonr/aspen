use super::super::LifecyclePhase;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRecoveryClass {
    NotStarted,
    RunningObserved,
    Terminal,
    Unknown,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRecoveryInventory {
    pub operation_ref: String,
    pub kind: NativeOperationKind,
    pub class: NativeRecoveryClass,
    pub is_retry_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRemovalPlan {
    pub instance_id: String,
    pub generation: u64,
}

// r[impl molten.system_extension.native_host.recovery]
pub fn admit_native_instance_recovery(
    profile: &AdmittedNativeHostProfile,
    executable: &AdmittedNativeExecutable,
    instance: &NativeInstanceRecord,
) -> Result<(), Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(8);
    if instance.schema != NATIVE_INSTANCE_STATE_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "instance-schema",
            actual: instance.schema.clone(),
            expected: NATIVE_INSTANCE_STATE_SCHEMA,
        });
    }
    for (field, is_matching) in [
        ("profile-ref", instance.profile_ref == profile.profile.profile_ref),
        ("executable-ref", instance.executable_ref == executable.executable.executable_ref),
        ("manifest-ref", instance.manifest_ref == executable.executable.manifest_ref),
        ("state-schema-ref", instance.state_schema_ref == executable.executable.state_schema_ref),
    ] {
        if !is_matching {
            issues.push(NativeHostIssue::IdentityMismatch(field));
        }
    }
    if instance.lifecycle.generation == 0 {
        issues.push(NativeHostIssue::StaleGeneration { actual: 0, active: 1 });
    }
    if matches!(
        instance.lifecycle.phase,
        LifecyclePhase::Running | LifecyclePhase::Failed | LifecyclePhase::Restarting
    ) && instance.checkpoint_ref.is_none()
    {
        issues.push(NativeHostIssue::IdentityMismatch("checkpoint-ref"));
    }
    if instance.unresolved.len() > profile.profile.max_unresolved_operations {
        issues.push(NativeHostIssue::TooManyUnresolvedOperations {
            actual: instance.unresolved.len(),
            maximum: profile.profile.max_unresolved_operations,
        });
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

// r[impl molten.system_extension.native_host.intent]
pub fn commit_native_operation_intent(
    profile: &AdmittedNativeHostProfile,
    instance: &NativeInstanceRecord,
    operation: NativeOperationRecord,
) -> Result<NativeInstanceRecord, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(4);
    if operation.schema != NATIVE_OPERATION_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "operation-schema",
            actual: operation.schema.clone(),
            expected: NATIVE_OPERATION_SCHEMA,
        });
    }
    if operation.generation != instance.lifecycle.generation {
        issues.push(NativeHostIssue::StaleGeneration {
            actual: operation.generation,
            active: instance.lifecycle.generation,
        });
    }
    if operation.state != NativeOperationState::IntentCommitted || operation.is_retry_permitted {
        issues.push(NativeHostIssue::InvalidOperationTransition {
            previous: operation.state,
            next: NativeOperationState::IntentCommitted,
        });
    }
    if instance.unresolved.iter().any(|pending| pending.operation_ref == operation.operation_ref)
        || instance.completed_operation_refs.contains(&operation.operation_ref)
    {
        issues.push(NativeHostIssue::DuplicateOperation(operation.operation_ref.clone()));
    }
    let Some(next_count) = instance.unresolved.len().checked_add(1) else {
        issues.push(NativeHostIssue::OperationCountOverflow);
        return Err(issues);
    };
    if next_count > profile.profile.max_unresolved_operations {
        issues.push(NativeHostIssue::TooManyUnresolvedOperations {
            actual: next_count,
            maximum: profile.profile.max_unresolved_operations,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut next = instance.clone();
    next.unresolved.push(operation);
    next.unresolved.sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
    Ok(next)
}

// r[impl molten.system_extension.native_host.recovery]
pub fn observe_native_operation(
    instance: &NativeInstanceRecord,
    operation_ref: &str,
    next_state: NativeOperationState,
    terminal_ref: Option<String>,
) -> Result<NativeInstanceRecord, Vec<NativeHostIssue>> {
    let Some(index) = instance.unresolved.iter().position(|operation| operation.operation_ref == operation_ref) else {
        return Err(vec![NativeHostIssue::OperationNotFound(operation_ref.to_string())]);
    };
    let operation = &instance.unresolved[index];
    if !valid_operation_transition(operation.state, next_state, terminal_ref.as_deref()) {
        return Err(vec![NativeHostIssue::InvalidOperationTransition {
            previous: operation.state,
            next: next_state,
        }]);
    }
    let mut next = instance.clone();
    if next_state == NativeOperationState::Terminal {
        let mut completed = next.unresolved.remove(index);
        completed.state = NativeOperationState::Terminal;
        completed.terminal_ref = terminal_ref;
        next.completed_operations.push(completed);
        next.completed_operations.sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
    } else {
        next.unresolved[index].state = next_state;
        next.unresolved[index].terminal_ref = terminal_ref;
    }
    Ok(next)
}

// r[impl molten.system_extension.native_host.effect_completion]
pub fn admit_native_effect_completion(
    instance: &NativeInstanceRecord,
    input: &NativeEffectCompletionInput,
) -> Result<NativeCompletionCallbackPlan, Vec<NativeHostIssue>> {
    if instance.completed_operation_refs.contains(&input.operation_ref) {
        return Err(vec![NativeHostIssue::CompletionAlreadyConsumed(input.operation_ref.clone())]);
    }
    let Some(operation) = instance
        .unresolved
        .iter()
        .chain(instance.completed_operations.iter())
        .find(|operation| operation.operation_ref == input.operation_ref)
    else {
        return Err(vec![NativeHostIssue::OperationNotFound(input.operation_ref.clone())]);
    };
    let mut issues = Vec::with_capacity(4);
    if operation.kind != NativeOperationKind::Effect {
        issues.push(NativeHostIssue::OperationKindMismatch);
    }
    if operation.parent_ref != input.effect_ref {
        issues.push(NativeHostIssue::IdentityMismatch("effect-ref"));
    }
    if input.generation != instance.lifecycle.generation || operation.generation != input.generation {
        issues.push(NativeHostIssue::StaleGeneration {
            actual: input.generation,
            active: instance.lifecycle.generation,
        });
    }
    for (field, reference) in [
        ("completion-ref", input.completion_ref.as_str()),
        ("port-binding-ref", input.port_binding_ref.as_str()),
    ] {
        if !super::super::valid_ref(reference) {
            issues.push(NativeHostIssue::MalformedRef {
                field,
                value: reference.to_string(),
            });
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(NativeCompletionCallbackPlan {
        completion_ref: input.completion_ref.clone(),
        payload_ref: input.completion_ref.clone(),
        generation: input.generation,
    })
}

pub fn consume_native_effect_completion(
    instance: &NativeInstanceRecord,
    input: &NativeEffectCompletionInput,
) -> Result<NativeInstanceRecord, Vec<NativeHostIssue>> {
    admit_native_effect_completion(instance, input)?;
    let mut next = instance.clone();
    next.completed_operation_refs.push(input.operation_ref.clone());
    next.completed_operation_refs.sort();
    next.completed_operation_refs.dedup();
    Ok(next)
}

// r[impl molten.system_extension.native_host.recovery]
pub fn classify_native_recovery(instance: &NativeInstanceRecord) -> Vec<NativeRecoveryInventory> {
    let mut inventory = instance
        .unresolved
        .iter()
        .map(|operation| NativeRecoveryInventory {
            operation_ref: operation.operation_ref.clone(),
            kind: operation.kind,
            class: if operation.generation != instance.lifecycle.generation {
                NativeRecoveryClass::Stale
            } else {
                match operation.state {
                    NativeOperationState::IntentCommitted => NativeRecoveryClass::NotStarted,
                    NativeOperationState::Started => NativeRecoveryClass::RunningObserved,
                    NativeOperationState::Terminal => NativeRecoveryClass::Terminal,
                    NativeOperationState::Unknown => NativeRecoveryClass::Unknown,
                    NativeOperationState::Stale => NativeRecoveryClass::Stale,
                }
            },
            is_retry_permitted: false,
        })
        .collect::<Vec<_>>();
    inventory.sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
    inventory
}

// r[impl molten.system_extension.native_host.operator]
pub fn admit_native_removal(instance: &NativeInstanceRecord) -> Result<NativeRemovalPlan, Vec<NativeHostIssue>> {
    let mut blockers = instance.unresolved.iter().map(|operation| operation.operation_ref.clone()).collect::<Vec<_>>();
    if instance.is_accepting_ingress {
        blockers.push("active-ingress".to_string());
    }
    if !instance.usage.is_idle() {
        blockers.push("active-resource-usage".to_string());
    }
    if !matches!(instance.lifecycle.phase, LifecyclePhase::Stopped | LifecyclePhase::Removed) {
        blockers.push("missing-terminal-lifecycle".to_string());
    }
    blockers.sort();
    if !blockers.is_empty() {
        return Err(vec![NativeHostIssue::RemovalBlocked(blockers)]);
    }
    Ok(NativeRemovalPlan {
        instance_id: instance.instance_id.clone(),
        generation: instance.lifecycle.generation,
    })
}

fn valid_operation_transition(
    previous: NativeOperationState,
    next: NativeOperationState,
    terminal_ref: Option<&str>,
) -> bool {
    match (previous, next) {
        (NativeOperationState::IntentCommitted, NativeOperationState::Started) => terminal_ref.is_none(),
        (
            NativeOperationState::IntentCommitted | NativeOperationState::Started | NativeOperationState::Unknown,
            NativeOperationState::Terminal,
        ) => terminal_ref.is_some_and(super::super::valid_ref),
        (
            NativeOperationState::IntentCommitted | NativeOperationState::Started,
            NativeOperationState::Unknown | NativeOperationState::Stale,
        ) => terminal_ref.is_none(),
        _ => false,
    }
}
