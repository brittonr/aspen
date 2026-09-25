
fn select_source<'a>(
    field: &str,
    sources: &'a [ConfigSourceInput],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<&'a ConfigSourceInput> {
    let mut selected = sources
        .first()
        .ok_or_else(|| MoltenError::invalid_harness(format!("field {field} has no sources")))?;
    for source in sources.iter().skip(1) {
        let selected_precedence = source_precedence(&selected.source_class);
        let source_precedence = source_precedence(&source.source_class);
        if source_precedence > selected_precedence {
            selected = source;
            continue;
        }
        if source_precedence == selected_precedence && source.value != selected.value {
            diagnostics
                .push_item(format!("conflicting-sources:{field}:{}:{}", selected.source_class, source.source_class));
        }
    }
    if selected.source_class == "cli-override" && !selected.admitted_override {
        diagnostics.push_item(format!("unadmitted-cli-override:{field}"));
    }
    Ok(selected)
}

fn merged_caveats(sources: &[ConfigSourceInput]) -> Result<Vec<String>> {
    let mut caveats = OrderedSet::new();
    for source in sources {
        for caveat in &source.caveats {
            caveats.insert(caveat.clone());
        }
    }
    ensure_caveat_bound(caveats.len(), "effective config merged caveats")?;
    Ok(caveats.into_iter().collect())
}

fn source_precedence(source_class: &str) -> u8 {
    match source_class {
        "cli-override" => SOURCE_PRECEDENCE_CLI,
        "profile" => SOURCE_PRECEDENCE_PROFILE,
        "environment" => SOURCE_PRECEDENCE_ENV,
        "ledger" => SOURCE_PRECEDENCE_LEDGER,
        "default" => SOURCE_PRECEDENCE_DEFAULT,
        _ => SOURCE_PRECEDENCE_DEFAULT,
    }
}

fn validate_source_class(source_class: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match source_class {
        "profile" | "cli-override" | "default" | "environment" | "ledger" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-source-class:{other}"));
            Ok(())
        }
    }
}

fn field_map(fields: &[EffectiveConfigField]) -> Result<OrderedMap<String, EffectiveConfigField>> {
    let mut output = OrderedMap::new();
    for field in fields {
        if output.insert(field.field.clone(), field.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate effective field {}", field.field)));
        }
    }
    Ok(output)
}

