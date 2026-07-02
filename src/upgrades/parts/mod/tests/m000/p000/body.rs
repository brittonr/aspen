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
