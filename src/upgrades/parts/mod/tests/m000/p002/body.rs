
    #[test]
    fn cleanup_task_requires_retention_and_dependency_impact_evidence() {
        let root = temp_dir("upgrade-cleanup-evidence");
        let store = UpgradeStore::open(&root.join("upgrades")).expect("upgrade store");
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
        let store = UpgradeStore::open(&root.join("upgrades")).expect("upgrade store");
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
        let missing_store = UpgradeStore::open(&root.join("missing-upgrades")).expect("upgrade store");
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

        let denied_store = UpgradeStore::open(&root.join("denied-upgrades")).expect("upgrade store");
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

        let mismatch_store = UpgradeStore::open(&root.join("mismatch-upgrades")).expect("upgrade store");
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

        let stale_store = UpgradeStore::open(&root.join("stale-compat-upgrades")).expect("upgrade store");
        let mut stale_input = protocol_drain_plan_input(&gate_ref, &gate.protocol_ref, &new_protocol_ref);
        stale_input.compatibility.old_refs = vec![test_ref("stale-protocol-v1")];
        let stale_plan = upgrade_plan_value(&stale_input).expect("stale compatibility plan");
        let stale_created = create_session(&stale_store, &stale_plan).expect("create stale session");
        let stale = execute_task(&stale_store, &ledger_root, &stale_created.plan.plan_ref, "drain-sessions")
            .expect("execute stale drain");
        assert_eq!(stale.receipt.decision, "deny");
        assert!(to_text(&stale.receipt.value).expect("stale text").contains("stale compatibility ref"));

        let empty_terminal_store = UpgradeStore::open(&root.join("empty-terminal-upgrades")).expect("upgrade store");
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
        let store = UpgradeStore::open(&root.join("upgrades")).expect("upgrade store");
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
        let store = UpgradeStore::open(&root.join("upgrades")).expect("upgrade store");
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

    fn name_move_fixture(root: &std::path::Path) -> (PathBuf, IoValue) {
        let ledger_root = root.join("ledger");
        let old = crate::ledger::import_artifact(&ledger_root, &parse_text("<module \"old\">").expect("old artifact"))
            .expect("import old")
            .artifact_ref;
        let new = crate::ledger::import_artifact(&ledger_root, &parse_text("<module \"new\">").expect("new artifact"))
            .expect("import new")
            .artifact_ref;
        let plan_value = name_move_plan_value(&ledger_root, &NameMovePlanInput {
            session_id: "session-capability-store".to_string(),
            name: "app/main".to_string(),
            from_ref: old,
            to_ref: new,
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("plan value");
        (ledger_root, plan_value)
    }

    // r[verify molten.upgrades.capability_rooted_store]
    #[test]
    fn content_addressed_members_are_published_once_and_never_rewritten() {
        let root = temp_dir("upgrade-immutable-members");
        let store_dir = root.join("upgrades");
        let store = UpgradeStore::open(&store_dir).expect("upgrade store");
        let (_ledger_root, plan_value) = name_move_fixture(&root);
        let created = create_session(&store, &plan_value).expect("create session");
        create_session(&store, &plan_value).expect("identical plan republishes idempotently");
        let plan_file = store_dir.join(plan_path(&created.plan.plan_ref).expect("plan path").as_path());
        std::fs::write(&plan_file, "<tampered>").expect("tamper with stored plan");
        let conflict = create_session(&store, &plan_value).expect_err("a different stored plan is a conflict");
        assert!(conflict.to_string().contains("already holds different content"), "{conflict}");
        assert_eq!(std::fs::read_to_string(&plan_file).expect("stored plan"), "<tampered>");
    }

    // r[verify molten.upgrades.capability_rooted_store]
    #[test]
    fn pointer_replacement_is_whole_and_leaves_no_stage() {
        let root = temp_dir("upgrade-pointer-replacement");
        let store_dir = root.join("upgrades");
        let store = UpgradeStore::open(&store_dir).expect("upgrade store");
        set_name_pointer(&store, "app/main", &test_ref("first")).expect("first pointer");
        set_name_pointer(&store, "app/main", &test_ref("second")).expect("second pointer");
        let pointer = read_name_pointer(&store, "app/main").expect("read pointer").expect("pointer exists");
        assert_eq!(pointer.artifact_ref, test_ref("second"));
        assert_eq!(pointer.previous_ref.as_deref(), Some(test_ref("first").as_str()));
        let names = std::fs::read_dir(store_dir.join("names")).expect("names directory");
        for entry in names {
            let name = entry.expect("name entry").file_name().to_string_lossy().into_owned();
            assert!(!name.starts_with('.'), "no publication stage remains: {name}");
        }
        let receipt = read_stored_receipt(&store, &pointer.receipt_ref).expect("pointer receipt is stored");
        assert_eq!(receipt.operation, "name-pointer-set");
    }

    // r[verify molten.upgrades.capability_rooted_store]
    #[test]
    fn interrupted_publication_stages_are_invisible_to_store_scans() {
        let root = temp_dir("upgrade-stage-scan");
        let store_dir = root.join("upgrades");
        let store = UpgradeStore::open(&store_dir).expect("upgrade store");
        let (_ledger_root, plan_value) = name_move_fixture(&root);
        create_session(&store, &plan_value).expect("create session");
        let before = upgrade_state_snapshot_ref(&store).expect("snapshot before");
        std::fs::write(store_dir.join("names").join(".blake3_00.preserves.stage-1-0"), "<partial")
            .expect("plant an interrupted stage");
        assert_eq!(upgrade_state_snapshot_ref(&store).expect("snapshot after"), before);
        assert!(read_name_pointers(&store).expect("pointers").is_empty());
    }

    // r[verify molten.upgrades.capability_rooted_store]
    #[cfg(unix)]
    #[test]
    fn links_inside_the_store_are_refused_for_reads_scans_and_writes() {
        let root = temp_dir("upgrade-store-links");
        let outside = root.join("outside");
        std::fs::create_dir_all(&outside).expect("outside directory");
        let store_dir = root.join("upgrades");
        let store = UpgradeStore::open(&store_dir).expect("upgrade store");
        set_name_pointer(&store, "app/main", &test_ref("first")).expect("first pointer");
        let leaf = store_dir.join(name_pointer_path("app/main").expect("pointer path").as_path());
        std::fs::remove_file(&leaf).expect("remove pointer leaf");
        std::os::unix::fs::symlink(outside.join("pointer.preserves"), &leaf).expect("plant leaf link");
        assert!(set_name_pointer(&store, "app/main", &test_ref("second")).is_err(), "a linked leaf is not replaced");
        assert!(!outside.join("pointer.preserves").exists(), "nothing is written through the link");
        assert!(read_name_pointer(&store, "app/main").is_err(), "a linked leaf is not read");
        assert!(read_name_pointers(&store).is_err(), "a scan refuses the link");

        std::fs::remove_file(&leaf).expect("remove leaf link");
        let (ledger_root, plan_value) = name_move_fixture(&root);
        let created = create_session(&store, &plan_value).expect("create session");
        std::fs::remove_dir_all(store_dir.join("status")).expect("remove status directory");
        std::os::unix::fs::symlink(&outside, store_dir.join("status")).expect("plant directory link");
        assert!(
            execute_task(&store, &ledger_root, &created.plan.plan_ref, "compatibility-alias").is_err(),
            "status is not published through a linked directory"
        );
        assert!(std::fs::read_dir(&outside).expect("outside entries").next().is_none(), "outside stays empty");
    }

    // r[verify molten.preserves_rail_toolkit.check_sets]
    #[test]
    fn a_denied_task_receipt_with_every_label_passing_never_completes_the_task() {
        let root = temp_dir("upgrade-labels-do-not-override-denial");
        let store = UpgradeStore::open(&root.join("upgrades")).expect("upgrade store");
        let old = test_ref("old");
        let new = test_ref("new");
        let plan_value = cutover_plan_value(&old, &new, vec![transcript_task_input("transcript", true)]);
        let created = create_session(&store, &plan_value).expect("create session");
        let denied = upgrade_receipt_value(&UpgradeReceiptValueInput {
            operation: "task-complete",
            decision: "deny",
            session_id: &created.plan.session_id,
            plan_ref: &created.plan.plan_ref,
            task_id: Some("transcript"),
            refs: &[],
            diagnostics: &["transcript replay failed".to_string()],
            checks: &[("transcript-replay", "pass"), ("no-mutation-on-deny", "pass")],
        })
        .expect("denied receipt with passing labels");
        let receipt = parse_upgrade_receipt(&denied).expect("labels satisfy the receipt shape");
        store_receipt(&store, &denied).expect("store denied receipt");
        let status_locator = status_path(&created.plan.session_id, "transcript").expect("status path");
        store.root().publish(&status_locator, receipt.receipt_ref.as_bytes()).expect("point status at the receipt");
        let status = status(&store, &created.plan.plan_ref).expect("status");
        assert!(status.tasks.iter().any(|task| task.task_id == "transcript" && !task.done), "denial wins over labels");
        let cutover = execute_task(&store, &root.join("ledger"), &created.plan.plan_ref, "cutover").expect("cutover");
        assert_eq!(cutover.receipt.decision, "deny");
    }

