    use super::*;

    #[test]
    fn roundtrip_schema_tagged_preserves_value() {
        let root = temp_dir("typed-storage-roundtrip");
        let value = crate::preserves_rail::parse_text("<profile \"alice\" 7>").expect("parse value");
        let producer_ref = test_ref("producer");
        let put = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: producer_ref.clone(),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("roundtrip"),
        })
        .expect("put value");
        let get = get_value(&root, "profiles", "alice", Some(&put.schema_ref), &Admission::local_fixture("roundtrip"))
            .expect("get value");
        assert_eq!(get.value, value);
        assert_eq!(get.storage_ref, put.storage_ref);
        let verify = verify_ref(&root, &put.storage_ref, Some(&put.schema_ref)).expect("verify ref");
        assert_eq!(verify.storage_ref, put.storage_ref);
        let receipt_refs = list_receipt_refs(&root).expect("receipt refs");
        assert!(receipt_refs.len() >= 3);
        assert!(
            parse_receipt_value(&put.receipt_value, None)
                .expect("parse put receipt")
                .checks
                .contains(&"write-admission".to_string())
        );
    }

    #[test]
    fn schema_mismatch_is_denied_before_write_or_load() {
        let root = temp_dir("typed-storage-schema-mismatch");
        let value = crate::preserves_rail::parse_text("\"alice\"").expect("parse value");
        let wrong_schema_ref = test_ref("wrong-schema");
        let error = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(wrong_schema_ref),
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("schema-mismatch"),
        })
        .expect_err("wrong write schema denied");
        assert!(error.to_string().contains("schema ref"));
        assert_eq!(list_receipt_refs(&root).expect("receipt refs").len(), 1);

        let put = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("schema-mismatch"),
        })
        .expect("put inferred");
        let load_error = get_value(
            &root,
            "profiles",
            "alice",
            Some(&test_ref("other-schema")),
            &Admission::local_fixture("schema-mismatch"),
        )
        .expect_err("wrong load schema denied");
        assert!(load_error.to_string().contains("schema ref"));
        assert!(verify_ref(&root, &put.storage_ref, Some(&put.schema_ref)).is_ok());
    }

    #[test]
    fn explicit_and_lazy_migrations_preserve_value_hash_and_trace_refs() {
        let root = temp_dir("typed-storage-migration");
        let value = crate::preserves_rail::parse_text("<profile \"alice\" 7>").expect("parse value");
        let admission = Admission::local_fixture("migration");
        let put = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let target_schema_ref = test_ref("profile-v2-schema");
        let recipe = migration_recipe_value(&MigrationRecipeInput {
            source_schema_ref: put.schema_ref.clone(),
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref("schema-rename-transformer"),
            transformer_kind: "schema-rename".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref("migration-policy")],
            evidence_refs: vec![test_ref("migration-evidence")],
        })
        .expect("recipe");
        let before_error = get_value(&root, "profiles", "alice", Some(&target_schema_ref), &admission)
            .expect_err("target schema rejected before migration");
        assert!(before_error.to_string().contains("schema ref"), "{before_error}");
        let migrated = migrate_value(&root, "profiles", "alice", &recipe, &admission).expect("migrate");
        assert_eq!(migrated.old_storage_ref, put.storage_ref);
        assert_eq!(migrated.old_value_ref, put.value_ref);
        assert_eq!(migrated.new_value_ref, put.value_ref);
        let parsed_receipt = parse_receipt_value(&migrated.receipt_value, None).expect("parse migrate receipt");
        assert!(parsed_receipt.checks.contains(&"migration-trace".to_string()));
        assert!(parsed_receipt.checks.contains(&"original-value-hash".to_string()));
        let loaded =
            get_value(&root, "profiles", "alice", Some(&target_schema_ref), &admission).expect("load migrated target");
        assert_eq!(loaded.value, value);
        assert_eq!(loaded.typed_ref.schema_ref, target_schema_ref);

        assert_lazy_load(&root, &admission);
    }

    fn assert_lazy_load(root: &Path, admission: &Admission) {
        let lazy_value = crate::preserves_rail::parse_text("<profile \"bob\" 9>").expect("parse lazy value");
        let lazy_put = put_value(root, &PutInput {
            namespace: "profiles".to_string(),
            key: "bob".to_string(),
            schema_ref: None,
            value: lazy_value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put lazy source");
        let lazy_target_schema_ref = test_ref("profile-v3-schema");
        let lazy_recipe = migration_recipe_value(&MigrationRecipeInput {
            source_schema_ref: lazy_put.schema_ref,
            target_schema_ref: lazy_target_schema_ref.clone(),
            transformer_ref: test_ref("lazy-transformer"),
            transformer_kind: "identity".to_string(),
            mode: "lazy-on-read".to_string(),
            policy_refs: vec![test_ref("lazy-policy")],
            evidence_refs: vec![test_ref("lazy-evidence")],
        })
        .expect("lazy recipe");
        let lazy_loaded = get_value_with_migration(MigrationGetInput {
            root,
            namespace: "profiles",
            key: "bob",
            expected_schema_ref: &lazy_target_schema_ref,
            migration_recipe_value: &lazy_recipe,
            admission,
        })
        .expect("lazy migration load");
        assert_eq!(lazy_loaded.value, lazy_value);
        assert_eq!(lazy_loaded.typed_ref.schema_ref, lazy_target_schema_ref);
    }

    #[test]
    fn migration_requires_matching_source_schema_and_admitted_recipe() {
        let root = temp_dir("typed-storage-migration-deny");
        let value = crate::preserves_rail::parse_text("\"alice\"").expect("parse value");
        let admission = Admission::local_fixture("migration-deny");
        let put = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let wrong_source_recipe = migration_recipe_value(&MigrationRecipeInput {
            source_schema_ref: test_ref("wrong-source"),
            target_schema_ref: test_ref("target"),
            transformer_ref: test_ref("transformer"),
            transformer_kind: "identity".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("wrong source recipe");
        let error = migrate_value(&root, "profiles", "alice", &wrong_source_recipe, &admission)
            .expect_err("wrong source denied");
        assert!(error.to_string().contains("source schema"), "{error}");

        let unsupported_transformer = record("storage-migration-recipe-v1", vec![
            string(crate::preserves_rail::TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA),
            record("source-schema-ref", vec![string(&put.schema_ref)]),
            record("target-schema-ref", vec![string(test_ref("target"))]),
            record("transformer", vec![string(test_ref("transformer")), string("ambient-script")]),
            record("mode", vec![string("explicit")]),
            refs_record("policy", &[test_ref("policy")]),
            refs_record("evidence", &[test_ref("evidence")]),
            checks_value(&[
                "migration-recipe-artifact",
                "source-schema-binding",
                "target-schema-binding",
                "transformer-artifact-binding",
                "policy-admission-required",
                "migration-trace-required",
            ]),
        ]);
        let error = parse_migration_recipe_value(&unsupported_transformer).expect_err("unsupported transformer denied");
        assert!(error.to_string().contains("transformer kind"), "{error}");
    }

    #[test]
    fn schema_identity_structural_alias_and_migration_available_integrate_with_loads() {
        let root = temp_dir("typed-storage-schema-identity");
        let value = crate::preserves_rail::parse_text("<profile \"alice\" 7>").expect("parse value");
        let admission = Admission::local_fixture("schema-identity");
        let put = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: admission.clone(),
        })
        .expect("put source");
        let shape = record("shape", vec![
            string("record"),
            string("profile"),
            sequence(vec![
                record("shape", vec![string("field"), string("name"), record("shape", vec![string("string")])]),
                record("shape", vec![string("field"), string("age"), record("shape", vec![string("u64")])]),
            ]),
        ]);
        let actual = parsed_id(
            crate::schema_identity::MODE_STRUCTURAL,
            put.schema_ref.clone(),
            shape.clone(),
            "actual-metadata",
            "actual identity",
        );
        let expected_schema_ref = test_ref("expected-structural-schema");
        let expected = parsed_id(
            crate::schema_identity::MODE_STRUCTURAL,
            expected_schema_ref.clone(),
            shape.clone(),
            "expected-metadata",
            "expected identity",
        );
        let compatibility = compatible(expected, actual, None, "structural compatibility");
        let loaded = get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root: &root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &expected_schema_ref,
            schema_compatibility_value: &compatibility,
            admission: &admission,
        })
        .expect("load through structural compatibility");
        assert_eq!(loaded.value, value);
        assert!(
            crate::preserves_rail::to_text(&loaded.receipt_value)
                .expect("receipt text")
                .contains("schema-compatibility-value")
        );

        assert_alternate_case(&root, &put, shape, &admission);
    }
