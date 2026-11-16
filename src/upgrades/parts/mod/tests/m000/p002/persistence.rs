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