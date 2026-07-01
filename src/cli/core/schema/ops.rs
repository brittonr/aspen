pub(super) fn identity(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Identity {
        shape,
        schema_ref,
        mode,
        brand_ref,
        out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("identity");
    };
    let shape = super::io::read_preserves_file(&shape)?;
    let value = molten::schema_identity::schema_identity_value(&molten::schema_identity::SchemaIdentityInput {
        mode,
        schema_ref,
        shape,
        brand_ref,
        metadata_refs: vec![super::io::cli_schema_ref("metadata", "identity")?],
        policy_refs: vec![super::io::cli_schema_ref("policy", "identity")?],
        evidence_refs: vec![super::io::cli_schema_ref("evidence", "identity")?],
    })?;
    let identity = molten::schema_identity::parse_schema_identity(&value)?;
    let receipt = molten::schema_identity::compatibility_receipt_value(
        "fingerprint",
        &molten::schema_identity::compatibility_decision_value(&molten::schema_identity::SchemaCompatibilityInput {
            expected: identity.clone(),
            actual: identity.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: identity.policy_refs.clone(),
            evidence_refs: identity.evidence_refs.clone(),
            deny_by_policy: false,
        })?,
    )?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&value)?)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
    println!(
        "schema identity ok identity={} schema={} fingerprint={} out={}",
        identity.identity_ref,
        identity.schema_ref,
        identity.structural_fingerprint,
        out.display()
    );
    Ok(())
}

pub(super) fn alias(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Alias {
        from_ref,
        to_ref,
        scope,
        out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("alias");
    };
    let value = molten::schema_identity::schema_alias_value(&molten::schema_identity::SchemaAliasInput {
        from_schema_ref: from_ref,
        to_schema_ref: to_ref,
        scope,
        policy_refs: vec![super::io::cli_schema_ref("policy", "alias")?],
        evidence_refs: vec![super::io::cli_schema_ref("evidence", "alias")?],
    })?;
    let alias = molten::schema_identity::parse_schema_alias(&value)?;
    let expected = local_unique_schema_identity(&alias.to_schema_ref)?;
    let actual = local_unique_schema_identity(&alias.from_schema_ref)?;
    let compatibility =
        molten::schema_identity::compatibility_decision_value(&molten::schema_identity::SchemaCompatibilityInput {
            expected,
            actual,
            alias: Some(alias.clone()),
            migration_ref: None,
            policy_refs: alias.policy_refs.clone(),
            evidence_refs: alias.evidence_refs.clone(),
            deny_by_policy: false,
        })?;
    let receipt = molten::schema_identity::compatibility_receipt_value("alias-admit", &compatibility)?;
    super::io::write_file(&out, &molten::preserves_rail::to_text(&value)?)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
    println!(
        "schema alias ok alias={} from={} to={} out={}",
        alias.alias_ref,
        alias.from_schema_ref,
        alias.to_schema_ref,
        out.display()
    );
    Ok(())
}

pub(super) fn compat(command: super::Command) -> molten::error::Result<()> {
    let super::Command::Compat {
        expected_identity,
        actual_identity,
        alias,
        migration_ref,
        out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("compat");
    };
    let expected =
        molten::schema_identity::parse_schema_identity(&super::io::read_preserves_file(&expected_identity)?)?;
    let actual = molten::schema_identity::parse_schema_identity(&super::io::read_preserves_file(&actual_identity)?)?;
    let alias = alias
        .as_ref()
        .map(|path| {
            super::io::read_preserves_file(path).and_then(|value| molten::schema_identity::parse_schema_alias(&value))
        })
        .transpose()?;
    let compatibility =
        molten::schema_identity::compatibility_decision_value(&molten::schema_identity::SchemaCompatibilityInput {
            expected,
            actual,
            alias,
            migration_ref,
            policy_refs: vec![super::io::cli_schema_ref("policy", "compat")?],
            evidence_refs: vec![super::io::cli_schema_ref("evidence", "compat")?],
            deny_by_policy: false,
        })?;
    let parsed = molten::schema_identity::parse_schema_compatibility(&compatibility)?;
    let receipt = molten::schema_identity::compatibility_receipt_value("compatibility", &compatibility)?;
    if let Some(path) = out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&compatibility)?)?;
    } else {
        println!("{}", molten::preserves_rail::to_text(&compatibility)?);
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
    eprintln!("schema compat ok decision={} compatibility={}", parsed.decision, parsed.compatibility_ref);
    Ok(())
}

pub(super) fn search_fingerprint(command: super::Command) -> molten::error::Result<()> {
    let super::Command::SearchFingerprint { registry, fingerprint } = command else {
        return dispatch_mismatch("search-fingerprint");
    };
    for identity in molten::schema_identity::search_registry_by_fingerprint(&registry, &fingerprint)? {
        println!("{} {} {}", identity.identity_ref, identity.schema_ref, identity.mode);
    }
    Ok(())
}

fn local_unique_schema_identity(schema_ref: &str) -> molten::error::Result<molten::schema_identity::SchemaIdentity> {
    let shape = molten::preserves_rail::record("shape", vec![molten::preserves_rail::string("any-preserves")]);
    let value = molten::schema_identity::schema_identity_value(&molten::schema_identity::SchemaIdentityInput {
        mode: molten::schema_identity::MODE_UNIQUE.to_string(),
        schema_ref: schema_ref.to_string(),
        shape,
        brand_ref: None,
        metadata_refs: vec![super::io::cli_schema_ref("metadata", schema_ref)?],
        policy_refs: vec![super::io::cli_schema_ref("policy", schema_ref)?],
        evidence_refs: vec![super::io::cli_schema_ref("evidence", schema_ref)?],
    })?;
    molten::schema_identity::parse_schema_identity(&value)
}

fn dispatch_mismatch(command: &str) -> molten::error::Result<()> {
    Err(molten::error::MoltenError::invalid_harness(format!("schema {command} dispatch mismatch")))
}
