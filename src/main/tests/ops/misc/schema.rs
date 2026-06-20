    #[test]
    fn cli_schema_identity_commands_work() {
        let dir = temp_dir("schema-cli");
        let files = schema_cli_files(&dir);
        write_file(&files.shape, schema_shape()).expect("write shape");
        let refs = run_schema_identities(&dir, &files);
        run_schema_alias_and_compat(&dir, &files, &refs);
        run_schema_fingerprint_search(&dir.join("registry"), schema_shape());
    }

    struct SchemaCliFiles {
        shape: PathBuf,
        expected_identity: PathBuf,
        actual_identity: PathBuf,
        alias: PathBuf,
        compat: PathBuf,
    }

    struct SchemaCliRefs {
        expected: String,
        actual: String,
    }

    fn schema_shape() -> &'static str {
        r#"<shape "record" "profile" [<shape "field" "name" <shape "string">> <shape "field" "age" <shape "u64">>]>"#
    }

    fn schema_cli_files(dir: &Path) -> SchemaCliFiles {
        SchemaCliFiles {
            shape: dir.join("shape.preserves"),
            expected_identity: dir.join("expected-identity.preserves"),
            actual_identity: dir.join("actual-identity.preserves"),
            alias: dir.join("alias.preserves"),
            compat: dir.join("compat.preserves"),
        }
    }

    fn run_schema_identities(dir: &Path, files: &SchemaCliFiles) -> SchemaCliRefs {
        let expected = test_ref("expected-schema-cli");
        let actual = test_ref("actual-schema-cli");
        run_schema_command(SchemaCommand::Identity {
            shape: files.shape.clone(),
            schema_ref: expected.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: files.expected_identity.clone(),
            receipt_out: Some(dir.join("expected-identity-receipt.preserves")),
        })
        .expect("schema expected identity");
        run_schema_command(SchemaCommand::Identity {
            shape: files.shape.clone(),
            schema_ref: actual.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: files.actual_identity.clone(),
            receipt_out: Some(dir.join("actual-identity-receipt.preserves")),
        })
        .expect("schema actual identity");
        SchemaCliRefs { expected, actual }
    }

    fn run_schema_alias_and_compat(dir: &Path, files: &SchemaCliFiles, refs: &SchemaCliRefs) {
        run_schema_command(SchemaCommand::Alias {
            from_ref: refs.actual.clone(),
            to_ref: refs.expected.clone(),
            scope: "storage".to_string(),
            out: files.alias.clone(),
            receipt_out: Some(dir.join("alias-receipt.preserves")),
        })
        .expect("schema alias");
        run_schema_command(SchemaCommand::Compat {
            expected_identity: files.expected_identity.clone(),
            actual_identity: files.actual_identity.clone(),
            alias: Some(files.alias.clone()),
            migration_ref: None,
            out: Some(files.compat.clone()),
            receipt_out: Some(dir.join("compat-receipt.preserves")),
        })
        .expect("schema compat");
        assert!(
            fs::read_to_string(&files.compat)
                .expect("read compat")
                .contains("schema-compatibility-v1")
        );
    }

    fn run_schema_fingerprint_search(registry: &Path, shape: &str) {
        let schema_artifact = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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
        artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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
            registry: registry.to_path_buf(),
            fingerprint: identity.structural_fingerprint,
        })
        .expect("schema search fingerprint");
    }
