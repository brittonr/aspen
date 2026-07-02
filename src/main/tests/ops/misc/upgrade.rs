    #[test]
    fn cli_upgrade_session_commands_work() {
        let dir = temp_dir("upgrade-cli");
        let ledger_root = dir.join("ledger");
        let store = dir.join("upgrades");
        let artifacts = import_upgrade_artifacts(&ledger_root);
        let plan = create_upgrade_plan(&dir, &ledger_root, &store, &artifacts);
        run_upgrade_tasks(&dir, &ledger_root, &store, &plan);
        check_upgrade_result(&dir, ledger_root, store, artifacts.new);
    }

    struct UpgradeArtifacts {
        old: String,
        new: String,
    }

    struct UpgradePlanFile {
        plan: molten::upgrades::UpgradePlan,
    }

    fn import_upgrade_artifacts(ledger_root: &Path) -> UpgradeArtifacts {
        let old = molten::ledger::import_artifact(ledger_root, &record("cli-old-artifact", vec![string("old")]))
            .expect("import old")
            .artifact_ref;
        let new = molten::ledger::import_artifact(ledger_root, &record("cli-new-artifact", vec![string("new")]))
            .expect("import new")
            .artifact_ref;
        UpgradeArtifacts { old, new }
    }

    fn create_upgrade_plan(
        dir: &Path,
        ledger_root: &Path,
        store: &Path,
        artifacts: &UpgradeArtifacts,
    ) -> UpgradePlanFile {
        let plan_out = dir.join("upgrade-plan.preserves");
        let source_gate = write_source_gate(dir);
        run_upgrade_command(UpgradeCommand::PlanNameMove {
            ledger: ledger_root.to_path_buf(),
            registry: None,
            session_id: "cli-upgrade".to_string(),
            name: "app/main".to_string(),
            from_ref: artifacts.old.clone(),
            to_ref: artifacts.new.clone(),
            source_gate_receipts: vec![source_gate],
            out: plan_out.clone(),
        })
        .expect("plan name move");
        let plan_value = read_preserves_file(&plan_out).expect("read plan");
        let plan = molten::upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: plan_out,
            store: store.to_path_buf(),
            receipt_out: Some(dir.join("upgrade-create-receipt.preserves")),
        })
        .expect("create upgrade");
        run_upgrade_command(UpgradeCommand::SetName {
            store: store.to_path_buf(),
            name: "app/main".to_string(),
            artifact_ref: artifacts.old.clone(),
            receipt_out: Some(dir.join("upgrade-set-name-receipt.preserves")),
        })
        .expect("set initial name");
        UpgradePlanFile { plan }
    }

    fn write_source_gate(dir: &Path) -> PathBuf {
        let source_gate = dir.join("octet-gate-receipt.preserves");
        let source_gate_value = molten::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture");
        write_file(&source_gate, &to_text(&source_gate_value).expect("source gate text")).expect("write source gate");
        source_gate
    }

    fn run_upgrade_tasks(dir: &Path, ledger_root: &Path, store: &Path, plan: &UpgradePlanFile) {
        for task_id in ["compatibility-alias", "transcript-gate", "move-name", "cutover"] {
            run_upgrade_command(UpgradeCommand::RunTask {
                store: store.to_path_buf(),
                ledger: ledger_root.to_path_buf(),
                plan_ref: plan.plan.plan_ref.clone(),
                task_id: task_id.to_string(),
                receipt_out: Some(dir.join(format!("upgrade-{task_id}-receipt.preserves"))),
            })
            .expect("run upgrade task");
        }
        run_upgrade_command(UpgradeCommand::Status {
            store: store.to_path_buf(),
            plan_ref: plan.plan.plan_ref.clone(),
        })
        .expect("upgrade status");
    }

    fn check_upgrade_result(dir: &Path, ledger_root: PathBuf, store: PathBuf, expected_ref: String) {
        let pointer = molten::upgrades::read_name_pointer(&store, "app/main")
            .expect("read name pointer")
            .expect("name pointer exists");
        assert_eq!(pointer.artifact_ref, expected_ref);
        run_upgrade_command(UpgradeCommand::CleanupCheck {
            store,
            ledger: ledger_root,
            registry: None,
            artifact_ref: pointer.previous_ref.expect("previous ref"),
            receipt_out: Some(dir.join("upgrade-cleanup-receipt.preserves")),
        })
        .expect("cleanup check emits denial receipt");
    }
