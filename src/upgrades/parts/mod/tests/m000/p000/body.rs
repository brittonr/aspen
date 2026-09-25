    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;

    type TestCase = hegel::TestCase;

    use super::*;

    #[test]
    fn name_move_session_keeps_artifacts_immutable_and_receipted() {
        let root = temp_dir("upgrade-name-move");
        let ledger_root = root.join("ledger");
        let store = root.join("upgrades");
        let old = crate::ledger::import_artifact(&ledger_root, &parse_text("<module \"old\">").expect("old artifact"))
            .expect("import old")
            .artifact_ref;
        let new = crate::ledger::import_artifact(&ledger_root, &parse_text("<module \"new\">").expect("new artifact"))
            .expect("import new")
            .artifact_ref;
        let dependent =
            crate::ledger::import_artifact(&ledger_root, &record("dependent", vec![string(&old), string("uses old")]))
                .expect("import dependent")
                .artifact_ref;
        let plan_value = name_move_plan_value(&ledger_root, &NameMovePlanInput {
            session_id: "session-name-move".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new.clone(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("plan value");
        let plan = parse_upgrade_plan(&plan_value).expect("parse plan");
        assert!(plan.impact_refs.contains(&old));
        assert!(plan.impact_refs.contains(&dependent));
        let created = create_session(&store, &plan_value).expect("create session");
        assert_eq!(created.receipt.decision, "pass");
        set_name_pointer(&store, "app/main", &old).expect("initial name pointer");
        for task_id in ["compatibility-alias", "transcript-gate", "move-name", "cutover"] {
            let executed = execute_task(&store, &ledger_root, &created.plan.plan_ref, task_id).expect("execute task");
            assert_eq!(executed.receipt.decision, "pass", "{task_id}");
        }
        let pointer = read_name_pointer(&store, "app/main").expect("read pointer").expect("pointer exists");
        assert_eq!(pointer.artifact_ref, new);
        let status = status(&store, &created.plan.plan_ref).expect("status");
        assert!(status.remaining_task_ids.is_empty());
        let cleanup_old = cleanup_admission(&store, &ledger_root, &old).expect("cleanup old");
        assert_eq!(cleanup_old.decision, "deny");
    }

    #[test]
    fn registry_backed_name_move_impact_uses_reverse_dependencies() {
        let root = temp_dir("upgrade-registry-impact");
        let registry_root = root.join("registry");
        let ledger_root = root.join("ledger");
        let old = crate::artifacts::install_artifact(&registry_root, &artifact_input("schema", "old", &[]))
            .expect("install old")
            .artifact_ref;
        let dependent = crate::artifacts::install_artifact(
            &registry_root,
            &artifact_input("steel", "dependent", std::slice::from_ref(&old)),
        )
        .expect("install dependent")
        .artifact_ref;
        let new = crate::artifacts::install_artifact(&registry_root, &artifact_input("schema", "new", &[]))
            .expect("install new")
            .artifact_ref;
        let plan_value = name_move_plan_value_with_registry(Some(&registry_root), &ledger_root, &NameMovePlanInput {
            session_id: "session-registry-impact".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new,
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("transcript-pass")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("registry impact plan");
        let plan = parse_upgrade_plan(&plan_value).expect("parse plan");
        assert!(plan.impact_refs.contains(&old));
        assert!(plan.impact_refs.contains(&dependent));
    }

    #[test]
    fn rollback_denies_irreversible_storage_migration_claims() {
        let root = temp_dir("upgrade-rollback");
        let store = root.join("upgrades");
        let source_schema = test_ref("schema-v1");
        let recipe = test_ref("migration-recipe");
        let plan_value = upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-storage-migration".to_string(),
            reason: "storage migration".to_string(),
            summary: "migrate durable records".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![source_schema.clone(), recipe.clone()],
            impact_refs: vec![source_schema.clone()],
            tasks: vec![UpgradeTaskInput {
                task_id: "migrate".to_string(),
                kind: "migrate-storage".to_string(),
                subject: "profiles".to_string(),
                from_ref: Some(source_schema),
                to_ref: Some(recipe),
                precondition_refs: vec![test_ref("storage-migration-policy")],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("schema-v1-old")],
                new_refs: vec![test_ref("schema-v2-new")],
                expires_at: Some(10),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: Vec::new(),
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("migration-review")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("plan value");
        let created = create_session(&store, &plan_value).expect("create session");
        let rollback = rollback_task(&store, &created.plan.plan_ref, "migrate").expect("rollback denied receipt");
        assert_eq!(rollback.decision, "deny");
        assert!(to_text(&rollback.value).expect("receipt text").contains("not reversible"));
    }

    #[test]
    fn structured_session_artifacts_cover_supported_surfaces_and_non_claims() {
        let old = test_ref("surface-old");
        let new = test_ref("surface-new");
        let plan_value = upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-structured-surfaces".to_string(),
            reason: "structured surfaces".to_string(),
            summary: "coordinate artifact schema policy handler transcript and cleanup rails".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old.clone(), new.clone()],
            impact_refs: vec![old.clone()],
            tasks: vec![
                surface_task("replace-artifact", "replace", &old, &new),
                surface_task("migrate-schema", "schema", &old, &new),
                surface_task("update-policy", "policy", &old, &new),
                surface_task("update-handler-profile", "handler", &old, &new),
                UpgradeTaskInput {
                    task_id: "cleanup".to_string(),
                    kind: "cleanup".to_string(),
                    subject: "cleanup".to_string(),
                    from_ref: Some(old.clone()),
                    to_ref: None,
                    precondition_refs: vec![test_ref("retention-evidence")],
                    postcondition_refs: vec![test_ref("impact-evidence")],
                    reversible: false,
                },
            ],
            compatibility: UpgradeCompatibilityWindow {
                old_refs: vec![old.clone()],
                new_refs: vec![new],
                expires_at: None,
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![old],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("review-evidence")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect("structured surface plan");
        let plan = parse_upgrade_plan(&plan_value).expect("parse structured plan");
        assert!(plan.checks.contains(&"structured-session-surfaces".to_string()));
        assert!(plan.checks.contains(&"external-workflows-not-replaced".to_string()));

        assert_source_control_replacement_claim_denied();
    }

    /// A plan that claims to replace source control or human review is denied.
    fn assert_source_control_replacement_claim_denied() {
        let denied = upgrade_plan_value(&UpgradePlanInput {
            session_id: "session-bad-claim".to_string(),
            reason: "compatible with UCM".to_string(),
            summary: "replace git and human review".to_string(),
            initiator_ref: test_ref("initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![test_ref("old")],
            impact_refs: vec![test_ref("old")],
            tasks: vec![UpgradeTaskInput {
                task_id: "transcript".to_string(),
                kind: "transcript-rerun".to_string(),
                subject: "transcript".to_string(),
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
            evidence_refs: vec![test_ref("review-evidence")],
            source_gate_receipt_values: source_gate_values(),
        })
        .expect_err("UCM/source-control replacement claim denied");
        assert!(denied.to_string().contains("UCM compatibility"), "{denied}");
    }

    #[test]
    fn task_status_requires_matching_receipt_not_checkbox_metadata() {
        let root = temp_dir("upgrade-status-receipt-backed");
        let store = root.join("upgrades");
        let old = test_ref("old");
        let new = test_ref("new");
        let plan_value = cutover_plan_value(&old, &new, vec![transcript_task_input("transcript", true)]);
        let created = create_session(&store, &plan_value).expect("create session");
        let fake_status_path = status_path(&store, &created.plan.session_id, "transcript").expect("status path");
        if let Some(parent) = fake_status_path.parent() {
            fs::create_dir_all(parent).expect("status parent");
        }
        fs::write(fake_status_path, test_ref("fake-checkbox-receipt")).expect("write fake status");
        let cutover = execute_task(&store, &root.join("ledger"), &created.plan.plan_ref, "cutover")
            .expect("cutover denied for fake status");
        assert_eq!(cutover.receipt.decision, "deny");
        assert!(to_text(&cutover.receipt.value).expect("cutover text").contains("transcript"));
    }

    #[test]
    fn cutover_denies_failed_replay_and_incomplete_migration_receipts() {
        let root = temp_dir("upgrade-cutover-denials");
        let failed_store = root.join("failed-replay");
        let old = test_ref("old");
        let new = test_ref("new");
        let failed_plan = cutover_plan_value(&old, &new, vec![transcript_task_input("transcript", false)]);
        let failed_created = create_session(&failed_store, &failed_plan).expect("create failed replay plan");
        let replay = execute_task(&failed_store, &root.join("ledger"), &failed_created.plan.plan_ref, "transcript")
            .expect("execute failed replay");
        assert_eq!(replay.receipt.decision, "deny");
        assert!(to_text(&replay.receipt.value).expect("replay text").contains("transcript-evidence"));
        let cutover = execute_task(&failed_store, &root.join("ledger"), &failed_created.plan.plan_ref, "cutover")
            .expect("cutover denied after failed replay");
        assert_eq!(cutover.receipt.decision, "deny");

        let migration_store = root.join("incomplete-migration");
        let migration_plan = cutover_plan_value(&old, &new, vec![
            transcript_task_input("transcript", true),
            UpgradeTaskInput {
                task_id: "migrate".to_string(),
                kind: "migrate-storage".to_string(),
                subject: "profiles".to_string(),
                from_ref: Some(old.clone()),
                to_ref: Some(test_ref("migration-recipe")),
                precondition_refs: vec![test_ref("migration-policy")],
                postcondition_refs: Vec::new(),
                reversible: false,
            },
        ]);
        let migration_created = create_session(&migration_store, &migration_plan).expect("create migration plan");
        execute_task(&migration_store, &root.join("ledger"), &migration_created.plan.plan_ref, "transcript")
            .expect("execute transcript");
        let migrate = execute_task(&migration_store, &root.join("ledger"), &migration_created.plan.plan_ref, "migrate")
            .expect("execute incomplete migration");
        assert_eq!(migrate.receipt.decision, "deny");
        assert!(to_text(&migrate.receipt.value).expect("migration text").contains("migration receipt"));
        let cutover = execute_task(&migration_store, &root.join("ledger"), &migration_created.plan.plan_ref, "cutover")
            .expect("cutover denied after incomplete migration");
        assert_eq!(cutover.receipt.decision, "deny");
    }
