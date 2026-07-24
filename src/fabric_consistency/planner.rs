use std::collections::BTreeSet;

use super::ConfigurationTransition;
use super::ConsistencyGroupBinding;
use super::ConsistencyGroupLifecycle;
use super::ConsistencyOperation;
use super::ConsistencyPlanDecision;
use super::ConsistencyPortCommandInput;
use super::ConsistencyPortPlan;
use super::ConsistencyReadMode;
use super::MAX_CONSISTENCY_AUTHORITY_REFS;
use super::MAX_CONSISTENCY_DIAGNOSTICS;
use super::MAX_CONSISTENCY_POLICY_REFS;
use super::NEXT_CONSISTENCY_EPOCH_STEP;
use super::binding::validate_content_ref;
use super::binding::validate_content_refs;
use super::binding::validate_identifier;
use super::canonical::PlanValueInput;
use super::canonical::plan_value;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn plan_consistency_operation(
    binding: &ConsistencyGroupBinding,
    input: ConsistencyPortCommandInput,
) -> Result<ConsistencyPortPlan> {
    validate_command_input(&input)?;
    let mut diagnostics = BTreeSet::new();
    collect_binding_diagnostics(binding, &input, &mut diagnostics);
    collect_operation_diagnostics(binding, &input, &mut diagnostics);
    if input.observed_in_flight_operations >= binding.max_in_flight_operations {
        diagnostics.insert("in-flight-operation-bound-exhausted");
    }
    if diagnostics.len() > MAX_CONSISTENCY_DIAGNOSTICS {
        return Err(MoltenError::invalid_harness("consistency diagnostics exceeded the bounded maximum"));
    }
    let decision = if diagnostics.is_empty() {
        ConsistencyPlanDecision::Admitted
    } else {
        ConsistencyPlanDecision::Denied
    };
    let lifecycle_after = planned_lifecycle(binding.lifecycle, &input.operation, decision);
    let diagnostics = diagnostics.into_iter().map(str::to_string).collect::<Vec<_>>();
    let value = plan_value(PlanValueInput {
        binding,
        command: &input,
        decision,
        lifecycle_before: binding.lifecycle,
        lifecycle_after,
        diagnostics: &diagnostics,
    });
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ConsistencyPortPlan {
        plan_ref,
        request_ref: input.request_ref,
        binding_ref: input.binding_ref,
        operation: input.operation,
        decision,
        lifecycle_before: binding.lifecycle,
        lifecycle_after,
        diagnostics,
        value,
    })
}

fn validate_command_input(input: &ConsistencyPortCommandInput) -> Result<()> {
    validate_content_ref(&input.request_ref, "consistency request ref")?;
    validate_content_ref(&input.binding_ref, "consistency binding ref")?;
    for (value, label) in [
        (&input.group_id, "consistency command group id"),
        (&input.extension_id, "consistency command extension id"),
        (&input.service_id, "consistency command service id"),
        (&input.engine_algorithm_profile, "consistency command algorithm profile"),
        (&input.engine_implementation_profile, "consistency command implementation profile"),
    ] {
        validate_identifier(value, label)?;
    }
    for (reference, label) in [
        (&input.application_manifest_ref, "application manifest ref"),
        (&input.membership_ref, "membership ref"),
        (&input.placement_ref, "placement ref"),
        (&input.fencing_ref, "fencing ref"),
        (&input.resource_profile_ref, "resource profile ref"),
    ] {
        validate_content_ref(reference, label)?;
    }
    validate_content_refs(&input.policy_refs, MAX_CONSISTENCY_POLICY_REFS, "consistency command policy refs", true)?;
    validate_content_refs(
        &input.authority_refs,
        MAX_CONSISTENCY_AUTHORITY_REFS,
        "consistency command authority refs",
        true,
    )?;
    validate_operation(&input.operation)
}

fn validate_operation(operation: &ConsistencyOperation) -> Result<()> {
    match operation {
        ConsistencyOperation::Open { .. }
        | ConsistencyOperation::Health
        | ConsistencyOperation::Drain
        | ConsistencyOperation::Status
        | ConsistencyOperation::Remove => Ok(()),
        ConsistencyOperation::Propose {
            command_ref,
            command_schema_ref,
            ..
        } => {
            validate_content_ref(command_ref, "consistency command ref")?;
            validate_content_ref(command_schema_ref, "consistency command schema ref")
        }
        ConsistencyOperation::Read { query_ref, .. } => validate_content_ref(query_ref, "consistency query ref"),
        ConsistencyOperation::Snapshot { snapshot_policy_ref } => {
            validate_content_ref(snapshot_policy_ref, "snapshot policy ref")
        }
        ConsistencyOperation::Recover {
            snapshot_ref,
            durable_boundary_ref,
        } => {
            validate_content_ref(snapshot_ref, "snapshot ref")?;
            validate_content_ref(durable_boundary_ref, "durable boundary ref")
        }
        ConsistencyOperation::Configure { transition } => {
            validate_content_ref(transition.next_membership_ref(), "next membership ref")
        }
    }
}

