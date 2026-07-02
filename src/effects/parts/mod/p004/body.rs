
fn require_scope_match(scope: &EffectScope, request: &EffectHandleRequest<'_>, label: &str) -> Result<()> {
    if scope.run_ref != request.run_ref {
        return Err(MoltenError::invalid_harness(format!("{label} run scope does not match request")));
    }
    if scope.session_ref != request.session_ref {
        return Err(MoltenError::invalid_harness(format!("{label} session scope does not match request")));
    }
    if scope.actor_ref.as_deref() != request.actor_ref {
        return Err(MoltenError::invalid_harness(format!("{label} actor scope does not match request")));
    }
    if scope.turn_ref.as_deref() != request.turn_ref {
        return Err(MoltenError::invalid_harness(format!("{label} turn scope does not match request")));
    }
    Ok(())
}

fn declared_effect_value(effect: &DeclaredEffect) -> IoValue {
    record("declared-effect", vec![
        record("effect-id", vec![string(&effect.effect_id)]),
        record("operation", vec![string(&effect.operation)]),
        record("schemas", vec![string(&effect.input_schema_ref), string(&effect.output_schema_ref)]),
        refs_record("evidence", &effect.evidence_refs),
    ])
}

fn operations_record(operations: &[String]) -> IoValue {
    record("operations", vec![sequence(operations.iter().map(string).collect())])
}

fn refs_record(label: &'static str, refs: &[String]) -> IoValue {
    record(label, vec![sequence(refs.iter().map(string).collect())])
}

fn checks_value(checks: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|check| record("check", vec![string(*check), string("pass")])).collect(),
    )])
}

fn diagnostics_record(diagnostics: &[String]) -> IoValue {
    record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())])
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_record(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_u64_value(value: &Value<IoValue>) -> Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&some[0], "optional u64").map(Some);
    }
    required_u64(value, "optional u64").map(Some)
}

fn parse_ref_sequence_record(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let sequence = required_sequence(&record[0], label)?;
    let mut refs = Vec::with_capacity(sequence.len());
    for entry in sequence.iter() {
        refs.push(required_ref(entry, label)?);
    }
    Ok(refs)
}

fn parse_string_sequence_record(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let strings = parse_string_sequence_record_unvalidated(value, label)?;
    validate_operations(&strings)?;
    Ok(strings)
}

fn parse_string_sequence_record_unvalidated(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let sequence = required_sequence(&record[0], label)?;
    let mut strings = Vec::with_capacity(sequence.len());
    for entry in sequence.iter() {
        strings.push(required_string(entry, label)?);
    }
    Ok(strings)
}

fn parse_declared_effects(value: &Value<IoValue>) -> Result<Vec<DeclaredEffect>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "effects", 1)?;
    let sequence = required_sequence(&record[0], "declared effects")?;
    let mut effects = Vec::with_capacity(sequence.len());
    for entry in sequence.iter() {
        let entry = value_to_iovalue(entry);
        let fields = simple_record(&entry, "declared-effect", 4)?;
        let schemas = value_to_iovalue(&fields[2]);
        let schemas = simple_record(&schemas, "schemas", 2)?;
        effects.push(DeclaredEffect {
            effect_id: required_record_string(&fields[0], "effect-id", "declared effect id")?,
            operation: required_record_string(&fields[1], "operation", "declared effect operation")?,
            input_schema_ref: required_ref(&schemas[0], "declared effect input schema ref")?,
            output_schema_ref: required_ref(&schemas[1], "declared effect output schema ref")?,
            evidence_refs: parse_ref_sequence_record(&fields[3], "evidence")?,
        });
    }
    validate_declared_effects(&effects)?;
    Ok(effects)
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "effect checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "effect check name")?;
        let status = required_string(&check[1], "effect check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("effect check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_check(checks: &[String], expected: &str, label: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} missing {expected} check")))
    }
}

