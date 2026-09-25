
    #[test]
    fn cleanup_task_requires_retention_and_dependency_impact_evidence() {
        let root = temp_dir("upgrade-cleanup-evidence");
        let store = root.join("upgrades");
        let ledger_root = root.join("ledger");
        let artifact = crate::ledger::import_artifact(&ledger_root, &parse_text("<old-artifact>").expect("artifact"))
            .expect("import artifact")
            .artifact_ref;
        let plan_value = upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-cleanup".to_string(),
            reason: "cleanup".to_string(),
            summary: "cleanup needs retention evidence".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![artifact.clone()],
            impact_refs: vec![artifact.clone()],
            tasks: vec![UpgradeTaskInput {
                task_id: "cleanup".to_string(),
                kind: "cleanup".to_string(),
                subject: "cleanup".to_string(),
                from_ref: Some(artifact.clone()),
                to_ref: None,
                precondition_refs: Vec::new(),
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![artifact.clone()],
                new_refs: vec![test_ref("replacement")],
                expires_at: None,
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![artifact],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("review-evidence")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("cleanup plan");
        let created = create_session(&store, &plan_value).expect("create cleanup plan");
        let cleanup = execute_task(&store, &ledger_root, &created.plan.plan_ref, "cleanup").expect("cleanup denied");
        assert_eq!(cleanup.receipt.decision, "deny");
        assert!(to_text(&cleanup.receipt.value).expect("cleanup text").contains("retention"));
    }

    #[test]
    fn upgrade_plan_requires_valid_source_gate_receipt_content() {
        let base_input = || UpgradePlanInput {
            session_id: "session-source-gate".to_string(),
            reason: "source gate".to_string(),
            summary: "validate strict source gate".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![test_ref("affected")],
            impact_refs: vec![test_ref("affected")],
            tasks: vec![UpgradeTaskInput {
                task_id: "transcript".to_string(),
                kind: "transcript-rerun".to_string(),
                subject: "source-gate".to_string(),
                from_ref: None,
                to_ref: None,
                precondition_refs: vec![test_ref("transcript")],
                postcondition_refs: Vec::new(),
                reversible: true,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("old")],
                new_refs: vec![test_ref("new")],
                expires_at: None,
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("old")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        };
        let pass = upgrade_plan_value(&base_input()).expect("passing source gate plan");
        let plan = parse_upgrade_plan(&pass).expect("parse pass plan");
        assert!(plan.evidence_refs.len() > 1);

        let mut missing = base_input();
        missing.source_gate_receipt_values.clear();
        assert!(
            upgrade_plan_value(&missing)
                .expect_err("missing source gate denied")
                .to_string()
                .contains("strict Octet source gate")
        );

        let denied_gate = parse_text(
            &to_text(&crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"))
                .expect("source gate text")
                .replacen("<decision \"pass\">", "<decision \"deny\">", 1),
        )
        .expect("denied gate parse");
        let mut denied = base_input();
        denied.source_gate_receipt_values = vec![denied_gate];
        assert!(
            upgrade_plan_value(&denied)
                .expect_err("denied source gate rejected")
                .to_string()
                .contains("source gate validation failed")
        );
    }

    #[test]
    fn protocol_drain_task_requires_passing_protocol_gate_evidence() {
        let root = temp_dir("upgrade-protocol-drain");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let gate = protocol_drain_gate();
        let gate_ref = crate::ledger::import_artifact(&ledger_root, &gate.value).expect("import gate").artifact_ref;
        assert_eq!(gate_ref, gate.receipt_ref);
        let new_protocol_ref = test_ref("protocol-v2");
        let plan_value =
            protocol_drain_plan_value(&gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("protocol drain plan");
        let created = create_session(&store, &plan_value).expect("create session");
        let executed =
            execute_task(&store, &ledger_root, &created.plan.plan_ref, "drain-sessions").expect("execute drain task");
        assert_eq!(executed.receipt.decision, "pass");
        let text = to_text(&executed.receipt.value).expect("receipt text");
        assert!(text.contains("protocol-session-drain"));
        assert!(status(&store, &created.plan.plan_ref).expect("status").remaining_task_ids.is_empty());
    }

    #[test]
    fn protocol_drain_task_denies_missing_stale_or_mismatched_gate_evidence() {
        let root = temp_dir("upgrade-protocol-drain-deny");
        let ledger_root = root.join("ledger");
        let missing_store = root.join("missing-upgrades");
        let gate = protocol_drain_gate();
        let new_protocol_ref = test_ref("protocol-v2");
        let missing_gate_ref = test_ref("missing-protocol-gate");
        let missing_plan =
            protocol_drain_plan_value(&missing_gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("missing plan");
        let missing_created = create_session(&missing_store, &missing_plan).expect("create missing session");
        let missing = execute_task(&missing_store, &ledger_root, &missing_created.plan.plan_ref, "drain-sessions")
            .expect("execute missing drain");
        assert_eq!(missing.receipt.decision, "deny");
        assert!(to_text(&missing.receipt.value).expect("missing text").contains("not readable from ledger"));

        let denied_store = root.join("denied-upgrades");
        let denied_gate = protocol_drain_gate_with_diagnostics(vec!["stale protocol lifecycle evidence".to_string()]);
        let denied_gate_ref = crate::ledger::import_artifact(&ledger_root, &denied_gate.value)
            .expect("import denied gate")
            .artifact_ref;
        let denied_plan =
            protocol_drain_plan_value(&denied_gate_ref, &gate.protocol_ref, &new_protocol_ref).expect("denied plan");
        let denied_created = create_session(&denied_store, &denied_plan).expect("create denied session");
        let denied = execute_task(&denied_store, &ledger_root, &denied_created.plan.plan_ref, "drain-sessions")
            .expect("execute denied drain");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(to_text(&denied.receipt.value).expect("denied text").contains("denied with decision"));

        let mismatch_store = root.join("mismatch-upgrades");
        let gate_ref =
            crate::ledger::import_artifact(&ledger_root, &gate.value).expect("import pass gate").artifact_ref;
        let wrong_protocol_ref = test_ref("wrong-protocol");
        let mismatch_plan =
            protocol_drain_plan_value(&gate_ref, &wrong_protocol_ref, &new_protocol_ref).expect("mismatch plan");
        let mismatch_created = create_session(&mismatch_store, &mismatch_plan).expect("create mismatch session");
        let mismatch = execute_task(&mismatch_store, &ledger_root, &mismatch_created.plan.plan_ref, "drain-sessions")
            .expect("execute mismatch drain");
        assert_eq!(mismatch.receipt.decision, "deny");
        assert!(to_text(&mismatch.receipt.value).expect("mismatch text").contains("expected one of"));

        let stale_store = root.join("stale-compat-upgrades");
        let mut stale_input = protocol_drain_plan_input(&gate_ref, &gate.protocol_ref, &new_protocol_ref);
        stale_input.compatibility.old_refs = vec![test_ref("stale-protocol-v1")];
        let stale_plan = upgrade_plan_value(&stale_input).expect("stale compatibility plan");
        let stale_created = create_session(&stale_store, &stale_plan).expect("create stale session");
        let stale = execute_task(&stale_store, &ledger_root, &stale_created.plan.plan_ref, "drain-sessions")
            .expect("execute stale drain");
        assert_eq!(stale.receipt.decision, "deny");
        assert!(to_text(&stale.receipt.value).expect("stale text").contains("stale compatibility ref"));

        let empty_terminal_store = root.join("empty-terminal-upgrades");
        let empty_terminal_gate = protocol_drain_gate_with_terminal_state_refs(&gate, Vec::new());
        let empty_terminal_gate_ref = crate::ledger::import_artifact(&ledger_root, &empty_terminal_gate)
            .expect("import empty terminal gate")
            .artifact_ref;
        let empty_terminal_plan =
            protocol_drain_plan_value(&empty_terminal_gate_ref, &gate.protocol_ref, &new_protocol_ref)
                .expect("empty terminal plan");
        let empty_terminal_created =
            create_session(&empty_terminal_store, &empty_terminal_plan).expect("create empty terminal session");
        let empty_terminal = execute_task(
            &empty_terminal_store,
            &ledger_root,
            &empty_terminal_created.plan.plan_ref,
            "drain-sessions",
        )
        .expect("execute empty terminal drain");
        assert_eq!(empty_terminal.receipt.decision, "deny");
        assert!(
            to_text(&empty_terminal.receipt.value)
                .expect("empty terminal text")
                .contains("does not bind terminal session state")
        );
    }

    #[test]
    fn denied_protocol_drain_and_cutover_preserve_pre_cutover_state() {
        let root = temp_dir("upgrade-protocol-no-mutation");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let gate = protocol_drain_gate();
        let new_protocol_ref = test_ref("protocol-v2");
        let missing_gate_ref = test_ref("missing-protocol-gate");
        let plan_value =
            protocol_drain_cutover_plan_value(&missing_gate_ref, &gate.protocol_ref, &new_protocol_ref)
                .expect("cutover drain plan");
        let created = create_session(&store, &plan_value).expect("create session");
        set_name_pointer(&store, "request-response-protocol", &gate.protocol_ref).expect("initial routing pointer");
        let transcript =
            execute_task(&store, &ledger_root, &created.plan.plan_ref, "transcript-gate").expect("transcript pass");
        assert_eq!(transcript.receipt.decision, "pass");

        let before_drain = upgrade_state_snapshot_ref(&store).expect("before drain snapshot");
        let denied_drain = execute_task(&store, &ledger_root, &created.plan.plan_ref, "drain-sessions")
            .expect("execute denied drain");
        let after_drain = upgrade_state_snapshot_ref(&store).expect("after drain snapshot");
        assert_eq!(denied_drain.receipt.decision, "deny");
        assert_eq!(before_drain, after_drain);
        assert!(to_text(&denied_drain.receipt.value).expect("drain text").contains("no-mutation-on-deny"));

        let before_cutover = upgrade_state_snapshot_ref(&store).expect("before cutover snapshot");
        let denied_cutover =
            execute_task(&store, &ledger_root, &created.plan.plan_ref, "cutover").expect("cutover denied");
        let after_cutover = upgrade_state_snapshot_ref(&store).expect("after cutover snapshot");
        assert_eq!(denied_cutover.receipt.decision, "deny");
        assert_eq!(before_cutover, after_cutover);
        assert!(
            to_text(&denied_cutover.receipt.value)
                .expect("cutover text")
                .contains("no-mutation-on-deny")
        );
        let pointer = read_name_pointer(&store, "request-response-protocol")
            .expect("read pointer")
            .expect("pointer exists");
        assert_eq!(pointer.artifact_ref, gate.protocol_ref);
    }

    #[test]
    fn cleanup_passes_only_without_active_references() {
        let root = temp_dir("upgrade-cleanup");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let artifact =
            crate::ledger::import_artifact(&ledger_root, &parse_text("<module \"unused\">").expect("artifact"))
                .expect("import artifact")
                .artifact_ref;
        let pass = cleanup_admission(&store, &ledger_root, &artifact).expect("cleanup pass");
        assert_eq!(pass.decision, "pass");
        set_name_pointer(&store, "unused", &artifact).expect("pin by name");
        let deny = cleanup_admission(&store, &ledger_root, &artifact).expect("cleanup deny");
        assert_eq!(deny.decision, "deny");
    }
