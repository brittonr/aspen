//! Current-run shutdown observation and restart lineage tests.

fn current_run_protected_state(root: &std::path::Path) -> Vec<(std::path::PathBuf, Option<Vec<u8>>)> {
    let mut paths = vec![
        root.join(super::super::settings::STARTUP_FILE),
        root.join(super::super::settings::HEALTH_FILE),
        root.join(super::super::settings::SHUTDOWN_FILE),
        root.join(super::super::settings::CONTROL_LOCK_FILE),
        root.join(super::super::settings::CONTROL_STOP_FILE),
        root.join(super::super::tracking::CURRENT_RUN_FILE),
    ];
    paths.extend(
        crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
            .iter()
            .map(|name| root.join(format!("receipts/adapter-shutdown-{name}.preserves"))),
    );
    paths
        .into_iter()
        .map(|path| {
            let bytes = if path.exists() {
                Some(std::fs::read(&path).expect("protected lifecycle bytes"))
            } else {
                None
            };
            (path, bytes)
        })
        .collect()
}

#[test]
fn current_run_shutdown_observation_binds_completion_and_restart_lineage() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let workspace = crate::test_support::process_workspace("node-current-run-lineage").expect("isolated workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &workspace,
        node_id: "node:lineage",
    })
    .expect("initialize");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &workspace })
        .expect("first run");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
    let first = super::super::tracking::current_node_run_binding(&root).expect("first binding");
    let stopped = super::super::stopping::stop_local_with_root(&root).expect("current shutdown");
    let observations = super::super::tracking::read_node_run_observations(&root, &first.startup_receipt_ref)
        .expect("completed observations");
    let control = crate::node_runtime::parse_control_receipt(&stopped.control_receipt_value).expect("shutdown control");
    let completion = observations.shutdown_observation.as_ref().expect("run-bound completion");
    assert_eq!(completion.run_ref, first.run_ref);
    assert_eq!(completion.shutdown_receipt_ref, stopped.shutdown_ref);
    assert_eq!(completion.control_receipt_ref, stopped.control_receipt_ref);
    assert_eq!(
        observations.classify("shutdown", Some(&control), true).lifecycle,
        crate::node_runtime::NodeRunLifecycle::Stopped
    );
    super::super::running::run_local_with_root_fixture_for_tests(&root).expect("restart with completed evidence");
    let second = super::super::tracking::current_node_run_binding(&root).expect("second binding");
    assert_eq!(second.previous_run_ref.as_deref(), Some(first.run_ref.as_str()));
    assert_ne!(second.run_ref, first.run_ref);
    assert!(!workspace.join(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE).exists());
    assert_eq!(
        super::super::leases::require_active_lock(&root).expect("current active binding").run_ref,
        second.run_ref
    );
}

#[test]
fn current_run_observation_rejection_precedes_shutdown_effects() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    for damage in [
        "missing-run",
        "unparseable-run",
        "unreadable-completion",
        "conflicting-lock",
        "conflicting-completion",
    ] {
        let workspace =
            crate::test_support::process_workspace("node-current-run-rejection").expect("isolated workspace");
        super::super::bootstrapping::init_local(&super::super::requests::InitInput {
            state_root: &workspace,
            node_id: "node:observation-rejection",
        })
        .expect("initialize");
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput {
            state_root: &workspace,
        })
        .expect("run node");
        let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
        let binding = super::super::tracking::current_node_run_binding(&root).expect("current binding");
        damage_current_run(damage, &workspace, &root, &binding);
        let before = current_run_protected_state(&workspace);
        let _error = super::super::stopping::stop_local_with_root(&root)
            .expect_err("inconsistent current-run evidence rejects shutdown");
        super::super::running::status_local_with_root(&root)
            .expect_err("inconsistent current-run observations reject status");
        assert_eq!(current_run_protected_state(&workspace), before, "lifecycle changed after {damage}");
        assert!(!workspace.join(super::super::settings::SHUTDOWN_FILE).exists());
        for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
            assert!(!workspace.join(format!("receipts/adapter-shutdown-{name}.preserves")).exists());
        }
    }
}

