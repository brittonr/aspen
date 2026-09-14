pub(super) fn binding_value(
    input: &super::ConsistencyGroupBindingInput,
    lifecycle: super::ConsistencyGroupLifecycle,
) -> preserves::IOValue {
    record("fabric-consistency-group-binding-v1", vec![
        field("schema", string(super::CONSISTENCY_GROUP_BINDING_SCHEMA)),
        field("group-id", string(&input.group_id)),
        field("extension-id", string(&input.extension_id)),
        field("service-id", string(&input.service_id)),
        field("service-generation", u64_value(input.service_generation)),
        field("application-manifest-ref", string(&input.application_manifest_ref)),
        field("algorithm-profile", string(&input.engine_algorithm_profile)),
        field("implementation-profile", string(&input.engine_implementation_profile)),
        field("membership-ref", string(&input.membership_ref)),
        field("config-epoch", u64_value(input.config_epoch)),
        field("placement-ref", string(&input.placement_ref)),
        field("fencing-ref", string(&input.fencing_ref)),
        field("fencing-epoch", u64_value(input.fencing_epoch)),
        field("resource-profile-ref", string(&input.resource_profile_ref)),
        field("policy-refs", strings(&input.policy_refs)),
        field("non-claims", strings(&input.non_claims)),
        field(
            "supported-read-modes",
            sequence(input.supported_read_modes.iter().map(|mode| string(mode.as_str())).collect()),
        ),
        field("max-command-bytes", u64_value(input.max_command_bytes)),
        field("max-in-flight-operations", u64_value(u64::from(input.max_in_flight_operations))),
        field("lifecycle", string(lifecycle.as_str())),
    ])
}

pub(super) struct PlanValueInput<'a> {
    pub binding: &'a super::ConsistencyGroupBinding,
    pub command: &'a super::ConsistencyPortCommandInput,
    pub decision: super::ConsistencyPlanDecision,
    pub lifecycle_before: super::ConsistencyGroupLifecycle,
    pub lifecycle_after: super::ConsistencyGroupLifecycle,
    pub diagnostics: &'a [String],
}

pub(super) fn plan_value(input: PlanValueInput<'_>) -> preserves::IOValue {
    let binding = input.binding;
    let command = input.command;
    record("fabric-consistency-operation-plan-v1", vec![
        field("schema", string(super::CONSISTENCY_PORT_PLAN_SCHEMA)),
        field("request-ref", string(&command.request_ref)),
        field("binding-ref", string(&command.binding_ref)),
        field("group-id", string(&command.group_id)),
        field("extension-id", string(&command.extension_id)),
        field("service-id", string(&command.service_id)),
        field("service-generation", u64_value(command.service_generation)),
        field("application-manifest-ref", string(&command.application_manifest_ref)),
        field("algorithm-profile", string(&command.engine_algorithm_profile)),
        field("implementation-profile", string(&command.engine_implementation_profile)),
        field("membership-ref", string(&command.membership_ref)),
        field("config-epoch", u64_value(command.config_epoch)),
        field("placement-ref", string(&command.placement_ref)),
        field("fencing-ref", string(&command.fencing_ref)),
        field("fencing-epoch", u64_value(command.fencing_epoch)),
        field("resource-profile-ref", string(&command.resource_profile_ref)),
        field("policy-refs", strings(&command.policy_refs)),
        field("authority-refs", strings(&command.authority_refs)),
        field("observed-in-flight-operations", u64_value(u64::from(command.observed_in_flight_operations))),
        field("operation", operation_value(&command.operation)),
        field("decision", string(input.decision.as_str())),
        field("lifecycle-before", string(input.lifecycle_before.as_str())),
        field("lifecycle-after", string(input.lifecycle_after.as_str())),
        field("diagnostics", strings(input.diagnostics)),
        field("group-non-claims", strings(&binding.non_claims)),
    ])
}

pub(super) fn outcome_value(
    plan_ref: &str,
    binding: &super::ConsistencyGroupBinding,
    input: &super::ConsistencyOutcomeInput,
    outcome_kind: super::ConsistencyOutcomeKind,
) -> preserves::IOValue {
    record("fabric-consistency-operation-outcome-v1", vec![
        field("schema", string(super::CONSISTENCY_PORT_OUTCOME_SCHEMA)),
        field("plan-ref", string(plan_ref)),
        field("request-ref", string(&input.request_ref)),
        field("binding-ref", string(&input.binding_ref)),
        field("group-id", string(&binding.group_id)),
        field("service-generation", u64_value(input.service_generation)),
        field("config-epoch", u64_value(input.config_epoch)),
        field("fencing-epoch", u64_value(input.fencing_epoch)),
        field("outcome", string(outcome_kind.as_str())),
        field("result-ref", optional_string(input.result_ref.as_deref())),
        field("evidence-refs", strings(&input.evidence_refs)),
        field("diagnostics", strings(&input.diagnostics)),
        field("non-claims", strings(&binding.non_claims)),
    ])
}

fn operation_value(operation: &super::ConsistencyOperation) -> preserves::IOValue {
    match operation {
        super::ConsistencyOperation::Open { mode } => record("open", vec![field("mode", string(mode.as_str()))]),
        super::ConsistencyOperation::Propose {
            command_ref,
            command_schema_ref,
            estimated_command_bytes,
        } => record("propose", vec![
            field("command-ref", string(command_ref)),
            field("command-schema-ref", string(command_schema_ref)),
            field("estimated-command-bytes", u64_value(*estimated_command_bytes)),
        ]),
        super::ConsistencyOperation::Read { query_ref, mode } => record("read", vec![
            field("query-ref", string(query_ref)),
            field("mode", string(mode.as_str())),
        ]),
        super::ConsistencyOperation::Snapshot { snapshot_policy_ref } => {
            record("snapshot", vec![field("snapshot-policy-ref", string(snapshot_policy_ref))])
        }
        super::ConsistencyOperation::Recover {
            snapshot_ref,
            durable_boundary_ref,
        } => record("recover", vec![
            field("snapshot-ref", string(snapshot_ref)),
            field("durable-boundary-ref", string(durable_boundary_ref)),
        ]),
        super::ConsistencyOperation::Configure { transition } => configuration_value(transition),
        super::ConsistencyOperation::Health => record("health", Vec::new()),
        super::ConsistencyOperation::Drain => record("drain", Vec::new()),
        super::ConsistencyOperation::Status => record("status", Vec::new()),
        super::ConsistencyOperation::Remove => record("remove", Vec::new()),
    }
}

fn configuration_value(transition: &super::ConfigurationTransition) -> preserves::IOValue {
    record("configure", vec![
        field("transition", string(transition.as_str())),
        field("next-membership-ref", string(transition.next_membership_ref())),
        field("next-config-epoch", u64_value(transition.next_config_epoch())),
    ])
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    record(label, vec![value])
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(|| record("none", Vec::new()), |present| record("some", vec![string(present)]))
}

fn strings(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(string).collect())
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
