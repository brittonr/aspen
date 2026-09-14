pub(super) fn plan_consistency_operation(
    binding: &super::ConsistencyGroupBinding,
    input: super::ConsistencyPortCommandInput,
) -> crate::error::Result<super::ConsistencyPortPlan> {
    validate_command_input(&input)?;
    let mut diagnostics = std::collections::BTreeSet::new();
    collect_binding_diagnostics(binding, &input, &mut diagnostics);
    collect_operation_diagnostics(binding, &input, &mut diagnostics);
    if input.observed_in_flight_operations >= binding.max_in_flight_operations {
        diagnostics.insert("in-flight-operation-bound-exhausted");
    }
    if diagnostics.len() > super::MAX_CONSISTENCY_DIAGNOSTICS {
        return Err(crate::error::MoltenError::invalid_harness("consistency diagnostics exceeded the bounded maximum"));
    }
    let decision = if diagnostics.is_empty() {
        super::ConsistencyPlanDecision::Admitted
    } else {
        super::ConsistencyPlanDecision::Denied
    };
    let lifecycle_after = planned_lifecycle(binding.lifecycle, &input.operation, decision);
    let diagnostics = diagnostics.into_iter().map(str::to_string).collect::<Vec<_>>();
    let value = super::canonical::plan_value(super::canonical::PlanValueInput {
        binding,
        command: &input,
        decision,
        lifecycle_before: binding.lifecycle,
        lifecycle_after,
        diagnostics: &diagnostics,
    });
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(super::ConsistencyPortPlan {
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

fn validate_command_input(input: &super::ConsistencyPortCommandInput) -> crate::error::Result<()> {
    super::binding::validate_content_ref(&input.request_ref, "consistency request ref")?;
    super::binding::validate_content_ref(&input.binding_ref, "consistency binding ref")?;
    for (value, label) in [
        (&input.group_id, "consistency command group id"),
        (&input.extension_id, "consistency command extension id"),
        (&input.service_id, "consistency command service id"),
        (&input.engine_algorithm_profile, "consistency command algorithm profile"),
        (&input.engine_implementation_profile, "consistency command implementation profile"),
    ] {
        super::binding::validate_identifier(value, label)?;
    }
    for (reference, label) in [
        (&input.application_manifest_ref, "application manifest ref"),
        (&input.membership_ref, "membership ref"),
        (&input.placement_ref, "placement ref"),
        (&input.fencing_ref, "fencing ref"),
        (&input.resource_profile_ref, "resource profile ref"),
    ] {
        super::binding::validate_content_ref(reference, label)?;
    }
    super::binding::validate_content_refs(
        &input.policy_refs,
        super::MAX_CONSISTENCY_POLICY_REFS,
        "consistency command policy refs",
        true,
    )?;
    super::binding::validate_content_refs(
        &input.authority_refs,
        super::MAX_CONSISTENCY_AUTHORITY_REFS,
        "consistency command authority refs",
        true,
    )?;
    validate_operation(&input.operation)
}

fn validate_operation(operation: &super::ConsistencyOperation) -> crate::error::Result<()> {
    match operation {
        super::ConsistencyOperation::Open { .. }
        | super::ConsistencyOperation::Health
        | super::ConsistencyOperation::Drain
        | super::ConsistencyOperation::Status
        | super::ConsistencyOperation::Remove => Ok(()),
        super::ConsistencyOperation::Propose {
            command_ref,
            command_schema_ref,
            ..
        } => {
            super::binding::validate_content_ref(command_ref, "consistency command ref")?;
            super::binding::validate_content_ref(command_schema_ref, "consistency command schema ref")
        }
        super::ConsistencyOperation::Read { query_ref, .. } => {
            super::binding::validate_content_ref(query_ref, "consistency query ref")
        }
        super::ConsistencyOperation::Snapshot { snapshot_policy_ref } => {
            super::binding::validate_content_ref(snapshot_policy_ref, "snapshot policy ref")
        }
        super::ConsistencyOperation::Recover {
            snapshot_ref,
            durable_boundary_ref,
        } => {
            super::binding::validate_content_ref(snapshot_ref, "snapshot ref")?;
            super::binding::validate_content_ref(durable_boundary_ref, "durable boundary ref")
        }
        super::ConsistencyOperation::Configure { transition } => {
            super::binding::validate_content_ref(transition.next_membership_ref(), "next membership ref")
        }
    }
}

fn collect_binding_diagnostics(
    binding: &super::ConsistencyGroupBinding,
    input: &super::ConsistencyPortCommandInput,
    diagnostics: &mut std::collections::BTreeSet<&str>,
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
    binding: &super::ConsistencyGroupBinding,
    input: &super::ConsistencyPortCommandInput,
    diagnostics: &mut std::collections::BTreeSet<&str>,
) {
    if !operation_allowed_for_lifecycle(binding.lifecycle, &input.operation) {
        diagnostics.insert("operation-denied-for-lifecycle");
    }
    match &input.operation {
        super::ConsistencyOperation::Propose {
            estimated_command_bytes,
            ..
        } if *estimated_command_bytes == 0 || *estimated_command_bytes > binding.max_command_bytes => {
            diagnostics.insert("command-byte-bound-denied");
        }
        super::ConsistencyOperation::Read { mode, .. }
            if *mode == super::ConsistencyReadMode::Lease || !binding.supported_read_modes.contains(mode) =>
        {
            diagnostics.insert("unsupported-read-mode");
        }
        super::ConsistencyOperation::Configure { transition } => {
            collect_configuration_diagnostics(binding, transition, diagnostics);
        }
        _ => {}
    }
}

fn collect_configuration_diagnostics(
    binding: &super::ConsistencyGroupBinding,
    transition: &super::ConfigurationTransition,
    diagnostics: &mut std::collections::BTreeSet<&str>,
) {
    if !matches!(transition, super::ConfigurationTransition::StaticMembershipRefresh { .. }) {
        diagnostics.insert("unsupported-configuration-transition");
    }
    let Some(expected_epoch) = binding.config_epoch.checked_add(super::NEXT_CONSISTENCY_EPOCH_STEP) else {
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

fn operation_allowed_for_lifecycle(
    lifecycle: super::ConsistencyGroupLifecycle,
    operation: &super::ConsistencyOperation,
) -> bool {
    match lifecycle {
        super::ConsistencyGroupLifecycle::Declared => {
            matches!(operation, super::ConsistencyOperation::Open { .. } | super::ConsistencyOperation::Status)
        }
        super::ConsistencyGroupLifecycle::Active => {
            !matches!(operation, super::ConsistencyOperation::Open { .. } | super::ConsistencyOperation::Remove)
        }
        super::ConsistencyGroupLifecycle::Draining => matches!(
            operation,
            super::ConsistencyOperation::Snapshot { .. }
                | super::ConsistencyOperation::Health
                | super::ConsistencyOperation::Drain
                | super::ConsistencyOperation::Status
                | super::ConsistencyOperation::Remove
        ),
        super::ConsistencyGroupLifecycle::Removed => matches!(operation, super::ConsistencyOperation::Status),
    }
}

fn planned_lifecycle(
    current: super::ConsistencyGroupLifecycle,
    operation: &super::ConsistencyOperation,
    decision: super::ConsistencyPlanDecision,
) -> super::ConsistencyGroupLifecycle {
    if decision == super::ConsistencyPlanDecision::Denied {
        return current;
    }
    match operation {
        super::ConsistencyOperation::Open { .. } => super::ConsistencyGroupLifecycle::Active,
        super::ConsistencyOperation::Drain => super::ConsistencyGroupLifecycle::Draining,
        super::ConsistencyOperation::Remove => super::ConsistencyGroupLifecycle::Removed,
        _ => current,
    }
}
