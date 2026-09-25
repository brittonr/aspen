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
/// Each field name can add a changed-value, a changed-source, and a changed-caveats diagnostic.
const DIAGNOSTICS_PER_CONFIG_FIELD: usize = 3;
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
    let mut diagnostics = Vec::with_capacity(names.len().saturating_mul(DIAGNOSTICS_PER_CONFIG_FIELD));
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
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<OrderedMap<String, Vec<ConfigSourceInput>>> {
    let mut grouped = OrderedMap::<String, Vec<ConfigSourceInput>>::new();
    for source in &input.sources {
        validate_source(source, diagnostics)?;
        grouped.entry(source.field.clone()).or_default().push(source.clone());
    }
    Ok(grouped)
}

fn validate_source(source: &ConfigSourceInput, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    validate_text("effective config field", &source.field)?;
    validate_text("effective config value", &source.value)?;
    validate_source_class(&source.source_class, diagnostics)?;
    if let Some(source_ref) = source.source_ref.as_ref() {
        validate_ref_with_diagnostics(source_ref, &source.source_class, diagnostics);
    } else if source.source_class != "default" {
        diagnostics.push_item(format!("missing-source-ref:{}:{}", source.field, source.source_class));
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
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<EffectiveConfigField>> {
    ensure_field_bound(grouped.len(), "effective config fields")?;
    let mut fields = Vec::with_capacity(grouped.len());
    for (field, sources) in grouped {
        let selected = select_source(field, sources, diagnostics)?;
        if release_mode && selected.source_class == "default" {
            diagnostics.push_item(format!("fixture-default-in-release:{field}"));
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
