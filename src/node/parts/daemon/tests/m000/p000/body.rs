    use super::*;

    #[test]
    fn ingress_ref_parser_rejects_short_fixture_refs() {
        let error = validate_ingress_ref("blake3:fixture", "node control ingress payload ref")
            .expect_err("short fixture ref denied");
        assert!(error.to_string().contains("canonical blake3 content ref"));
        validate_ingress_ref(
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "node control ingress payload ref",
        )
        .expect("valid canonical ref");
    }

    #[test]
    fn local_node_init_run_status_stop_and_restart_recovery_are_receipted() {
        let root = temp_dir("node-daemon-lifecycle");
        let init = init_local(&InitInput {
            state_root: &root,
            node_id: "node:test",
        })
        .expect("init node");
        crate::preserves_rail::validate_content_ref(&init.config_ref).expect("config ref is canonical");
        crate::preserves_rail::validate_content_ref(&init.profile_resolution_ref)
            .expect("profile resolution ref is canonical");
        let resolution_text = crate::preserves_rail::to_text(&init.profile_resolution_value).expect("resolution text");
        assert!(resolution_text.contains("local-fixture-config"));
        let run = run_local(&RunInput { state_root: &root }).expect("run node");
        crate::preserves_rail::validate_content_ref(&run.startup_ref).expect("startup ref is canonical");
        assert_eq!(run.adapter_receipt_refs.len(), crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
        let startup = crate::node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup parse");
        assert!(startup.profile_metadata_refs.contains(&init.profile_resolution_ref));
        let status = status_local(&StatusInput { state_root: &root }).expect("status node");
        assert_eq!(status.status, "running");
        let stop = stop_local(&StopInput { state_root: &root }).expect("stop node");
        crate::preserves_rail::validate_content_ref(&stop.shutdown_ref).expect("shutdown ref is canonical");
        let stopped = status_local(&StatusInput { state_root: &root }).expect("stopped status");
        assert_eq!(stopped.status, "stopped");
        let restarted = run_local(&RunInput { state_root: &root }).expect("restart node");
        crate::preserves_rail::validate_content_ref(&restarted.startup_ref).expect("restart startup ref is canonical");
        let restarted_status = status_local(&StatusInput { state_root: &root }).expect("restarted status");
        assert_eq!(restarted_status.status, "running");
        let stale = run_local(&RunInput { state_root: &root }).expect_err("stale running state denied");
        assert!(stale.to_string().contains("previous startup has no clean shutdown receipt"));
        let restart = crate::node_runtime::node_restart_health_receipt_value(
            &crate::node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&stop.shutdown_ref),
                index_receipt_refs: &index_receipt_refs(&root).expect("index refs"),
                head_refs: std::slice::from_ref(&run.startup_ref),
                open_job_refs: &[],
                diagnostics: &[],
            },
        )
        .expect("restart health");
        let restart_health = crate::node_runtime::parse_node_health_receipt(&restart).expect("parse health");
        assert_eq!(restart_health.decision, "pass");
    }

    #[test]
    fn init_denies_existing_lifecycle_states() {
        let initialized_root = temp_dir("node-daemon-reinit-initialized");
        init_local(&InitInput {
            state_root: &initialized_root,
            node_id: "node:initialized",
        })
        .expect("init once");
        let initialized = init_local(&InitInput {
            state_root: &initialized_root,
            node_id: "node:initialized",
        })
        .expect_err("reinit initialized denied");
        assert!(initialized.to_string().contains("already has Initialized lifecycle state"));

        let running_root = temp_dir("node-daemon-reinit-running");
        init_local(&InitInput {
            state_root: &running_root,
            node_id: "node:running",
        })
        .expect("init running root");
        run_local(&RunInput { state_root: &running_root }).expect("run root");
        let running = init_local(&InitInput {
            state_root: &running_root,
            node_id: "node:running",
        })
        .expect_err("reinit running denied");
        assert!(running.to_string().contains("already has Running lifecycle state"));

        let stopped_root = temp_dir("node-daemon-reinit-stopped");
        init_local(&InitInput {
            state_root: &stopped_root,
            node_id: "node:stopped",
        })
        .expect("init stopped root");
        run_local(&RunInput { state_root: &stopped_root }).expect("run stopped root");
        stop_local(&StopInput { state_root: &stopped_root }).expect("stop root");
        let stopped = init_local(&InitInput {
            state_root: &stopped_root,
            node_id: "node:stopped",
        })
        .expect_err("reinit stopped denied");
        assert!(stopped.to_string().contains("already has Stopped lifecycle state"));
    }

    #[test]
    fn lifecycle_state_classifier_covers_valid_and_invalid_states() {
        let empty = NodeLifecycleFiles {
            has_config: false,
            has_identity_receipt: false,
            has_startup: false,
            has_shutdown: false,
            has_active_lock: false,
        };
        assert_eq!(node_lifecycle_state(&empty), NodeLifecycleState::Empty);

        let initialized = NodeLifecycleFiles {
            has_config: true,
            has_identity_receipt: true,
            has_startup: false,
            has_shutdown: false,
            has_active_lock: false,
        };
        assert_eq!(node_lifecycle_state(&initialized), NodeLifecycleState::Initialized);

        let running = NodeLifecycleFiles {
            has_config: true,
            has_identity_receipt: true,
            has_startup: true,
            has_shutdown: false,
            has_active_lock: true,
        };
        assert_eq!(node_lifecycle_state(&running), NodeLifecycleState::Running);

        let stopped = NodeLifecycleFiles {
            has_config: true,
            has_identity_receipt: true,
            has_startup: true,
            has_shutdown: true,
            has_active_lock: false,
        };
        assert_eq!(node_lifecycle_state(&stopped), NodeLifecycleState::Stopped);

        let stale_lock_without_startup = NodeLifecycleFiles {
            has_config: true,
            has_identity_receipt: true,
            has_startup: false,
            has_shutdown: false,
            has_active_lock: true,
        };
        assert_eq!(node_lifecycle_state(&stale_lock_without_startup), NodeLifecycleState::Inconsistent);
    }

    #[test]
    fn ambient_current_directory_state_root_is_denied() {
        let denied = init_local(&InitInput {
            state_root: Path::new("."),
            node_id: "node:test",
        })
        .expect_err("ambient state denied");
        assert!(denied.to_string().contains("ambient current directory"));
        let request = status_request().expect("status request");
        let control_denied = submit_control_request(&ControlSubmitInput {
            state_root: Path::new("."),
            request_value: &request.value,
        })
        .expect_err("ambient control denied");
        assert!(control_denied.to_string().contains("ambient current directory"));
    }

    #[test]
    fn control_inbox_dispatch_imports_receipts_and_denies_missing_operation_payloads() {
        let root = initialized_control_root("node-control-socket", "node:control");
        let status = assert_status_dispatch(&root);
        assert_missing_payload_denied(&root, &status);
        assert_missing_authority_denied(&root, &status);
        assert_shutdown_dispatch(&root);
        assert_dispatch_requires_lock(&root);
    }

    fn initialized_control_root(label: &str, node_id: &str) -> PathBuf {
        let root = temp_dir(label);
        init_local(&InitInput {
            state_root: &root,
            node_id,
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        root
    }

    fn submit_and_dispatch(root: &Path, request_value: &IoValue) -> ControlDispatch {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value,
        })
        .expect("submit request");
        assert!(submitted.inbox_path.exists());
        dispatch_control_request(&ControlDispatchInput {
            state_root: root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch request")
    }

    fn assert_status_dispatch(root: &Path) -> crate::node_runtime::ControlRequest {
        let request = status_request().expect("status request");
        let dispatched = submit_and_dispatch(root, &request.value);
        assert_eq!(dispatched.operation, "status");
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, request.request_ref);
        assert_ledger_contains(root, &[
            "node-control-request",
            "node-control-queue-receipt",
            "node-health-receipt",
            "node-control-receipt",
        ]);
        request
    }

    fn assert_ledger_contains(root: &Path, expected: &[&str]) {
        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        for expected_kind in expected {
            assert!(kinds.iter().any(|kind| kind.as_str() == *expected_kind), "missing ledger kind {expected_kind}");
        }
    }

    fn assert_missing_payload_denied(root: &Path, status: &crate::node_runtime::ControlRequest) {
        let target_ref = local_ref("install-target", "fixture").expect("target ref");
        let install_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: Some(&target_ref),
                payload_ref: None,
                authority_refs: &status.authority_refs,
                policy_refs: &status.policy_refs,
                resource_refs: &status.resource_refs,
                evidence_refs: &[],
            })
            .expect("install request");
        let dispatch = submit_and_dispatch(root, &install_value);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires payload ref")));
    }

    fn assert_missing_authority_denied(root: &Path, status: &crate::node_runtime::ControlRequest) {
        let missing_authority =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &[],
                policy_refs: &status.policy_refs,
                resource_refs: &status.resource_refs,
                evidence_refs: &[],
            })
            .expect("missing authority request");
        let dispatch = submit_and_dispatch(root, &missing_authority);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("missing receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));
    }

    fn assert_shutdown_dispatch(root: &Path) {
        let request = shutdown_request().expect("shutdown request");
        let dispatch = submit_and_dispatch(root, &request.value);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("shutdown receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
    }

    fn assert_dispatch_requires_lock(root: &Path) {
        let error = dispatch_control_request(&ControlDispatchInput {
            state_root: root,
            request_path: None,
        })
        .expect_err("dispatch requires lock");
        assert!(error.to_string().contains("active node lock"));
    }

    #[test]
    fn control_loop_processes_queue_idempotently_and_stops_on_shutdown() {
        let root = temp_dir("node-control-loop");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:loop",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        let first_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run one status request");
        assert_eq!(first_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert!(!first_loop.has_stopped);
        assert_eq!(crate::ledger::artifact_kind(&first_loop.loop_receipt_value), "node-control-loop-receipt");
        assert_eq!(crate::ledger::artifact_kind(&first_loop.heartbeat_receipt_value), "node-control-heartbeat-receipt");

        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate status");
        let duplicate_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run duplicate status request");
        assert_eq!(duplicate_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert_eq!(duplicate_loop.dispatch_receipt_refs, first_loop.dispatch_receipt_refs);

        let shutdown_request = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_loop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: DEFAULT_CONTROL_LOOP_REQUESTS,
        })
        .expect("run shutdown request");
        assert!(shutdown_loop.has_stopped);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        let after_stop = run_control_loop(&ControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect_err("stopped node loop denied");
        assert!(after_stop.to_string().contains("active node lock"));

        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list loop ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "node-control-loop-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-control-heartbeat-receipt"));
    }

    #[test]
    fn duplicate_request_with_conflicting_archive_fails_closed() {
        let root = temp_dir("node-control-duplicate-conflict");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:duplicate",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        dispatch_control_request(&ControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        write_preserves(
            &control_outbox_request_path(&root, &status_request.request_ref),
            &crate::preserves_rail::record("tampered-node-control-request", vec![crate::preserves_rail::string(
                "conflict",
            )]),
        )
        .expect("tamper archived request");
        let duplicate = submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate");
        let denied = dispatch_control_request(&ControlDispatchInput {
            state_root: &root,
            request_path: Some(&duplicate.inbox_path),
        })
        .expect_err("conflicting duplicate denied");
        assert!(denied.to_string().contains("conflicts with archived request evidence"));
    }
