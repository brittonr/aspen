    #[test]
    fn cli_coordination_commands_work() {
        let dir = temp_dir("coordination-cli");
        let out = dir.join("coordination-fixture");
        run_coordination_command(CoordinationCommand::RunFixture { out: out.clone() }).expect("coordination fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read coordination report");
        assert!(
            to_text(&report_value)
                .expect("render coordination report")
                .contains("coordination-fixture-report-v1")
        );
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read coordination manifest");
        let parsed =
            coordination::parse_coordination_service_manifest(&manifest_value).expect("parse coordination manifest");
        assert_eq!(parsed.service_id, "coordination:local");
        run_coordination_command(CoordinationCommand::Show { artifact: manifest }).expect("coordination show manifest");

        let policy_ref = cli_synthetic_ref("coordination-cli-policy").expect("policy ref");
        let resource_ref = cli_synthetic_ref("coordination-cli-resource").expect("resource ref");
        let authority_ref = cli_synthetic_ref("coordination-cli-authority").expect("authority ref");
        let operation_id_ref = cli_synthetic_ref("coordination-cli-operation").expect("operation ref");
        let generated_manifest = dir.join("coordination.manifest.preserves");
        run_coordination_command(CoordinationCommand::Manifest {
            service_id: "coordination:local".to_string(),
            services: vec![coordination::SERVICE_QUEUE.to_string()],
            control_group_ref: None,
            queue_capacity: 2,
            semaphore_capacity: coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
            rate_limit: coordination::DEFAULT_COORDINATION_RATE_LIMIT,
            barrier_parties: coordination::DEFAULT_COORDINATION_BARRIER_PARTIES,
            policy_refs: vec![policy_ref.clone()],
            resource_refs: vec![resource_ref.clone()],
            out: Some(generated_manifest.clone()),
        })
        .expect("coordination manifest");
        let generated_manifest_value = read_preserves_file(&generated_manifest).expect("read generated manifest");
        let generated_manifest_parsed = coordination::parse_coordination_service_manifest(&generated_manifest_value)
            .expect("parse generated coordination manifest");
        assert_eq!(generated_manifest_parsed.services, vec![coordination::SERVICE_QUEUE.to_string()]);

        let payload = dir.join("queue-item.preserves");
        write_file(&payload, r#"<item "cli-one">"#).expect("write queue payload");
        let request = dir.join("coordination.request.preserves");
        run_coordination_command(CoordinationCommand::Request {
            service: coordination::SERVICE_QUEUE.to_string(),
            operation: coordination::OP_ENQUEUE.to_string(),
            key: "queue:cli".to_string(),
            client_session: "client-cli".to_string(),
            operation_id_ref,
            payload: Some(payload),
            authority_refs: vec![authority_ref],
            resource_refs: vec![resource_ref],
            policy_refs: vec![policy_ref],
            out: Some(request.clone()),
        })
        .expect("coordination request");
        run_coordination_command(CoordinationCommand::Show {
            artifact: request.clone(),
        })
        .expect("show request");

        let apply_out = dir.join("coordination-apply");
        run_coordination_command(CoordinationCommand::Apply {
            manifest: generated_manifest,
            requests: vec![request.clone(), request],
            out: apply_out.clone(),
        })
        .expect("coordination apply");
        let apply_report = read_preserves_file(&apply_out.join("report.preserves")).expect("read apply report");
        let parsed_report = coordination::parse_coordination_apply_report(&apply_report).expect("parse apply report");
        assert_eq!(parsed_report.decision, "pass");
        assert_eq!(parsed_report.receipt_refs.len(), 2);
        assert_eq!(parsed_report.receipt_refs[0], parsed_report.receipt_refs[1]);
        assert_eq!(parsed_report.assertion_refs.len(), 2);
        assert_eq!(parsed_report.assertion_refs[0], parsed_report.assertion_refs[1]);
        run_coordination_command(CoordinationCommand::Show {
            artifact: apply_out.join("report.preserves"),
        })
        .expect("coordination show apply report");
    }

    #[test]
    fn cli_secrets_commands_work() {
        let dir = temp_dir("secrets-cli");
        let out = dir.join("secrets-fixture");
        run_secrets_command(SecretsCommand::RunFixture { out: out.clone() }).expect("secrets fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read secrets report");
        let summary = secrets::fixture_report_summary(&report_value).expect("summary");
        assert!(summary.contains("plaintext=redacted"));
        let secret = out.join("secret.preserves");
        let secret_value = read_preserves_file(&secret).expect("read secret");
        let parsed = secrets::parse_secret_ref(&secret_value).expect("parse secret");
        assert_eq!(parsed.secret_id, "secret:fixture");
        run_secrets_command(SecretsCommand::Show { artifact: report }).expect("show report");
        run_secrets_command(SecretsCommand::Show { artifact: secret }).expect("show secret");
    }

    #[test]
    fn cli_plugin_lifecycle_commands_work() {
        let dir = temp_dir("plugin-cli");
        let state_root = dir.join("state");
        let out = dir.join("plugin-fixture");
        run_plugin_command(PluginCommand::RunFixture {
            state_root,
            out: out.clone(),
        })
        .expect("plugin fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read plugin report");
        assert!(to_text(&report_value).expect("render plugin report").contains("plugin-fixture-report-v1"));
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read plugin manifest");
        let parsed = plugin_host::parse_plugin_manifest(&manifest_value).expect("parse plugin manifest");
        assert_eq!(parsed.plugin_id, "plugin:minimal");
        run_plugin_command(PluginCommand::Show { artifact: report }).expect("plugin show report");
        run_plugin_command(PluginCommand::Show { artifact: manifest }).expect("plugin show manifest");
    }

    #[test]
    fn cli_schema_identity_commands_work() {
        let dir = temp_dir("schema-cli");
        let registry = dir.join("registry");
        let shape_file = dir.join("shape.preserves");
        let expected_identity_out = dir.join("expected-identity.preserves");
        let actual_identity_out = dir.join("actual-identity.preserves");
        let alias_out = dir.join("alias.preserves");
        let compat_out = dir.join("compat.preserves");
        let shape = r#"<shape "record" "profile" [<shape "field" "name" <shape "string">> <shape "field" "age" <shape "u64">>]>"#;
        write_file(&shape_file, shape).expect("write shape");
        let expected_schema_ref = test_ref("expected-schema-cli");
        let actual_schema_ref = test_ref("actual-schema-cli");
        run_schema_command(SchemaCommand::Identity {
            shape: shape_file.clone(),
            schema_ref: expected_schema_ref.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: expected_identity_out.clone(),
            receipt_out: Some(dir.join("expected-identity-receipt.preserves")),
        })
        .expect("schema expected identity");
        run_schema_command(SchemaCommand::Identity {
            shape: shape_file,
            schema_ref: actual_schema_ref.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: actual_identity_out.clone(),
            receipt_out: Some(dir.join("actual-identity-receipt.preserves")),
        })
        .expect("schema actual identity");
        run_schema_command(SchemaCommand::Alias {
            from_ref: actual_schema_ref,
            to_ref: expected_schema_ref,
            scope: "storage".to_string(),
            out: alias_out.clone(),
            receipt_out: Some(dir.join("alias-receipt.preserves")),
        })
        .expect("schema alias");
        run_schema_command(SchemaCommand::Compat {
            expected_identity: expected_identity_out.clone(),
            actual_identity: actual_identity_out.clone(),
            alias: Some(alias_out),
            migration_ref: None,
            out: Some(compat_out.clone()),
            receipt_out: Some(dir.join("compat-receipt.preserves")),
        })
        .expect("schema compat");
        assert!(fs::read_to_string(&compat_out).expect("read compat").contains("schema-compatibility-v1"));
        let schema_artifact = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: record("schema-source", vec![string("cli")]),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema artifact");
        let identity_value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
            mode: "structural".to_string(),
            schema_ref: schema_artifact.artifact_ref.clone(),
            shape: parse_text(shape).expect("parse shape"),
            brand_ref: None,
            metadata_refs: vec![test_ref("metadata")],
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("identity value");
        let identity = schema_identity::parse_schema_identity(&identity_value).expect("parse identity");
        artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity_value,
            schema_refs: vec![schema_artifact.artifact_ref.clone()],
            dependency_refs: vec![schema_artifact.artifact_ref],
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema identity artifact");
        run_schema_command(SchemaCommand::SearchFingerprint {
            registry,
            fingerprint: identity.structural_fingerprint,
        })
        .expect("schema search fingerprint");
    }

    #[test]
    fn cli_upgrade_session_commands_work() {
        let dir = temp_dir("upgrade-cli");
        let ledger_root = dir.join("ledger");
        let store = dir.join("upgrades");
        let old = ledger::import_artifact(&ledger_root, &record("cli-old-artifact", vec![string("old")]))
            .expect("import old")
            .artifact_ref;
        let new = ledger::import_artifact(&ledger_root, &record("cli-new-artifact", vec![string("new")]))
            .expect("import new")
            .artifact_ref;
        let plan_out = dir.join("upgrade-plan.preserves");
        let source_gate = dir.join("octet-gate-receipt.preserves");
        write_file(
            &source_gate,
            &to_text(&octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"))
                .expect("source gate text"),
        )
        .expect("write source gate");
        run_upgrade_command(UpgradeCommand::PlanNameMove {
            ledger: ledger_root.clone(),
            registry: None,
            session_id: "cli-upgrade".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new.clone(),
            source_gate_receipts: vec![source_gate],
            out: plan_out.clone(),
        })
        .expect("plan name move");
        let plan_value = read_preserves_file(&plan_out).expect("read plan");
        let plan = upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: plan_out,
            store: store.clone(),
            receipt_out: Some(dir.join("upgrade-create-receipt.preserves")),
        })
        .expect("create upgrade");
        run_upgrade_command(UpgradeCommand::SetName {
            store: store.clone(),
            name: "app/main".to_string(),
            artifact_ref: old,
            receipt_out: Some(dir.join("upgrade-set-name-receipt.preserves")),
        })
        .expect("set initial name");
        for task_id in ["compatibility-alias", "transcript-gate", "move-name", "cutover"] {
            run_upgrade_command(UpgradeCommand::RunTask {
                store: store.clone(),
                ledger: ledger_root.clone(),
                plan_ref: plan.plan_ref.clone(),
                task_id: task_id.to_string(),
                receipt_out: Some(dir.join(format!("upgrade-{task_id}-receipt.preserves"))),
            })
            .expect("run upgrade task");
        }
        run_upgrade_command(UpgradeCommand::Status {
            store: store.clone(),
            plan_ref: plan.plan_ref.clone(),
        })
        .expect("upgrade status");
        let pointer = upgrades::read_name_pointer(&store, "app/main")
            .expect("read name pointer")
            .expect("name pointer exists");
        assert_eq!(pointer.artifact_ref, new);
        run_upgrade_command(UpgradeCommand::CleanupCheck {
            store,
            ledger: ledger_root,
            registry: None,
            artifact_ref: pointer.previous_ref.expect("previous ref"),
            receipt_out: Some(dir.join("upgrade-cleanup-receipt.preserves")),
        })
        .expect("cleanup check emits denial receipt");
    }
