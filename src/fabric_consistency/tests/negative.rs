use super::*;

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_binding_rejects_missing_policy_lease_and_over_bounds() {
    let mut missing_policy = binding_input();
    missing_policy.policy_refs.clear();
    assert!(canonical_consistency_group_binding(missing_policy).is_err());

    let mut lease = binding_input();
    lease.supported_read_modes.push(ConsistencyReadMode::Lease);
    assert!(canonical_consistency_group_binding(lease).is_err());

    let mut command_bound = binding_input();
    command_bound.max_command_bytes = MAX_CONSISTENCY_COMMAND_BYTES + NEXT_CONSISTENCY_EPOCH_STEP;
    assert!(canonical_consistency_group_binding(command_bound).is_err());

    let mut in_flight_bound = binding_input();
    in_flight_bound.max_in_flight_operations =
        MAX_CONSISTENCY_IN_FLIGHT_OPERATIONS + u32::try_from(NEXT_CONSISTENCY_EPOCH_STEP).expect("step");
    assert!(canonical_consistency_group_binding(in_flight_bound).is_err());
}

#[test]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_exact_binding_mismatches_deny_without_mutation() {
    let binding = active_binding();
    let mut command = command_for(&binding, ConsistencyOperation::Propose {
        command_ref: test_ref("command"),
        command_schema_ref: test_ref("schema"),
        estimated_command_bytes: COMMAND_BYTES,
    });
    command.extension_id = "extension-b".to_string();
    command.service_generation += NEXT_CONSISTENCY_EPOCH_STEP;
    command.application_manifest_ref = test_ref("wrong-application");
    command.membership_ref = test_ref("wrong-membership");
    command.config_epoch += NEXT_CONSISTENCY_EPOCH_STEP;
    command.placement_ref = test_ref("wrong-placement");
    command.fencing_epoch += NEXT_CONSISTENCY_EPOCH_STEP;
    command.resource_profile_ref = test_ref("wrong-resource");
    command.policy_refs = vec![test_ref("wrong-policy")];

    let plan = plan_consistency_operation(&binding, command).expect("denied plan");
    assert!(!plan.admitted());
    for expected in [
        "extension-owner-mismatch",
        "service-generation-mismatch",
        "application-manifest-mismatch",
        "membership-mismatch",
        "config-epoch-mismatch",
        "placement-mismatch",
        "fencing-epoch-mismatch",
        "resource-profile-mismatch",
        "policy-refs-mismatch",
    ] {
        assert!(plan.diagnostics.iter().any(|actual| actual == expected));
    }
    let denial = normalized_denial(&binding, &plan);
    assert_eq!(denial.kind, ConsistencyOutcomeKind::Denied);
    assert_eq!(apply_consistency_outcome(&binding, &plan, &denial).expect("denial is non-mutating"), binding);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn fabric_consistency_unsupported_modes_and_resource_exhaustion_deny() {
    let binding = active_binding();
    let lease = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Read {
            query_ref: test_ref("lease-query"),
            mode: ConsistencyReadMode::Lease,
        }),
    )
    .expect("lease denial");
    assert_eq!(lease.diagnostics, vec!["unsupported-read-mode"]);

    for transition in [
        ConfigurationTransition::DynamicMembership {
            next_membership_ref: test_ref("dynamic-membership"),
            next_config_epoch: NEXT_CONFIG_EPOCH,
        },
        ConfigurationTransition::JointConsensus {
            next_membership_ref: test_ref("joint-membership"),
            next_config_epoch: NEXT_CONFIG_EPOCH,
        },
    ] {
        let plan =
            plan_consistency_operation(&binding, command_for(&binding, ConsistencyOperation::Configure { transition }))
                .expect("configuration denial");
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "unsupported-configuration-transition"));
    }

    let oversized = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Propose {
            command_ref: test_ref("oversized-command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: MAX_COMMAND_BYTES + NEXT_CONSISTENCY_EPOCH_STEP,
        }),
    )
    .expect("oversized denial");
    assert_eq!(oversized.diagnostics, vec!["command-byte-bound-denied"]);

    let mut exhausted_command = command_for(&binding, ConsistencyOperation::Health);
    exhausted_command.observed_in_flight_operations = MAX_IN_FLIGHT;
    let exhausted = plan_consistency_operation(&binding, exhausted_command).expect("exhausted denial");
    assert_eq!(exhausted.diagnostics, vec!["in-flight-operation-bound-exhausted"]);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn fabric_consistency_draining_denies_mutation_read_recovery_and_configuration() {
    let active = active_binding();
    let drain =
        plan_consistency_operation(&active, command_for(&active, ConsistencyOperation::Drain)).expect("drain plan");
    let drain_outcome = normalized_success(&active, &drain, ConsistencyOutcomeKind::Drained);
    let draining = apply_consistency_outcome(&active, &drain, &drain_outcome).expect("draining");
    let denied_operations = vec![
        ConsistencyOperation::Propose {
            command_ref: test_ref("draining-command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: COMMAND_BYTES,
        },
        ConsistencyOperation::Read {
            query_ref: test_ref("draining-query"),
            mode: ConsistencyReadMode::Linearizable,
        },
        ConsistencyOperation::Recover {
            snapshot_ref: test_ref("snapshot"),
            durable_boundary_ref: test_ref("durable-boundary"),
        },
        ConsistencyOperation::Configure {
            transition: ConfigurationTransition::StaticMembershipRefresh {
                next_membership_ref: test_ref("next-membership"),
                next_config_epoch: NEXT_CONFIG_EPOCH,
            },
        },
    ];
    for operation in denied_operations {
        let plan = plan_consistency_operation(&draining, command_for(&draining, operation)).expect("draining denial");
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "operation-denied-for-lifecycle"));
    }
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_outcomes_reject_wrong_kind_stale_epoch_and_bad_shape() {
    let binding = active_binding();
    let propose = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Propose {
            command_ref: test_ref("command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: COMMAND_BYTES,
        }),
    )
    .expect("propose");
    let base = ConsistencyOutcomeInput {
        request_ref: propose.request_ref.clone(),
        binding_ref: propose.binding_ref.clone(),
        service_generation: binding.service_generation,
        config_epoch: binding.config_epoch,
        fencing_epoch: binding.fencing_epoch,
        kind: ConsistencyOutcomeKind::Committed,
        result_ref: Some(test_ref("result")),
        evidence_refs: vec![test_ref("evidence")],
        diagnostics: Vec::new(),
    };

    let mut wrong_kind = base.clone();
    wrong_kind.kind = ConsistencyOutcomeKind::ReadCurrent;
    assert!(normalize_consistency_outcome(&binding, &propose, wrong_kind).is_err());

    let mut stale = base.clone();
    stale.fencing_epoch += NEXT_CONSISTENCY_EPOCH_STEP;
    assert!(normalize_consistency_outcome(&binding, &propose, stale).is_err());

    let mut bad_failure = base;
    bad_failure.kind = ConsistencyOutcomeKind::Uncertain;
    bad_failure.diagnostics = vec!["delivery-uncertain".to_string()];
    assert!(normalize_consistency_outcome(&binding, &propose, bad_failure).is_err());
}
