    #[test]
    fn cli_typed_storage_commands_work() {
        let dir = temp_dir("storage-cli");
        let store = dir.join("typed-storage");
        let typed = put_profile_value(&dir, &store);
        get_and_verify_profile(&dir, &store, &typed);
        migrate_profile(&dir, &store, &typed.schema_ref);
        assert_wrong_schema_denied(store);
    }

    struct StoredProfile {
        schema_ref: String,
        storage_ref: String,
    }

    fn put_profile_value(dir: &Path, store: &Path) -> StoredProfile {
        let value_file = dir.join("value.preserves");
        let typed_ref_out = dir.join("typed-ref.preserves");
        write_file(&value_file, "<profile \"alice\" 7>").expect("write value");
        run_storage_command(StorageCommand::Put {
            value: value_file,
            store: store.to_path_buf(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            producer_ref: None,
            ref_out: Some(typed_ref_out.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("storage put");
        let typed_ref_value = read_preserves_file(&typed_ref_out).expect("read typed ref");
        let typed_ref = typed_storage::parse_typed_ref_value(&typed_ref_value).expect("parse typed ref");
        StoredProfile {
            schema_ref: typed_ref.schema_ref,
            storage_ref: typed_ref.storage_ref,
        }
    }

    fn get_and_verify_profile(dir: &Path, store: &Path, typed: &StoredProfile) {
        let get_out = dir.join("get.preserves");
        run_storage_command(StorageCommand::Get {
            store: store.to_path_buf(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(typed.schema_ref.clone()),
            migration_recipe: None,
            out: Some(get_out.clone()),
            receipt_out: Some(dir.join("get-receipt.preserves")),
        })
        .expect("storage get");
        assert_eq!(fs::read_to_string(&get_out).expect("read get"), "<profile \"alice\" 7>");
        run_storage_command(StorageCommand::Verify {
            storage_ref: typed.storage_ref.clone(),
            store: store.to_path_buf(),
            schema_ref: Some(typed.schema_ref.clone()),
            receipt_out: Some(dir.join("verify-receipt.preserves")),
        })
        .expect("storage verify");
    }

    fn migrate_profile(dir: &Path, store: &Path, source_schema_ref: &str) {
        let recipe_path = dir.join("migration-recipe.preserves");
        let target_schema_ref = test_ref("storage-cli-target-schema");
        run_storage_command(StorageCommand::Recipe {
            source_schema_ref: source_schema_ref.to_string(),
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref("storage-cli-transformer"),
            transformer_kind: "schema-rename".to_string(),
            mode: "explicit".to_string(),
            out: recipe_path.clone(),
        })
        .expect("storage recipe");
        run_storage_command(StorageCommand::Migrate {
            recipe: recipe_path,
            store: store.to_path_buf(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            ref_out: Some(dir.join("migrated-ref.preserves")),
            receipt_out: Some(dir.join("migrate-receipt.preserves")),
        })
        .expect("storage migrate");
        run_storage_command(StorageCommand::Get {
            store: store.to_path_buf(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(target_schema_ref),
            migration_recipe: None,
            out: Some(dir.join("get-migrated.preserves")),
            receipt_out: Some(dir.join("get-migrated-receipt.preserves")),
        })
        .expect("storage get migrated");
    }

    fn assert_wrong_schema_denied(store: PathBuf) {
        let error = run_storage_command(StorageCommand::Get {
            store,
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(test_ref("wrong-schema")),
            migration_recipe: None,
            out: None,
            receipt_out: None,
        })
        .expect_err("wrong schema get denied");
        assert!(error.to_string().contains("schema ref"), "{error}");
    }
