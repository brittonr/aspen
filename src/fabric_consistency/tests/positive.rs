use super::*;

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn binding_open_and_attach_are_canonical() {
    let left = declared_binding();
    let right = declared_binding();
    assert_eq!(left, right);
    assert_eq!(left.lifecycle, GroupLifecycle::Declared);

    let create = plan(
        &left,
        command_for(&left, Request::Open {
            mode: GroupOpenMode::Create,
        }),
    )
    .expect("create plan");
    assert!(create.admitted());
    assert_eq!(create.lifecycle_after, GroupLifecycle::Active);
    assert_eq!(normalized_success(&left, &create, Kind::Opened).kind, Kind::Opened);

    let attached_source = declared_binding();
    let attach = plan(
        &attached_source,
        command_for(&attached_source, Request::Open {
            mode: GroupOpenMode::Attach,
        }),
    )
    .expect("attach plan");
    assert!(attach.admitted());
    let attach_outcome = normalized_success(&attached_source, &attach, Kind::Opened);
    let attached = apply_outcome(&attached_source, &attach, &attach_outcome).expect("attached");
    assert_eq!(attached.lifecycle, GroupLifecycle::Active);
    assert_ne!(attached.binding_ref, attached_source.binding_ref);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn active_port_normalizes_proposals_and_reads() {
    let binding = active_binding();

    let propose = plan(
        &binding,
        command_for(&binding, Request::Propose {
            command_ref: test_ref("command"),
            command_schema_ref: test_ref("command-schema"),
            estimated_command_bytes: COMMAND_BYTES,
        }),
    )
    .expect("propose");
    assert!(propose.admitted());
    assert_eq!(normalized_success(&binding, &propose, Kind::Committed).kind, Kind::Committed);

    let linearizable = plan(
        &binding,
        command_for(&binding, Request::Read {
            query_ref: test_ref("linearizable-query"),
            mode: ReadMode::Linearizable,
        }),
    )
    .expect("linearizable read");
    assert!(linearizable.admitted());
    assert_eq!(normalized_success(&binding, &linearizable, Kind::ReadCurrent,).kind, Kind::ReadCurrent);

    let local = plan(
        &binding,
        command_for(&binding, Request::Read {
            query_ref: test_ref("local-query"),
            mode: ReadMode::LocalStale,
        }),
    )
    .expect("local read");
    assert!(local.admitted());
    assert_eq!(normalized_success(&binding, &local, Kind::ReadLocal).kind, Kind::ReadLocal);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
// r[verify molten.fabric_consistency.group_isolation]
fn active_port_handles_maintenance_operations() {
    let binding = active_binding();
    let snapshot = plan(
        &binding,
        command_for(&binding, Request::Snapshot {
            snapshot_policy_ref: test_ref("snapshot-policy"),
        }),
    )
    .expect("snapshot");
    assert!(snapshot.admitted());
    normalized_success(&binding, &snapshot, Kind::SnapshotCreated);

    let recover = plan(
        &binding,
        command_for(&binding, Request::Recover {
            snapshot_ref: test_ref("snapshot"),
            durable_boundary_ref: test_ref("durable-boundary"),
        }),
    )
    .expect("recover");
    assert!(recover.admitted());
    normalized_success(&binding, &recover, Kind::Recovered);

    let configure = plan(
        &binding,
        command_for(&binding, Request::Configure {
            transition: ConfigurationTransition::StaticMembershipRefresh {
                next_membership_ref: test_ref("next-membership"),
                next_config_epoch: NEXT_CONFIG_EPOCH,
            },
        }),
    )
    .expect("configure");
    assert!(configure.admitted());
    let configuration_outcome = normalized_success(&binding, &configure, Kind::ConfigurationApplied);
    let configured = apply_outcome(&binding, &configure, &configuration_outcome).expect("configured binding");
    assert_eq!(configured.config_epoch, NEXT_CONFIG_EPOCH);
    assert_eq!(configured.membership_ref, test_ref("next-membership"));

    let health = plan(&binding, command_for(&binding, Request::Health)).expect("health");
    assert!(health.admitted());
    normalized_success(&binding, &health, Kind::HealthObserved);

    let status = plan(&binding, command_for(&binding, Request::Status)).expect("status");
    assert!(status.admitted());
    normalized_success(&binding, &status, Kind::StatusObserved);
}

#[test]
// r[verify molten.fabric_consistency.extension_port]
fn lifecycle_drains_and_removes_without_hidden_mutation() {
    let active = active_binding();
    let drain = plan(&active, command_for(&active, Request::Drain)).expect("drain");
    assert!(drain.admitted());
    let drain_outcome = normalized_success(&active, &drain, Kind::Drained);
    let draining = apply_outcome(&active, &drain, &drain_outcome).expect("draining");
    assert_eq!(draining.lifecycle, GroupLifecycle::Draining);

    let remove = plan(&draining, command_for(&draining, Request::Remove)).expect("remove");
    assert!(remove.admitted());
    let remove_outcome = normalized_success(&draining, &remove, Kind::Removed);
    let removed = apply_outcome(&draining, &remove, &remove_outcome).expect("removed");
    assert_eq!(removed.lifecycle, GroupLifecycle::Removed);

    let status = plan(&removed, command_for(&removed, Request::Status)).expect("removed status");
    assert!(status.admitted());
    normalized_success(&removed, &status, Kind::StatusObserved);
}