fn effective_config_value(
    input: &EffectiveConfigInput,
    decision: &str,
    diagnostics: &[String],
    fields: &[EffectiveConfigField],
) -> Result<IoValue> {
    Ok(record("effective-config-readback-v1", vec![
        string(EFFECTIVE_CONFIG_SCHEMA),
        field_string("decision", decision),
        field_sequence("profile-refs", string_values(&input.profile_refs)?),
        record("release-mode", vec![bool_value(input.release_mode)]),
        field_sequence("fields", effective_field_values(fields)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn effective_config_diff_value(
    left: &EffectiveConfigReadback,
    right: &EffectiveConfigReadback,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("effective-config-diff-v1", vec![
        string(EFFECTIVE_CONFIG_DIFF_SCHEMA),
        field_string("decision", decision),
        field_string("left-ref", &left.fingerprint_ref),
        field_string("right-ref", &right.fingerprint_ref),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn effective_field_values(fields: &[EffectiveConfigField]) -> Result<Vec<IoValue>> {
    fields
        .iter()
        .map(|field| {
            Ok(record("field", vec![
                field_string("name", &field.field),
                field_string("value", &field.value),
                field_string("selected-source-class", &field.selected_source_class),
                field_string("selected-source-ref", field.selected_source_ref.as_deref().unwrap_or(NONE_REF)),
                field_sequence("caveats", string_values(&field.caveats)?),
                field_sequence("traces", source_values(&field.traces)?),
            ]))
        })
        .collect()
}

fn source_values(sources: &[ConfigSourceInput]) -> Result<Vec<IoValue>> {
    sources
        .iter()
        .map(|source| {
            Ok(record("source", vec![
                field_string("field", &source.field),
                field_string("value", &source.value),
                field_string("source-class", &source.source_class),
                field_string("source-ref", source.source_ref.as_deref().unwrap_or(NONE_REF)),
                record("admitted-override", vec![bool_value(source.admitted_override)]),
                field_sequence("caveats", string_values(&source.caveats)?),
            ]))
        })
        .collect()
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_ref_with_diagnostics(reference: &str, label: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if let Err(error) = validate_ref(reference, label) {
        diagnostics.push_item(format!("stale-ref:{label}:{reference}:{error}"));
    }
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_PROFILE_REFS, label)
}

fn ensure_source_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_SOURCES, label)
}

fn ensure_field_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_FIELDS, label)
}

fn ensure_caveat_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_CAVEATS, label)
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "effective config diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn source(field: &str, value: &str, source_class: &str, source_ref: Option<String>) -> ConfigSourceInput {
        ConfigSourceInput {
            field: field.to_string(),
            value: value.to_string(),
            source_class: source_class.to_string(),
            source_ref,
            admitted_override: source_class == "cli-override",
            caveats: Vec::new(),
        }
    }

    fn valid_input() -> EffectiveConfigInput {
        EffectiveConfigInput {
            profile_refs: vec![local_ref("profile")],
            sources: vec![
                source("node.id", "node:local", "profile", Some(local_ref("profile-node"))),
                source("state.root", "target/node", "cli-override", Some(local_ref("cli-state-root"))),
                ConfigSourceInput {
                    caveats: vec!["local fixture only".to_string()],
                    ..source("max.events", "16", "default", None)
                },
            ],
            release_mode: false,
            diagnostics: Vec::new(),
        }
    }

    // r[verify molten.project.effective_config_readback.artifact]
    // r[verify molten.project.effective_config_readback.source_trace]
    // r[verify molten.project.effective_config_readback.cli_core]
    // r[verify molten.project.effective_config_readback.evidence_only]
    #[test]
    fn effective_config_readback_has_stable_canonical_identity() {
        let first = build_effective_config_readback(&valid_input()).expect("first readback");
        let second = build_effective_config_readback(&valid_input()).expect("second readback");
        assert_eq!(first.decision, DECISION_PASS);
        assert_eq!(first.fingerprint_ref, second.fingerprint_ref);
        assert!(explain_effective_config(&first).expect("explain").contains("effective-config ref=blake3:"));
    }

    #[test]
    fn effective_config_denies_conflicts_hidden_defaults_and_stale_refs() {
        let mut input = valid_input();
        input.release_mode = true;
        input.sources.push(source("node.id", "node:other", "profile", Some(local_ref("other-profile"))));
        input.sources.push(source("policy.ref", "blake3:policy", "ledger", Some("not-a-ref".to_string())));
        let readback = build_effective_config_readback(&input).expect("readback");
        assert_eq!(readback.decision, DECISION_DENY);
        assert!(readback.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("conflicting-sources:node.id")));
        assert!(readback.diagnostics.iter().any(|diagnostic| diagnostic == "fixture-default-in-release:max.events"));
        assert!(readback.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("stale-ref:ledger:not-a-ref")));
    }

    #[test]
    fn effective_config_diff_reports_normalized_changes() {
        let left = build_effective_config_readback(&valid_input()).expect("left");
        let mut changed = valid_input();
        changed.sources.push(source("node.id", "node:local", "cli-override", Some(local_ref("cli-node"))));
        let right = build_effective_config_readback(&changed).expect("right");
        let diff = diff_effective_config_readbacks(&left, &right).expect("diff");
        assert_eq!(diff.decision, DECISION_DENY);
        assert!(diff.diagnostics.iter().any(|diagnostic| diagnostic == "changed-source:node.id"));
    }

    #[test]
    fn effective_config_readback_cannot_authorize_mutation_by_itself() {
        let readback = build_effective_config_readback(&valid_input()).expect("readback");
        let use_decision =
            evaluate_readback_authorization_use(&readback.fingerprint_ref, "install", &[]).expect("authorization use");
        assert_eq!(use_decision.decision, DECISION_DENY);
        assert!(
            use_decision
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "effective-config-readback-is-evidence-only")
        );
        assert!(use_decision.diagnostics.iter().any(|diagnostic| diagnostic == "missing-subsystem-evidence:install"));
    }
}
