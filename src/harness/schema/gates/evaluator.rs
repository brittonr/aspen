pub(super) fn nickel_policy_source(
    policy: &crate::runtime::AdmissionPolicy,
    policy_ref: &str,
) -> crate::error::Result<String> {
    let mut source = String::from("{\n");
    source.push_str(&format!(
        "  schema_version = {},\n",
        nickel_string(crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA)
    ));
    source.push_str(&format!("  policy_schema = {},\n", nickel_string(crate::preserves_rail::HARNESS_POLICY_SCHEMA)));
    source.push_str(&format!("  policy_ref = {},\n", nickel_string(policy_ref)));
    source.push_str("  deny_rules = [\n");
    for rule in policy.deny_rules() {
        source.push_str("    {\n");
        source.push_str(&format!("      actor = {},\n", nickel_optional_string(rule.actor.as_deref())));
        source.push_str(&format!(
            "      action = {},\n",
            nickel_optional_string(rule.action.as_ref().map(crate::runtime::AdmissionAction::as_str))
        ));
        source.push_str(&format!("      target = {},\n", nickel_optional_string(rule.target.as_deref())));
        source.push_str(&format!("      value = {},\n", nickel_optional_runtime_value(rule.value.as_ref())?));
        source.push_str(&format!("      reason = {},\n", nickel_string(&rule.reason)));
        source.push_str("    },\n");
    }
    source.push_str("  ],\n}");
    Ok(source)
}

fn nickel_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), nickel_string)
}

fn nickel_optional_runtime_value(
    value: Option<&crate::harness::nucleus::RuntimeValue>,
) -> crate::error::Result<String> {
    match value {
        Some(value) => {
            let text = crate::preserves_rail::to_text(value.as_iovalue())?;
            let value_ref = crate::preserves_rail::canonical_hash(value.as_iovalue())?;
            Ok(format!("{{ preserves = {}, ref = {} }}", nickel_string(&text), nickel_string(&value_ref)))
        }
        None => Ok("null".to_string()),
    }
}

pub(super) fn nickel_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => escaped.push_str(&format!("\\u{{{:x}}}", character as u32)),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

// r[impl molten.nickel_toolchain.boundary]
pub(super) fn nickel_export_json(source: &str) -> crate::error::Result<String> {
    let mut context = nickel_lang::Context::new();
    let expression = context.eval_deep_for_export(source).map_err(nickel_error)?;
    context.expr_to_json(&expression).map_err(nickel_error)
}

fn nickel_error(error: nickel_lang::Error) -> crate::error::Failure {
    let mut message = Vec::new();
    if error.format(&mut message, nickel_lang::ErrorFormat::Text).is_ok() {
        crate::error::Failure::invalid_harness(format!(
            "Nickel static policy normalization failed: {}",
            String::from_utf8_lossy(&message).trim()
        ))
    } else {
        crate::error::Failure::invalid_harness(format!("Nickel static policy normalization failed: {error:?}"))
    }
}
