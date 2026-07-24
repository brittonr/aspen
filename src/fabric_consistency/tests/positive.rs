use super::*;

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_binding_open_and_attach_are_canonical() {
    let left = declared_binding();
    let right = declared_binding();
    assert_eq!(left, right);
    assert_eq!(left.lifecycle, ConsistencyGroupLifecycle::Declared);

    let create = plan_consistency_operation(
        &left,
        command_for(&left, ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        }),
    )
    .expect("create plan");
    assert!(create.admitted());
    assert_eq!(create.lifecycle_after, ConsistencyGroupLifecycle::Active);
    assert_eq!(
        normalized_success(&left, &create, ConsistencyOutcomeKind::Opened).kind,
        ConsistencyOutcomeKind::Opened
    );

    let attached_source = declared_binding();
    let attach = plan_consistency_operation(
        &attached_source,
        command_for(&attached_source, ConsistencyOperation::Open {
            mode: GroupOpenMode::Attach,
        }),
    )
    .expect("attach plan");
    assert!(attach.admitted());
    let attach_outcome = normalized_success(&attached_source, &attach, ConsistencyOutcomeKind::Opened);
    let attached = apply_consistency_outcome(&attached_source, &attach, &attach_outcome).expect("attached");
    assert_eq!(attached.lifecycle, ConsistencyGroupLifecycle::Active);
    assert_ne!(attached.binding_ref, attached_source.binding_ref);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_active_port_normalizes_proposals_and_reads() {
    let binding = active_binding();

    let propose = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Propose {
            command_ref: test_ref("command"),
            command_schema_ref: test_ref("command-schema"),
            estimated_command_bytes: COMMAND_BYTES,
        }),
    )
    .expect("propose");
    assert!(propose.admitted());
    assert_eq!(
        normalized_success(&binding, &propose, ConsistencyOutcomeKind::Committed).kind,
        ConsistencyOutcomeKind::Committed
    );

    let linearizable = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Read {
            query_ref: test_ref("linearizable-query"),
            mode: ConsistencyReadMode::Linearizable,
        }),
    )
    .expect("linearizable read");
    assert!(linearizable.admitted());
    assert_eq!(
        normalized_success(&binding, &linearizable, ConsistencyOutcomeKind::ReadCurrent,).kind,
        ConsistencyOutcomeKind::ReadCurrent
    );

    let local = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Read {
            query_ref: test_ref("local-query"),
            mode: ConsistencyReadMode::LocalStale,
        }),
    )
    .expect("local read");
    assert!(local.admitted());
    assert_eq!(
        normalized_success(&binding, &local, ConsistencyOutcomeKind::ReadLocal).kind,
        ConsistencyOutcomeKind::ReadLocal
    );
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn fabric_consistency_active_port_handles_maintenance_operations() {
    let binding = active_binding();
    let snapshot = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Snapshot {
            snapshot_policy_ref: test_ref("snapshot-policy"),
        }),
    )
    .expect("snapshot");
    assert!(snapshot.admitted());
    normalized_success(&binding, &snapshot, ConsistencyOutcomeKind::SnapshotCreated);

    let recover = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Recover {
            snapshot_ref: test_ref("snapshot"),
            durable_boundary_ref: test_ref("durable-boundary"),
        }),
    )
    .expect("recover");
    assert!(recover.admitted());
    normalized_success(&binding, &recover, ConsistencyOutcomeKind::Recovered);

    let configure = plan_consistency_operation(
        &binding,
        command_for(&binding, ConsistencyOperation::Configure {
            transition: ConfigurationTransition::StaticMembershipRefresh {
                next_membership_ref: test_ref("next-membership"),
                next_config_epoch: NEXT_CONFIG_EPOCH,
            },
        }),
    )
    .expect("configure");
    assert!(configure.admitted());
    let configuration_outcome = normalized_success(&binding, &configure, ConsistencyOutcomeKind::ConfigurationApplied);
    let configured =
        apply_consistency_outcome(&binding, &configure, &configuration_outcome).expect("configured binding");
    assert_eq!(configured.config_epoch, NEXT_CONFIG_EPOCH);
    assert_eq!(configured.membership_ref, test_ref("next-membership"));

    let health =
        plan_consistency_operation(&binding, command_for(&binding, ConsistencyOperation::Health)).expect("health");
    assert!(health.admitted());
    normalized_success(&binding, &health, ConsistencyOutcomeKind::HealthObserved);

    let status =
        plan_consistency_operation(&binding, command_for(&binding, ConsistencyOperation::Status)).expect("status");
    assert!(status.admitted());
    normalized_success(&binding, &status, ConsistencyOutcomeKind::StatusObserved);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn fabric_consistency_lifecycle_drains_and_removes_without_hidden_mutation() {
    let active = active_binding();
    let drain = plan_consistency_operation(&active, command_for(&active, ConsistencyOperation::Drain)).expect("drain");
    assert!(drain.admitted());
    let drain_outcome = normalized_success(&active, &drain, ConsistencyOutcomeKind::Drained);
    let draining = apply_consistency_outcome(&active, &drain, &drain_outcome).expect("draining");
    assert_eq!(draining.lifecycle, ConsistencyGroupLifecycle::Draining);

    let remove =
        plan_consistency_operation(&draining, command_for(&draining, ConsistencyOperation::Remove)).expect("remove");
    assert!(remove.admitted());
    let remove_outcome = normalized_success(&draining, &remove, ConsistencyOutcomeKind::Removed);
    let removed = apply_consistency_outcome(&draining, &remove, &remove_outcome).expect("removed");
    assert_eq!(removed.lifecycle, ConsistencyGroupLifecycle::Removed);

    let status = plan_consistency_operation(&removed, command_for(&removed, ConsistencyOperation::Status))
        .expect("removed status");
    assert!(status.admitted());
    normalized_success(&removed, &status, ConsistencyOutcomeKind::StatusObserved);
}