fn validate_declared_effects(effects: &[DeclaredEffect]) -> Result<()> {
    if effects.is_empty() {
        return Err(MoltenError::invalid_harness("effect manifest must declare at least one effect"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for effect in effects {
        validate_effect_id(&effect.effect_id)?;
        validate_operation(&effect.operation)?;
        require_ref(&effect.input_schema_ref, "declared effect input schema ref")?;
        require_ref(&effect.output_schema_ref, "declared effect output schema ref")?;
        validate_refs(&effect.evidence_refs, "declared effect evidence ref")?;
        let key = (effect.effect_id.as_str(), effect.operation.as_str());
        if !seen.insert(key) {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate declared effect {} operation {}",
                effect.effect_id, effect.operation
            )));
        }
    }
    Ok(())
}

fn validate_operations(operations: &[String]) -> Result<()> {
    if operations.is_empty() {
        return Err(MoltenError::invalid_harness("effect operation set must not be empty"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for operation in operations {
        validate_operation(operation)?;
        if !seen.insert(operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate effect operation {operation}")));
        }
    }
    Ok(())
}

fn validate_effect_id(effect_id: &str) -> Result<()> {
    validate_non_empty(effect_id, "effect id")?;
    if !effect_id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_' | ':' | '/' | '.')
    }) {
        return Err(MoltenError::invalid_harness(format!(
            "effect id {effect_id} must use lowercase ascii, digits, or effect separators"
        )));
    }
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    validate_non_empty(operation, "effect operation")?;
    if !operation.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_' | ':' | '/' | '.')
    }) {
        return Err(MoltenError::invalid_harness(format!(
            "effect operation {operation} must use lowercase ascii, digits, or effect separators"
        )));
    }
    Ok(())
}

fn validate_executor_kind(executor_kind: &str) -> Result<()> {
    match executor_kind {
        "native" | "steel" | "wasm" | "adapter" | "remote-proxy" | "job" | "protocol" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect manifest executor kind {executor_kind}"))),
    }
}

fn validate_handler_profile(profile: &str) -> Result<()> {
    match profile {
        HANDLER_PROFILE_PRODUCTION
        | HANDLER_PROFILE_LOCAL
        | HANDLER_PROFILE_MOCK
        | HANDLER_PROFILE_CHAOS
        | HANDLER_PROFILE_PROFILING
        | HANDLER_PROFILE_DRY_RUN => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect handler profile {profile}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect decision {decision}"))),
    }
}

fn validate_transfer(transfer: &str) -> Result<()> {
    match transfer {
        TRANSFER_LOCAL_ONLY | TRANSFER_ATTENUATED_DELEGATION | TRANSFER_REMOTE_PROXY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect handle transfer policy {transfer}"))),
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value in refs {
        require_ref(value, field)?;
    }
    Ok(())
}

fn validate_unique_refs(refs: &[String], field: &str) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for value in refs {
        require_ref(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {field} {value}")));
        }
    }
    Ok(())
}

fn validate_operation_subset(parent: &[String], child: &[String]) -> Result<()> {
    validate_operations(child)?;
    for operation in child {
        if !parent.iter().any(|candidate| candidate == operation) {
            return Err(MoltenError::invalid_harness(format!(
                "attenuated effect handle operation {operation} is not in parent operation set"
            )));
        }
    }
    Ok(())
}

fn validate_scope_narrows(parent: &EffectScope, child: &EffectScope) -> Result<()> {
    validate_scope(child)?;
    if parent.run_ref != child.run_ref || parent.session_ref != child.session_ref {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot widen run/session scope"));
    }
    if let Some(parent_actor) = parent.actor_ref.as_deref()
        && child.actor_ref.as_deref() != Some(parent_actor)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent actor scope"));
    }
    if let Some(parent_turn) = parent.turn_ref.as_deref()
        && child.turn_ref.as_deref() != Some(parent_turn)
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot escape parent turn scope"));
    }
    Ok(())
}