fn damage_current_run(
    damage: &str,
    workspace: &std::path::Path,
    root: &crate::node_state::DirectoryRoot,
    binding: &crate::node_runtime::NodeRunBinding,
) {
    match damage {
        "missing-run" => {
            std::fs::remove_file(workspace.join(super::super::tracking::CURRENT_RUN_FILE)).expect("remove binding")
        }
        "unparseable-run" => std::fs::write(workspace.join(super::super::tracking::CURRENT_RUN_FILE), b"<invalid-run>")
            .expect("damage binding"),
        "unreadable-completion" => {
            std::fs::create_dir(workspace.join(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE))
                .expect("non-file observation")
        }
        "conflicting-lock" => {
            let other = super::super::storage::local_ref("different-node-run", "conflicting").expect("other run ref");
            let value = super::super::leases::active_lock_value(root, &binding.startup_receipt_ref, &other)
                .expect("conflicting lock");
            super::super::storage::write_preserves(
                root,
                &super::super::storage::fixed_node_path(super::super::settings::CONTROL_LOCK_FILE).expect("lock path"),
                &value,
            )
            .expect("replace lock");
        }
        "conflicting-completion" => {
            let other = super::super::storage::local_ref("different-node-run", "conflicting").expect("other run ref");
            let value = crate::node_runtime::node_run_shutdown_observation_value(
                &crate::node_runtime::NodeRunShutdownObservationValueInput {
                    run_ref: &other,
                    startup_receipt_ref: &binding.startup_receipt_ref,
                    shutdown_receipt_ref: &other,
                    control_receipt_ref: &other,
                },
            )
            .expect("conflicting completion");
            super::super::storage::write_preserves(
                root,
                &super::super::storage::fixed_node_path(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE)
                    .expect("completion path"),
                &value,
            )
            .expect("place conflicting completion");
        }
        _ => unreachable!("bounded damage cases"),
    }
}

#[test]
fn legacy_completion_without_run_binding_cannot_report_stopped_or_restart() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    for missing in [
        super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE,
        super::super::tracking::CURRENT_RUN_FILE,
    ] {
        let workspace =
            crate::test_support::process_workspace("node-historical-completion-only").expect("isolated workspace");
        super::super::bootstrapping::init_local(&super::super::requests::InitInput {
            state_root: &workspace,
            node_id: "node:historical",
        })
        .expect("initialize");
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput {
            state_root: &workspace,
        })
        .expect("run node");
        let stopped = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &workspace })
            .expect("complete original run");
        std::fs::remove_file(workspace.join(missing)).expect("remove current-run observation");
        let before = current_run_protected_state(&workspace);
        assert_eq!(
            crate::node_runtime::parse_node_shutdown_receipt(&stopped.shutdown_value)
                .expect("historical shutdown remains readable")
                .receipt_ref,
            stopped.shutdown_ref
        );
        assert_eq!(
            crate::node_runtime::parse_control_receipt(&stopped.control_receipt_value)
                .expect("historical control remains readable")
                .receipt_ref,
            stopped.control_receipt_ref
        );
        super::super::running::status_local(&super::super::requests::StatusInput { state_root: &workspace })
            .expect_err("legacy receipt alone cannot prove current stopped");
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput {
            state_root: &workspace,
        })
        .expect_err("missing run lineage cannot reset the run identity");
        assert_eq!(current_run_protected_state(&workspace), before);
        assert!(!workspace.join(super::super::settings::CONTROL_LOCK_FILE).exists());
    }
}

#[test]
fn denied_repeat_stop_preserves_completed_run_status_and_restart() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let workspace = crate::test_support::process_workspace("node-repeat-stop-readback").expect("isolated workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &workspace,
        node_id: "node:repeat-stop",
    })
    .expect("initialize");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &workspace })
        .expect("run node");
    let stopped = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &workspace })
        .expect("complete current run");
    super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &workspace })
        .expect_err("repeat stop remains denied");
    assert_eq!(
        super::super::running::status_local(&super::super::requests::StatusInput { state_root: &workspace })
            .expect("completed status")
            .status,
        "stopped"
    );
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
    let startup = super::super::leases::current_startup_receipt(&root).expect("completed startup");
    let observations = super::super::tracking::read_node_run_observations(&root, &startup.receipt_ref)
        .expect("completed observations");
    assert_eq!(
        super::super::tracking::completed_node_run_control(&root, &observations)
            .expect("immutable control")
            .expect("completion control")
            .receipt_ref,
        stopped.control_receipt_ref
    );
    super::super::running::run_local_with_root_fixture_for_tests(&root)
        .expect("restart retains successful completion despite later denial");
    assert_eq!(super::super::running::status_local_with_root(&root).expect("restarted status").status, "running");
}
