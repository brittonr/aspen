
pub fn get_value_with_migration(input: MigrationGetInput<'_>) -> Result<Get> {
    match get_value_inner(GetValueInnerInput {
        root: input.root,
        namespace: input.namespace,
        key: input.key,
        expected_schema_ref: Some(input.expected_schema_ref),
        admission: input.admission,
        migration_receipt_value: None,
        schema_compatibility_value: None,
    }) {
        Ok(value) => Ok(value),
        Err(first_error) => {
            let recipe = parse_migration_recipe_value(input.migration_recipe_value)?;
            if recipe.target_schema_ref != input.expected_schema_ref {
                return Err(MoltenError::invalid_harness(
                    "typed storage lazy migration rejected: recipe target schema does not match expected schema ref",
                ));
            }
            if !matches!(recipe.mode.as_str(), "lazy-on-read" | "explicit") {
                return Err(MoltenError::invalid_harness(format!(
                    "typed storage lazy migration rejected: recipe mode {} cannot run on read",
                    recipe.mode
                )));
            }
            let migrated =
                migrate_value(input.root, input.namespace, input.key, input.migration_recipe_value, input.admission)
                    .map_err(|migration_error| {
                        MoltenError::invalid_harness(format!(
                            "typed storage lazy migration failed after load miss {first_error}: {migration_error}"
                        ))
                    })?;
            get_value_inner(GetValueInnerInput {
                root: input.root,
                namespace: input.namespace,
                key: input.key,
                expected_schema_ref: Some(input.expected_schema_ref),
                admission: input.admission,
                migration_receipt_value: Some(&migrated.receipt_value),
                schema_compatibility_value: None,
            })
        }
    }
}
