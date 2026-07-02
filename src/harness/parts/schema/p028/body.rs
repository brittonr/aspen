
fn validate_profiled_output(value: &IoValue, profile: ReproExportProfile) -> Result<Vec<String>> {
    let mut encrypted_refs = Vec::with_capacity(8);
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol() {
                let label = label.as_ref();
                if matches!(label, "secret" | "confidential" | "credential" | "private" | "secret-ref-v1") {
                    return Err(MoltenError::invalid_harness(format!(
                        "redaction transform missed sensitive marker {label}"
                    )));
                }
                if label == "encrypted-ref" {
                    return Err(MoltenError::invalid_harness(
                        "malformed encrypted-ref marker in redacted repro bundle",
                    ));
                }
                if label == "encrypted-ref-v1" {
                    if profile != ReproExportProfile::EncryptedPrivate {
                        return Err(MoltenError::invalid_harness(
                            "encrypted refs are allowed only in encrypted-private repro bundles",
                        ));
                    }
                    let encrypted = crate::secrets::parse_encrypted_ref(&current)?;
                    ensure_redaction_bound(
                        encrypted_refs.len() + 1,
                        MAX_REDACTION_ENCRYPTED_REFS,
                        "redaction encrypted refs",
                    )?;
                    encrypted_refs.push(encrypted.encrypted_ref);
                    continue;
                }
            }
            stack.push(value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    encrypted_refs.sort();
    encrypted_refs.dedup();
    Ok(encrypted_refs)
}

fn first_sensitive_marker(value: &IoValue) -> Option<String> {
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol()
                && FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label.as_ref())
            {
                return Some(label.into_owned());
            }
            stack.push(value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            ValueClass::Atomic(_) | ValueClass::Embedded => {}
            ValueClass::Compound(CompoundClass::Record)
            | ValueClass::Compound(CompoundClass::Sequence)
            | ValueClass::Compound(CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(value_to_iovalue(&child));
                }
            }
            ValueClass::Compound(CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(value_to_iovalue(&key));
                    stack.push(value_to_iovalue(&value));
                }
            }
        }
    }
    None
}

fn repro_seal_value(report: &Report, gate_receipt_ref: &str) -> IoValue {
    record("repro-seal", vec![
        string(crate::preserves_rail::HARNESS_REPRO_SEAL_SCHEMA),
        record("decision", vec![string("pass")]),
        record("gate-receipt-ref", vec![string(gate_receipt_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("profile", vec![string(&report.profile)]),
        record("replay-status", vec![string(&report.replay_status)]),
    ])
}

fn sealed_repro_checks_value() -> IoValue {
    record("seal-checks", vec![sequence(
        [
            "sealed-report",
            "embedded-gate-receipt",
            "report-ref-binding",
            "suite-ref-binding",
            "actor-registry-binding",
            "effect-log-binding",
            "policy-gate-ref-binding",
            "capability-gate-ref-binding",
            "budget-gate-ref-binding",
            "redaction-preflight",
            "redaction-gate-ref-binding",
            "no-sensitive-markers",
            "replay-metadata-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn default_report_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "report.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn default_failure_bundle_command() -> Vec<String> {
    [
        "molten",
        "test",
        "repro",
        "export",
        "failure.preserves",
        "--out",
        "repro",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn required_record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_hash(&record[0], field)
}

fn required_record_bool(value: &Value<IoValue>, label: &str, field: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_bool(&record[0], field)
}

fn required_record_u64(value: &Value<IoValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], field)
}

fn required_record_sequence(value: &Value<IoValue>, label: &str, field: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let owned = value_to_iovalue(&record[0]);
    Ok(owned
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))?
        .into_owned())
}

fn required_record_hash_sequence(value: &Value<IoValue>, label: &str, field: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_hash(&value, field)).collect()
}

fn required_record_string_sequence(value: &Value<IoValue>, label: &str, field: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

fn required_record_iovalue_sequence(value: &Value<IoValue>, label: &str, field: &str) -> Result<Vec<IoValue>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], field)?;
    Ok(values.iter().map(|value| value_to_iovalue(&value)).collect())
}

fn validate_tool_record(value: &Value<IoValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let tool = simple_record(&value, "tool", 2)?;
    let name = required_string(&tool[0], "repro bundle tool name")?;
    if name != "molten" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro bundle tool {name}")));
    }
    let version = required_string(&tool[1], "repro bundle tool version")?;
    if version.is_empty() {
        return Err(MoltenError::invalid_harness("repro bundle tool version must not be empty"));
    }
    Ok(())
}

fn validate_sequence_record(value: &Value<IoValue>, label: &str, field: &str) -> Result<()> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_sequence(&record[0], field)?;
    Ok(())
}

fn parse_artifact_refs(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let artifact_refs = simple_record(&value, "artifact-refs", 1)?;
    let ref_values = required_sequence(&artifact_refs[0], "repro bundle artifact refs")?;
    let mut refs = Vec::with_capacity(ref_values.len());
    for ref_value in ref_values.iter() {
        let ref_value = value_to_iovalue(&ref_value);
        let artifact_ref = simple_record(&ref_value, "artifact-ref", 2)?;
        refs.push((
            required_string(&artifact_ref[0], "artifact ref kind")?,
            required_string(&artifact_ref[1], "artifact ref value")?,
        ));
    }
    Ok(refs)
}

fn require_artifact_ref(refs: &[(String, String)], kind: &str, expected: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, actual_ref)| actual_kind == kind && actual_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("repro bundle artifact refs missing {kind} ref {expected}")))
    }
}

pub fn effect_log_value(entries: &[EffectLogEntry]) -> IoValue {
    record("effect-log-v1", vec![
        string(crate::preserves_rail::HARNESS_EFFECT_LOG_SCHEMA),
        sequence(
            entries
                .iter()
                .map(|entry| {
                    record("effect-entry", vec![
                        u64_value(entry.sequence),
                        entry.request.clone(),
                        entry.response.clone(),
                    ])
                })
                .collect(),
        ),
    ])
}

pub fn budget_limits_value(budget: &Budget) -> IoValue {
    record("budget-v1", vec![
        string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        limits_value(budget),
    ])
}

pub fn budget_value(budget: &Budget, usage: &BudgetUsage) -> IoValue {
    record("budget-v1", vec![
        string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        limits_value(budget),
        record("usage", vec![
            u64_value(usage.steps),
            u64_value(usage.effects),
            u64_value(usage.events),
            u64_value(usage.report_bytes),
        ]),
    ])
}
