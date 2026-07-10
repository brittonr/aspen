
    fn assert_alternate_case(root: &Path, put: &Put, shape: IoValue, admission: &Admission) {
        let unique_expected_schema_ref = test_ref("unique-expected-schema");
        let actual = parsed_id(
            crate::schema_identity::MODE_UNIQUE,
            put.schema_ref.clone(),
            shape.clone(),
            "actual-unique",
            "actual unique identity",
        );
        let expected = parsed_id(
            crate::schema_identity::MODE_UNIQUE,
            unique_expected_schema_ref.clone(),
            shape,
            "expected-unique",
            "expected unique identity",
        );
        let mismatch = compatible(expected.clone(), actual.clone(), None, "unique mismatch");
        let error = get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &unique_expected_schema_ref,
            schema_compatibility_value: &mismatch,
            admission,
        })
        .expect_err("unique mismatch denied");
        assert!(error.to_string().contains("schema ref"), "{error}");

        let binding = parsed_binding(&put.schema_ref, &unique_expected_schema_ref);
        let admitted = compatible(expected, actual, Some(binding), "alias compatibility");
        get_value_with_schema_compatibility(SchemaCompatibilityGetInput {
            root,
            namespace: "profiles",
            key: "alice",
            expected_schema_ref: &unique_expected_schema_ref,
            schema_compatibility_value: &admitted,
            admission,
        })
        .expect("alias load admitted");
    }

    fn parsed_id(
        mode: &str,
        schema_ref: String,
        shape: IoValue,
        metadata_label: &str,
        context: &str,
    ) -> crate::schema_identity::Identity {
        let value = crate::schema_identity::identity_value(&crate::schema_identity::IdentityInput {
            mode: mode.to_string(),
            schema_ref,
            shape,
            brand_ref: None,
            metadata_refs: vec![test_ref(metadata_label)],
            policy_refs: vec![test_ref("schema-policy")],
            evidence_refs: vec![test_ref("schema-evidence")],
        })
        .expect(context);
        crate::schema_identity::parse_identity(&value).expect(context)
    }

    fn compatible(
        expected: crate::schema_identity::Identity,
        actual: crate::schema_identity::Identity,
        alias: Option<crate::schema_identity::Alias>,
        context: &str,
    ) -> IoValue {
        crate::schema_identity::compatibility_decision_value(&crate::schema_identity::CompatibilityInput {
            expected,
            actual,
            alias,
            migration_ref: None,
            policy_refs: vec![test_ref("compat-policy")],
            evidence_refs: vec![test_ref("compat-evidence")],
            deny_by_policy: false,
        })
        .expect(context)
    }

    fn parsed_binding(from_schema_ref: &str, to_schema_ref: &str) -> crate::schema_identity::Alias {
        let value = crate::schema_identity::alias_value(&crate::schema_identity::AliasInput {
            from_schema_ref: from_schema_ref.to_string(),
            to_schema_ref: to_schema_ref.to_string(),
            scope: "storage".to_string(),
            policy_refs: vec![test_ref("alias-policy")],
            evidence_refs: vec![test_ref("alias-evidence")],
        })
        .expect("alias");
        crate::schema_identity::parse_alias(&value).expect("parse alias")
    }

    #[test]
    fn large_values_use_chunk_backed_content_refs() {
        let root = temp_dir("typed-storage-large");
        let large = "x".repeat(INLINE_VALUE_LIMIT + 128);
        let value = IoValue::new(large.clone());
        let put = put_value(&root, &PutInput {
            namespace: "large".to_string(),
            key: "payload".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("large"),
        })
        .expect("put large");
        let parsed = parse_entry_ref_value(&put.typed_ref_value).expect("parse typed ref");
        let Payload::ContentRef { manifest_ref, .. } = &parsed.payload else {
            panic!("large typed storage value must use chunk manifest ref");
        };
        let manifest = crate::chunk_store::read_manifest(&chunk_root(&root), manifest_ref)
            .expect("read typed storage chunk manifest");
        assert_eq!(manifest.object_kind, "typed-storage-value");
        let get = get_value(&root, "large", "payload", Some(&put.schema_ref), &Admission::local_fixture("large"))
            .expect("get large");
        assert_eq!(get.value.as_string().expect("string").as_ref(), large);

        let chunk_hex = crate::preserves_rail::content_ref_hex(&manifest.chunks[0].chunk_ref).expect("chunk hex");
        std::fs::write(chunk_root(&root).join("chunks").join(format!("blake3_{chunk_hex}.bin")), b"tampered")
            .expect("tamper typed storage chunk");
        let error = get_value(&root, "large", "payload", Some(&put.schema_ref), &Admission::local_fixture("large"))
            .expect_err("tampered chunk denies before typed storage load");
        let message = error.to_string();
        let is_integrity_boundary_mentioned = message.contains("chunk") || message.contains("hash");
        assert!(is_integrity_boundary_mentioned, "{message}");
    }

    #[test]
    fn storage_admission_denies_missing_schema_unadmitted_decoder_and_function_payloads() {
        let missing_schema = typed_ref_fixture(
            record("schema-ref", vec![record("none", Vec::new())]),
            default_typed_ref_checks(),
            Vec::new(),
        );
        let error = parse_entry_ref_value(&missing_schema).expect_err("missing schema ref denied");
        assert!(error.to_string().contains("schema-ref"), "{error}");

        let decoder_ref = test_ref("decoder-artifact");
        let unadmitted_decoder = typed_ref_fixture(
            record("schema-ref", vec![string(test_ref("schema"))]),
            &[
                "typed-durable-ref",
                "schema-ref-binding",
                "schema-identity-binding",
                "value-ref-binding",
                "producer-artifact-binding",
                "retention-binding",
                "provenance-binding",
                "handle-not-authority",
            ],
            vec![decoder_ref],
        );
        let error = parse_entry_ref_value(&unadmitted_decoder).expect_err("unadmitted decoder denied");
        assert!(error.to_string().contains("decoder-artifact-admission"), "{error}");

        let root = temp_dir("typed-storage-function-deny");
        let function_payload = crate::preserves_rail::parse_text("<decoder <serialized-function \"opaque\">>")
            .expect("parse function payload");
        let error = put_value(&root, &PutInput {
            namespace: "profiles".to_string(),
            key: "decoder".to_string(),
            schema_ref: None,
            value: function_payload,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("function-deny"),
        })
        .expect_err("serialized function denied");
        assert!(error.to_string().contains("serialized function"), "{error}");
    }

    #[test]
    fn storage_refs_do_not_mint_authority_from_snapshots() {
        let root = temp_dir("typed-storage-authority");
        let value = crate::preserves_rail::parse_text("<snapshot [\"state\"]>").expect("parse snapshot");
        let put = put_value(&root, &PutInput {
            namespace: "snapshots".to_string(),
            key: "actor".to_string(),
            schema_ref: None,
            value,
            producer_ref: test_ref("producer"),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            admission: Admission::local_fixture("authority"),
        })
        .expect("put snapshot");
        let missing_authority = Admission {
            actor_ref: test_ref("actor"),
            capability_ref: "not-a-ref".to_string(),
            policy_ref: test_ref("policy"),
            resource_refs: vec![test_ref("resource")],
            evidence_refs: vec![test_ref("evidence")],
        };
        let error = get_value(&root, "snapshots", "actor", Some(&put.schema_ref), &missing_authority)
            .expect_err("bad authority denied");
        assert!(error.to_string().contains("capability"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_entry_ref_hashes_schema_and_revision_are_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("typed-storage-hegel");
        let value = IoValue::new(format!("value-{salt}"));
        let input = PutInput {
            namespace: format!("ns-{salt}"),
            key: format!("key-{salt}"),
            schema_ref: None,
            value: value.clone(),
            producer_ref: test_ref(&format!("producer-{salt}")),
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            admission: Admission::local_fixture(&format!("hegel-{salt}")),
        };
        let first = put_value(&root, &input).expect("first put");
        let first_ref = parse_entry_ref_value(&first.typed_ref_value).expect("first ref");
        assert_eq!(first.schema_ref, inferred_schema_ref(&value).expect("schema ref"));
        assert_eq!(first_ref.revision, 1);
        let loaded = get_value(&root, &input.namespace, &input.key, Some(&first.schema_ref), &input.admission)
            .expect("load first");
        assert_eq!(canonical_hash(&loaded.value).expect("loaded value ref"), first.value_ref);
        let second = put_value(&root, &input).expect("second put");
        let second_ref = parse_entry_ref_value(&second.typed_ref_value).expect("second ref");
        assert_eq!(second_ref.revision, 2);
        assert_ne!(first.storage_ref, second.storage_ref);
        assert_eq!(first.value_ref, second.value_ref);

        let target_schema_ref = test_ref(&format!("target-schema-{salt}"));
        let recipe = migration_recipe_value(&MigrationRecipeInput {
            source_schema_ref: second.schema_ref.clone(),
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref(&format!("transformer-{salt}")),
            transformer_kind: "identity".to_string(),
            mode: "explicit".to_string(),
            policy_refs: vec![test_ref(&format!("migration-policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("migration-evidence-{salt}"))],
        })
        .expect("migration recipe");
        let migrated = migrate_value(&root, &input.namespace, &input.key, &recipe, &input.admission).expect("migrate");
        assert_eq!(migrated.old_value_ref, second.value_ref);
        assert_eq!(migrated.new_value_ref, second.value_ref);
        let migrated_ref = parse_entry_ref_value(&migrated.typed_ref_value).expect("migrated ref");
        assert_eq!(migrated_ref.schema_ref, target_schema_ref);
        assert_eq!(migrated_ref.revision, 3);
        let receipt = parse_receipt_value(&migrated.receipt_value, None).expect("migration receipt");
        assert!(receipt.checks.contains(&"migration-trace".to_string()));
        assert!(receipt.checks.contains(&"result-value-hash".to_string()));
    }

    fn typed_ref_fixture(schema_field: IoValue, checks: &[&str], decoder_artifact_refs: Vec<String>) -> IoValue {
        record("typed-storage-ref-v1", vec![
            string(crate::preserves_rail::TYPED_STORAGE_REF_SCHEMA),
            record("namespace", vec![string("fixture")]),
            record("key", vec![string("value")]),
            schema_field,
            record("value-ref", vec![string(test_ref("value"))]),
            record("payload", vec![record("inline", vec![u64_value(0)])]),
            record("producer", vec![string(test_ref("producer"))]),
            refs_record("policy", &[test_ref("policy")]),
            refs_record("evidence", &[test_ref("evidence")]),
            record("revision", vec![u64_value(1)]),
            record("authority", vec![
                string(test_ref("actor")),
                string(test_ref("capability")),
                string(test_ref("effect-handle")),
            ]),
            checks_value(checks),
            record("schema-identity", vec![string(SCHEMA_IDENTITY_MODE_INFERRED_PRESERVES_CLASS)]),
            refs_record("consumers", &[]),
            record("handler-profile", vec![string(STORAGE_HANDLER_PROFILE_REDB)]),
            refs_record("capabilities", &[test_ref("capability")]),
            refs_record("retention", &[test_ref("retention")]),
            refs_record("provenance", &[test_ref("provenance")]),
            refs_record("decoder-artifacts", &decoder_artifact_refs),
        ])
    }

    fn default_typed_ref_checks() -> &'static [&'static str] {
        &[
            "typed-durable-ref",
            "schema-ref-binding",
            "schema-identity-binding",
            "value-ref-binding",
            "producer-artifact-binding",
            "intended-consumer-binding",
            "handler-profile-binding",
            "capability-binding",
            "retention-binding",
            "provenance-binding",
            "evidence-binding",
            "decoder-artifact-admission",
            "handle-not-authority",
            "no-raw-memory-layout",
            "no-function-serialization",
        ]
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("typed-storage-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
