
    fn assert_dispatch_requires_lock(root: &Path) {
        let error = dispatch_control_request_entry(&ControlDispatchEntryInput {
            state_root: root,
            request_entry: None,
        })
        .expect_err("dispatch requires lock");
        assert!(error.to_string().contains("active node lock"));
    }

    #[test]
    fn compatibility_request_path_rejects_directory_authority() {
        // r[verify molten.node.cap_std_request_lifecycle]
        let root = temp_dir("node-control-compatibility-path-deny");
        let error = dispatch_control_request(&ControlDispatchInput {
            state_root: &root,
            request_path: Some(Path::new("../outside.preserves")),
        })
        .expect_err("directory-bearing compatibility path must deny");
        assert!(error.to_string().contains("selected state root inbox"));
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
        dispatch_control_request_entry(&ControlDispatchEntryInput {
            state_root: &root,
            request_entry: Some(&submitted.inbox_entry),
        })
        .expect("dispatch status");
        let state_root = crate::node_state::NodeStateRoot::open(&root).expect("open node state root");
        write_preserves(
            &state_root,
            &control_outbox_request_path(&status_request.request_ref).expect("outbox request path"),
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
        let denied = dispatch_control_request_entry(&ControlDispatchEntryInput {
            state_root: &root,
            request_entry: Some(&duplicate.inbox_entry),
        })
        .expect_err("conflicting duplicate denied");
        assert!(denied.to_string().contains("conflicts with archived request evidence"));
    }