fn collect_binding_diagnostics(
    binding: &ConsistencyGroupBinding,
    input: &ConsistencyPortCommandInput,
    diagnostics: &mut BTreeSet<&str>,
) {
    for (matches, diagnostic) in [
        (binding.binding_ref == input.binding_ref, "binding-ref-mismatch"),
        (binding.group_id == input.group_id, "group-mismatch"),
        (binding.extension_id == input.extension_id, "extension-owner-mismatch"),
        (binding.service_id == input.service_id, "service-owner-mismatch"),
        (binding.service_generation == input.service_generation, "service-generation-mismatch"),
        (binding.application_manifest_ref == input.application_manifest_ref, "application-manifest-mismatch"),
        (binding.engine_algorithm_profile == input.engine_algorithm_profile, "algorithm-profile-mismatch"),
        (
            binding.engine_implementation_profile == input.engine_implementation_profile,
            "implementation-profile-mismatch",
        ),
        (binding.membership_ref == input.membership_ref, "membership-mismatch"),
        (binding.config_epoch == input.config_epoch, "config-epoch-mismatch"),
        (binding.placement_ref == input.placement_ref, "placement-mismatch"),
        (binding.fencing_ref == input.fencing_ref, "fencing-ref-mismatch"),
        (binding.fencing_epoch == input.fencing_epoch, "fencing-epoch-mismatch"),
        (binding.resource_profile_ref == input.resource_profile_ref, "resource-profile-mismatch"),
        (binding.policy_refs == input.policy_refs, "policy-refs-mismatch"),
    ] {
        if !matches {
            diagnostics.insert(diagnostic);
        }
    }
}

fn collect_operation_diagnostics(
    binding: &ConsistencyGroupBinding,
    input: &ConsistencyPortCommandInput,
    diagnostics: &mut BTreeSet<&str>,
) {
    if !operation_allowed_for_lifecycle(binding.lifecycle, &input.operation) {
        diagnostics.insert("operation-denied-for-lifecycle");
    }
    match &input.operation {
        ConsistencyOperation::Propose {
            estimated_command_bytes,
            ..
        } if *estimated_command_bytes == 0 || *estimated_command_bytes > binding.max_command_bytes => {
            diagnostics.insert("command-byte-bound-denied");
        }
        ConsistencyOperation::Read { mode, .. }
            if *mode == ConsistencyReadMode::Lease || !binding.supported_read_modes.contains(mode) =>
        {
            diagnostics.insert("unsupported-read-mode");
        }
        ConsistencyOperation::Configure { transition } => {
            collect_configuration_diagnostics(binding, transition, diagnostics);
        }
        _ => {}
    }
}

fn collect_configuration_diagnostics(
    binding: &ConsistencyGroupBinding,
    transition: &ConfigurationTransition,
    diagnostics: &mut BTreeSet<&str>,
) {
    if !matches!(transition, ConfigurationTransition::StaticMembershipRefresh { .. }) {
        diagnostics.insert("unsupported-configuration-transition");
    }
    let Some(expected_epoch) = binding.config_epoch.checked_add(NEXT_CONSISTENCY_EPOCH_STEP) else {
        diagnostics.insert("config-epoch-overflow");
        return;
    };
    if transition.next_config_epoch() != expected_epoch {
        diagnostics.insert("next-config-epoch-mismatch");
    }
    if transition.next_membership_ref() == binding.membership_ref {
        diagnostics.insert("membership-refresh-is-noop");
    }
}

fn operation_allowed_for_lifecycle(lifecycle: ConsistencyGroupLifecycle, operation: &ConsistencyOperation) -> bool {
    match lifecycle {
        ConsistencyGroupLifecycle::Declared => {
            matches!(operation, ConsistencyOperation::Open { .. } | ConsistencyOperation::Status)
        }
        ConsistencyGroupLifecycle::Active => {
            !matches!(operation, ConsistencyOperation::Open { .. } | ConsistencyOperation::Remove)
        }
        ConsistencyGroupLifecycle::Draining => matches!(
            operation,
            ConsistencyOperation::Snapshot { .. }
                | ConsistencyOperation::Health
                | ConsistencyOperation::Drain
                | ConsistencyOperation::Status
                | ConsistencyOperation::Remove
        ),
        ConsistencyGroupLifecycle::Removed => matches!(operation, ConsistencyOperation::Status),
    }
}

fn planned_lifecycle(
    current: ConsistencyGroupLifecycle,
    operation: &ConsistencyOperation,
    decision: ConsistencyPlanDecision,
) -> ConsistencyGroupLifecycle {
    if decision == ConsistencyPlanDecision::Denied {
        return current;
    }
    match operation {
        ConsistencyOperation::Open { .. } => ConsistencyGroupLifecycle::Active,
        ConsistencyOperation::Drain => ConsistencyGroupLifecycle::Draining,
        ConsistencyOperation::Remove => ConsistencyGroupLifecycle::Removed,
        _ => current,
    }
}
