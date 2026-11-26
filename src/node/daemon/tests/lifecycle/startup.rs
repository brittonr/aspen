//! Local init, run, status, stop, and restart lifecycle tests.

#[test]
fn ingress_ref_parser_rejects_short_fixture_refs() {
    let error = super::super::guards::validate_ingress_ref("blake3:fixture", "node control ingress payload ref")
        .expect_err("short fixture ref denied");
    assert!(error.to_string().contains("canonical blake3 content ref"));
    super::super::guards::validate_ingress_ref(
        "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "node control ingress payload ref",
    )
    .expect("valid canonical ref");
}

#[test]
fn local_node_init_run_status_stop_and_restart_recovery_are_receipted() {
    let root = crate::test_support::process_workspace("node_daemon_lifecycle").expect("create isolated node workspace");
    let init = super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:test",
    })
    .expect("init node");
    crate::preserves_rail::validate_content_ref(&init.config_ref).expect("config ref is canonical");
    crate::preserves_rail::validate_content_ref(&init.profile_resolution_ref)
        .expect("profile resolution ref is canonical");
    let resolution_text = crate::preserves_rail::to_text(&init.profile_resolution_value).expect("resolution text");
    assert!(resolution_text.contains("local-fixture-config"));
    let run =
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
            .expect("run node");
    crate::preserves_rail::validate_content_ref(&run.startup_ref).expect("startup ref is canonical");
    assert_eq!(run.adapter_receipt_refs.len(), crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    let startup = crate::node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup parse");
    assert!(startup.profile_metadata_refs.contains(&init.profile_resolution_ref));
    let status = super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
        .expect("status node");
    assert_eq!(status.status, "running");
    let stop = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &root })
        .expect("stop node");
    crate::preserves_rail::validate_content_ref(&stop.shutdown_ref).expect("shutdown ref is canonical");
    let stopped = super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
        .expect("stopped status");
    assert_eq!(stopped.status, "stopped");
    let restarted =
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
            .expect("restart node");
    crate::preserves_rail::validate_content_ref(&restarted.startup_ref).expect("restart startup ref is canonical");
    let restarted_status =
        super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
            .expect("restarted status");
    assert_eq!(restarted_status.status, "running");
    let stale =
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
            .expect_err("stale running state denied");
    assert!(stale.to_string().contains("previous startup has no clean shutdown receipt"));
    let restart =
        crate::node_runtime::node_restart_health_receipt_value(&crate::node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &startup,
            shutdown_receipt_ref: Some(&stop.shutdown_ref),
            index_receipt_refs: &super::super::storage::index_receipt_refs(&root).expect("index refs"),
            head_refs: std::slice::from_ref(&run.startup_ref),
            open_job_refs: &[],
            diagnostics: &[],
        })
        .expect("restart health");
    let restart_health = crate::node_runtime::parse_node_health_receipt(&restart).expect("parse health");
    assert_eq!(restart_health.decision, "pass");
}

#[test]
fn init_denies_existing_lifecycle_states() {
    let initialized_root = super::fixtures::temp_dir("node-daemon-reinit-initialized");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &initialized_root,
        node_id: "node:initialized",
    })
    .expect("init once");
    let initialized = super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &initialized_root,
        node_id: "node:initialized",
    })
    .expect_err("reinit initialized denied");
    assert!(initialized.to_string().contains("already has Initialized lifecycle state"));

    let running_root = super::fixtures::temp_dir("node-daemon-reinit-running");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &running_root,
        node_id: "node:running",
    })
    .expect("init running root");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput {
        state_root: &running_root,
    })
    .expect("run root");
    let running = super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &running_root,
        node_id: "node:running",
    })
    .expect_err("reinit running denied");
    assert!(running.to_string().contains("already has Running lifecycle state"));

    let stopped_root = super::fixtures::temp_dir("node-daemon-reinit-stopped");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &stopped_root,
        node_id: "node:stopped",
    })
    .expect("init stopped root");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput {
        state_root: &stopped_root,
    })
    .expect("run stopped root");
    super::super::stopping::stop_local(&super::super::requests::StopInput {
        state_root: &stopped_root,
    })
    .expect("stop root");
    let stopped = super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &stopped_root,
        node_id: "node:stopped",
    })
    .expect_err("reinit stopped denied");
    assert!(stopped.to_string().contains("already has Stopped lifecycle state"));
}

#[test]
fn lifecycle_state_classifier_covers_valid_and_invalid_states() {
    let empty = super::super::inspection::NodeLifecycleFiles {
        has_config: false,
        has_identity_receipt: false,
        has_startup: false,
        has_shutdown: false,
        has_active_lock: false,
    };
    assert_eq!(
        super::super::inspection::node_lifecycle_state(&empty),
        super::super::inspection::NodeLifecycleState::Empty
    );

    let initialized = super::super::inspection::NodeLifecycleFiles {
        has_config: true,
        has_identity_receipt: true,
        has_startup: false,
        has_shutdown: false,
        has_active_lock: false,
    };
    assert_eq!(
        super::super::inspection::node_lifecycle_state(&initialized),
        super::super::inspection::NodeLifecycleState::Initialized
    );

    let running = super::super::inspection::NodeLifecycleFiles {
        has_config: true,
        has_identity_receipt: true,
        has_startup: true,
        has_shutdown: false,
        has_active_lock: true,
    };
    assert_eq!(
        super::super::inspection::node_lifecycle_state(&running),
        super::super::inspection::NodeLifecycleState::Running
    );

    let stopped = super::super::inspection::NodeLifecycleFiles {
        has_config: true,
        has_identity_receipt: true,
        has_startup: true,
        has_shutdown: true,
        has_active_lock: false,
    };
    assert_eq!(
        super::super::inspection::node_lifecycle_state(&stopped),
        super::super::inspection::NodeLifecycleState::Stopped
    );

    let stale_lock_without_startup = super::super::inspection::NodeLifecycleFiles {
        has_config: true,
        has_identity_receipt: true,
        has_startup: false,
        has_shutdown: false,
        has_active_lock: true,
    };
    assert_eq!(
        super::super::inspection::node_lifecycle_state(&stale_lock_without_startup),
        super::super::inspection::NodeLifecycleState::Inconsistent
    );
}

#[test]
fn ambient_current_directory_state_root_is_denied() {
    let denied = super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: std::path::Path::new("."),
        node_id: "node:test",
    })
    .expect_err("ambient state denied");
    assert!(denied.to_string().contains("ambient current directory"));
    let request = super::super::running::status_request().expect("status request");
    let control_denied =
        super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
            state_root: std::path::Path::new("."),
            request_value: &request.value,
        })
        .expect_err("ambient control denied");
    assert!(control_denied.to_string().contains("ambient current directory"));
}
