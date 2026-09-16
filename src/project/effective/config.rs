type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const EFFECTIVE_CONFIG_SCHEMA: &str = "molten.project.effective-config-readback.v1";
const EFFECTIVE_CONFIG_DIFF_SCHEMA: &str = "molten.project.effective-config-diff.v1";
const EFFECTIVE_CONFIG_AUTHORIZATION_SCHEMA: &str = "molten.project.effective-config-authorization-use.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const NONE_REF: &str = "none";
const MAX_PROFILE_REFS: usize = 128;
const MAX_SOURCES: usize = 512;
const MAX_FIELDS: usize = 512;
const MAX_CAVEATS: usize = 128;
const MAX_DIAGNOSTICS: usize = 4096;
const SOURCE_PRECEDENCE_CLI: u8 = 50;
const SOURCE_PRECEDENCE_PROFILE: u8 = 40;
const SOURCE_PRECEDENCE_ENV: u8 = 30;
const SOURCE_PRECEDENCE_LEDGER: u8 = 20;
const SOURCE_PRECEDENCE_DEFAULT: u8 = 10;
const EVIDENCE_ONLY_CAVEAT: &str = "effective-config readbacks are evidence-only diagnostics and do not grant authority, policy admission, provenance trust, source-gate acceptance, resource rights, retention clearance, transport correctness, execution permission, deployment trust, or release eligibility";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSourceInput {
    pub field: String,
    pub value: String,
    pub source_class: String,
    pub source_ref: Option<String>,
    pub admitted_override: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfigInput {
    pub profile_refs: Vec<String>,
    pub sources: Vec<ConfigSourceInput>,
    pub release_mode: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfigField {
    pub field: String,
    pub value: String,
    pub selected_source_class: String,
    pub selected_source_ref: Option<String>,
    pub caveats: Vec<String>,
    pub traces: Vec<ConfigSourceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfigReadback {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub fields: Vec<EffectiveConfigField>,
    pub fingerprint_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfigDiff {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub diff_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfigAuthorizationUse {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

// r[impl molten.project.effective_config_readback.artifact]
// r[impl molten.project.effective_config_readback.source_trace]
// r[impl molten.project.effective_config_readback.cli_core]
// r[impl molten.project.effective_config_readback.evidence_only]
pub fn build_effective_config_readback(input: &EffectiveConfigInput) -> Result<EffectiveConfigReadback> {
    ensure_ref_bound(input.profile_refs.len(), "effective config profile refs")?;
    ensure_source_bound(input.sources.len(), "effective config sources")?;
    let mut diagnostics = input.diagnostics.clone();
    for profile_ref in &input.profile_refs {
        validate_ref_with_diagnostics(profile_ref, "effective config profile", &mut diagnostics);
    }
    let grouped = group_sources(input, &mut diagnostics)?;
    let fields = select_effective_fields(&grouped, input.release_mode, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = effective_config_value(input, decision, &diagnostics, &fields)?;
    let fingerprint_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(EffectiveConfigReadback {
        decision: decision.to_string(),
        diagnostics,
        fields,
        fingerprint_ref,
        value,
    })
}

pub fn explain_effective_config(readback: &EffectiveConfigReadback) -> Result<String> {
    let mut lines = vec![format!(
        "effective-config ref={} decision={}",
        readback.fingerprint_ref, readback.decision
    )];
    for field in &readback.fields {
        lines.push(format!(
            "{}={} source={} ref={} caveats={}",
            field.field,
            field.value,
            field.selected_source_class,
            field.selected_source_ref.as_deref().unwrap_or(NONE_REF),
            field.caveats.join(",")
        ));
    }
    if !readback.diagnostics.is_empty() {
        lines.push(format!("diagnostics={}", readback.diagnostics.join("; ")));
    }
    lines.push(EVIDENCE_ONLY_CAVEAT.to_string());
    Ok(lines.join("\n"))
}

pub fn diff_effective_config_readbacks(
    left: &EffectiveConfigReadback,
    right: &EffectiveConfigReadback,
) -> Result<EffectiveConfigDiff> {
    let left_fields = field_map(&left.fields)?;
    let right_fields = field_map(&right.fields)?;
    let mut names = OrderedSet::new();
    names.extend(left_fields.keys().cloned());
    names.extend(right_fields.keys().cloned());
    let mut diagnostics = Vec::new();
    for name in names {
        match (left_fields.get(&name), right_fields.get(&name)) {
            (Some(left), Some(right)) => {
                if left.value != right.value {
                    diagnostics.push(format!("changed-value:{name}"));
                }
                if left.selected_source_class != right.selected_source_class
                    || left.selected_source_ref != right.selected_source_ref
                {
                    diagnostics.push(format!("changed-source:{name}"));
                }
                if left.caveats != right.caveats {
                    diagnostics.push(format!("changed-caveats:{name}"));
                }
            }
            (Some(_), None) => diagnostics.push(format!("removed-field:{name}")),
            (None, Some(_)) => diagnostics.push(format!("added-field:{name}")),
            (None, None) => {}
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = effective_config_diff_value(left, right, decision, &diagnostics)?;
    let diff_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(EffectiveConfigDiff {
        decision: decision.to_string(),
        diagnostics,
        diff_ref,
        value,
    })
}

pub fn evaluate_readback_authorization_use(
    readback_ref: &str,
    requested_gate: &str,
    subsystem_evidence_refs: &[String],
) -> Result<EffectiveConfigAuthorizationUse> {
    validate_ref(readback_ref, "effective config readback")?;
    validate_text("requested gate", requested_gate)?;
    ensure_ref_bound(subsystem_evidence_refs.len(), "subsystem evidence refs")?;
    let mut diagnostics = Vec::new();
    for reference in subsystem_evidence_refs {
        validate_ref_with_diagnostics(reference, "subsystem evidence", &mut diagnostics);
    }
    diagnostics.push("effective-config-readback-is-evidence-only".to_string());
    if subsystem_evidence_refs.is_empty() {
        diagnostics.push(format!("missing-subsystem-evidence:{requested_gate}"));
    }
    diagnostics.sort();
    diagnostics.dedup();
    let value = record("effective-config-authorization-use-v1", vec![
        string(EFFECTIVE_CONFIG_AUTHORIZATION_SCHEMA),
        field_string("decision", DECISION_DENY),
        field_string("readback-ref", readback_ref),
        field_string("requested-gate", requested_gate),
        field_sequence("subsystem-evidence", string_values(subsystem_evidence_refs)?),
        field_sequence("diagnostics", string_values(&diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]);
    Ok(EffectiveConfigAuthorizationUse {
        decision: DECISION_DENY.to_string(),
        diagnostics,
        value,
    })
}

fn group_sources(
    input: &EffectiveConfigInput,
    diagnostics: &mut Vec<String>,
) -> Result<OrderedMap<String, Vec<ConfigSourceInput>>> {
    let mut grouped = OrderedMap::<String, Vec<ConfigSourceInput>>::new();
    for source in &input.sources {
        validate_source(source, diagnostics)?;
        grouped.entry(source.field.clone()).or_default().push(source.clone());
    }
    Ok(grouped)
}

fn validate_source(source: &ConfigSourceInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_text("effective config field", &source.field)?;
    validate_text("effective config value", &source.value)?;
    validate_source_class(&source.source_class, diagnostics)?;
    if let Some(source_ref) = source.source_ref.as_ref() {
        validate_ref_with_diagnostics(source_ref, &source.source_class, diagnostics);
    } else if source.source_class != "default" {
        diagnostics.push(format!("missing-source-ref:{}:{}", source.field, source.source_class));
    }
    ensure_caveat_bound(source.caveats.len(), "effective config source caveats")?;
    for caveat in &source.caveats {
        validate_text("effective config caveat", caveat)?;
    }
    Ok(())
}

fn select_effective_fields(
    grouped: &OrderedMap<String, Vec<ConfigSourceInput>>,
    release_mode: bool,
    diagnostics: &mut Vec<String>,
) -> Result<Vec<EffectiveConfigField>> {
    ensure_field_bound(grouped.len(), "effective config fields")?;
    let mut fields = Vec::with_capacity(grouped.len());
    for (field, sources) in grouped {
        let selected = select_source(field, sources, diagnostics)?;
        if release_mode && selected.source_class == "default" {
            diagnostics.push(format!("fixture-default-in-release:{field}"));
        }
        let caveats = merged_caveats(sources)?;
        fields.push(EffectiveConfigField {
            field: field.clone(),
            value: selected.value.clone(),
            selected_source_class: selected.source_class.clone(),
            selected_source_ref: selected.source_ref.clone(),
            caveats,
            traces: sources.clone(),
        });
    }
    Ok(fields)
}

fn select_source<'a>(
    field: &str,
    sources: &'a [ConfigSourceInput],
    diagnostics: &mut Vec<String>,
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
            diagnostics.push(format!("conflicting-sources:{field}:{}:{}", selected.source_class, source.source_class));
        }
    }
    if selected.source_class == "cli-override" && !selected.admitted_override {
        diagnostics.push(format!("unadmitted-cli-override:{field}"));
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

fn validate_source_class(source_class: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match source_class {
        "profile" | "cli-override" | "default" | "environment" | "ledger" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-source-class:{other}"));
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

fn validate_ref_with_diagnostics(reference: &str, label: &str, diagnostics: &mut Vec<String>) {
    if let Err(error) = validate_ref(reference, label) {
        diagnostics.push(format!("stale-ref:{label}:{reference}:{error}"));
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
