use super::*;

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn binding_rejects_missing_policy_lease_and_over_bounds() {
    let mut missing_policy = binding_input();
    missing_policy.policy_refs.clear();
    assert!(canonical_group(missing_policy).is_err());

    let mut lease = binding_input();
    lease.supported_read_modes.push(ReadMode::Lease);
    assert!(canonical_group(lease).is_err());

    let mut command_bound = binding_input();
    command_bound.max_command_bytes = MAX_CONSISTENCY_COMMAND_BYTES + NEXT_CONSISTENCY_EPOCH_STEP;
    assert!(canonical_group(command_bound).is_err());

    let mut in_flight_bound = binding_input();
    in_flight_bound.max_in_flight_operations =
        MAX_CONSISTENCY_IN_FLIGHT_OPERATIONS + u32::try_from(NEXT_CONSISTENCY_EPOCH_STEP).expect("step");
    assert!(canonical_group(in_flight_bound).is_err());
}

#[test]
// r[verify molten.fabric_consistency.group_isolation]
fn exact_binding_mismatches_deny_without_mutation() {
    let binding = active_binding();
    let mut command = command_for(&binding, Request::Propose {
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

    let plan = plan(&binding, command).expect("denied plan");
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
    assert_eq!(denial.kind, Kind::Denied);
    assert_eq!(apply_outcome(&binding, &plan, &denial).expect("denial is non-mutating"), binding);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn unsupported_modes_and_resource_exhaustion_deny() {
    let binding = active_binding();
    let lease = plan(
        &binding,
        command_for(&binding, Request::Read {
            query_ref: test_ref("lease-query"),
            mode: ReadMode::Lease,
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
            plan(&binding, command_for(&binding, Request::Configure { transition })).expect("configuration denial");
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "unsupported-configuration-transition"));
    }

    let oversized = plan(
        &binding,
        command_for(&binding, Request::Propose {
            command_ref: test_ref("oversized-command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: MAX_COMMAND_BYTES + NEXT_CONSISTENCY_EPOCH_STEP,
        }),
    )
    .expect("oversized denial");
    assert_eq!(oversized.diagnostics, vec!["command-byte-bound-denied"]);

    let mut exhausted_command = command_for(&binding, Request::Health);
    exhausted_command.observed_in_flight_operations = MAX_IN_FLIGHT;
    let exhausted = plan(&binding, exhausted_command).expect("exhausted denial");
    assert_eq!(exhausted.diagnostics, vec!["in-flight-operation-bound-exhausted"]);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn draining_denies_mutation_read_recovery_and_configuration() {
    let active = active_binding();
    let drain = plan(&active, command_for(&active, Request::Drain)).expect("drain plan");
    let drain_outcome = normalized_success(&active, &drain, Kind::Drained);
    let draining = apply_outcome(&active, &drain, &drain_outcome).expect("draining");
    let denied_operations = vec![
        Request::Propose {
            command_ref: test_ref("draining-command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: COMMAND_BYTES,
        },
        Request::Read {
            query_ref: test_ref("draining-query"),
            mode: ReadMode::Linearizable,
        },
        Request::Recover {
            snapshot_ref: test_ref("snapshot"),
            durable_boundary_ref: test_ref("durable-boundary"),
        },
        Request::Configure {
            transition: ConfigurationTransition::StaticMembershipRefresh {
                next_membership_ref: test_ref("next-membership"),
                next_config_epoch: NEXT_CONFIG_EPOCH,
            },
        },
    ];
    for operation in denied_operations {
        let plan = plan(&draining, command_for(&draining, operation)).expect("draining denial");
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "operation-denied-for-lifecycle"));
    }
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn outcomes_reject_wrong_kind_stale_epoch_and_bad_shape() {
    let binding = active_binding();
    let propose = plan(
        &binding,
        command_for(&binding, Request::Propose {
            command_ref: test_ref("command"),
            command_schema_ref: test_ref("schema"),
            estimated_command_bytes: COMMAND_BYTES,
        }),
    )
    .expect("propose");
    let base = Input {
        request_ref: propose.request_ref.clone(),
        binding_ref: propose.binding_ref.clone(),
        service_generation: binding.service_generation,
        config_epoch: binding.config_epoch,
        fencing_epoch: binding.fencing_epoch,
        kind: Kind::Committed,
        result_ref: Some(test_ref("result")),
        evidence_refs: vec![test_ref("evidence")],
        diagnostics: Vec::new(),
    };

    let mut wrong_kind = base.clone();
    wrong_kind.kind = Kind::ReadCurrent;
    assert!(normalize(&binding, &propose, wrong_kind).is_err());

    let mut stale = base.clone();
    stale.fencing_epoch += NEXT_CONSISTENCY_EPOCH_STEP;
    assert!(normalize(&binding, &propose, stale).is_err());

    let mut bad_failure = base;
    bad_failure.kind = Kind::Uncertain;
    bad_failure.diagnostics = vec!["delivery-uncertain".to_string()];
    assert!(normalize(&binding, &propose, bad_failure).is_err());
}
