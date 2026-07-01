pub(super) fn put(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Put {
        value,
        store,
        namespace,
        key,
        schema_ref,
        producer_ref,
        ref_out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("put");
    };
    let value = super::io::read_preserves_file(&value)?;
    let producer_ref = match producer_ref {
        Some(producer_ref) => producer_ref,
        None => super::io::cli_storage_ref("producer", &namespace, &key)?,
    };
    let admission = molten::typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
    let put = molten::typed_storage::put_value(&store, &molten::typed_storage::TypedStoragePutInput {
        namespace: namespace.clone(),
        key: key.clone(),
        schema_ref,
        value,
        producer_ref,
        policy_refs: vec![admission.policy_ref.clone()],
        evidence_refs: admission.evidence_refs.clone(),
        admission,
    })?;
    if let Some(path) = ref_out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&put.typed_ref_value)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &put.receipt_value)?;
    println!(
        "storage put ok namespace={} key={} storage_ref={} schema_ref={} value_ref={} store={}",
        namespace,
        key,
        put.storage_ref,
        put.schema_ref,
        put.value_ref,
        store.display()
    );
    Ok(())
}

pub(super) fn get(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Get {
        store,
        namespace,
        key,
        schema_ref,
        migration_recipe,
        out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("get");
    };
    let admission = molten::typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
    let get = if let Some(migration_recipe) = migration_recipe.as_ref() {
        let expected_schema_ref = schema_ref.as_deref().ok_or_else(|| {
            molten::error::MoltenError::invalid_harness("storage get --migration-recipe requires --schema-ref target")
        })?;
        let recipe_value = super::io::read_preserves_file(migration_recipe)?;
        molten::typed_storage::get_value_with_migration(molten::typed_storage::MigrationGetInput {
            root: &store,
            namespace: &namespace,
            key: &key,
            expected_schema_ref,
            migration_recipe_value: &recipe_value,
            admission: &admission,
        })?
    } else {
        molten::typed_storage::get_value(&store, &namespace, &key, schema_ref.as_deref(), &admission)?
    };
    let text = molten::preserves_rail::to_text(&get.value)?;
    if let Some(path) = out.as_ref() {
        super::io::write_file(path, &text)?;
    } else {
        println!("{text}");
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &get.receipt_value)?;
    eprintln!(
        "storage get ok namespace={} key={} storage_ref={} schema_ref={} store={}",
        namespace,
        key,
        get.storage_ref,
        get.typed_ref.schema_ref,
        store.display()
    );
    Ok(())
}

pub(super) fn recipe(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Recipe {
        source_schema_ref,
        target_schema_ref,
        transformer_ref,
        transformer_kind,
        mode,
        out,
    } = command
    else {
        return dispatch_mismatch("recipe");
    };
    let recipe = molten::typed_storage::migration_recipe_value(&molten::typed_storage::StorageMigrationRecipeInput {
        source_schema_ref,
        target_schema_ref,
        transformer_ref,
        transformer_kind,
        mode,
        policy_refs: vec![super::io::cli_storage_ref("migration-policy", "recipe", "policy")?],
        evidence_refs: vec![super::io::cli_storage_ref("migration-evidence", "recipe", "evidence")?],
    })?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&recipe)?)?;
    println!(
        "storage recipe ok recipe_ref={} out={}",
        molten::preserves_rail::canonical_hash(&recipe)?,
        out.display()
    );
    Ok(())
}

pub(super) fn migrate(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Migrate {
        recipe,
        store,
        namespace,
        key,
        ref_out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("migrate");
    };
    let recipe = super::io::read_preserves_file(&recipe)?;
    let admission = molten::typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
    let migrated = molten::typed_storage::migrate_value(&store, &namespace, &key, &recipe, &admission)?;
    if let Some(path) = ref_out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&migrated.typed_ref_value)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &migrated.receipt_value)?;
    println!(
        "storage migrate ok namespace={} key={} old_ref={} new_ref={} recipe={} store={}",
        namespace,
        key,
        migrated.old_storage_ref,
        migrated.new_storage_ref,
        migrated.recipe_ref,
        store.display()
    );
    Ok(())
}

pub(super) fn verify(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Verify {
        storage_ref,
        store,
        schema_ref,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("verify");
    };
    let verified = molten::typed_storage::verify_ref(&store, &storage_ref, schema_ref.as_deref())?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &verified.receipt_value)?;
    println!(
        "storage verify ok storage_ref={} namespace={} key={} schema_ref={} store={}",
        verified.storage_ref,
        verified.typed_ref.namespace,
        verified.typed_ref.key,
        verified.typed_ref.schema_ref,
        store.display()
    );
    Ok(())
}

fn dispatch_mismatch(command: &str) -> molten::error::Result<()> {
    Err(molten::error::MoltenError::invalid_harness(format!("storage {command} dispatch mismatch")))
}
