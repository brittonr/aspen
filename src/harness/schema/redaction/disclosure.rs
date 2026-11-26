use preserves::ValueImpl;

const FORBIDDEN_REDACTION_MARKERS: &[&str] = &[
    "secret",
    "confidential",
    "credential",
    "private",
    "encrypted-ref",
    "encrypted-ref-v1",
    "secret-ref-v1",
];

pub(super) fn redaction_policy_value() -> preserves::IOValue {
    crate::preserves_rail::record("redaction-policy-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string("deny-sensitive-markers")]),
        crate::preserves_rail::record("forbidden-markers", vec![crate::preserves_rail::sequence(
            FORBIDDEN_REDACTION_MARKERS.iter().map(|marker| crate::preserves_rail::string(*marker)).collect(),
        )]),
    ])
}

pub(super) fn redaction_gate_value(
    report_value: &preserves::IOValue,
    report: &super::model::Report,
) -> crate::error::Result<preserves::IOValue> {
    if let Some(marker) = first_sensitive_marker(report_value) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "redaction preflight found sensitive marker {marker}; sealed pass repro bundles require explicit redaction before export"
        )));
    }
    let policy = redaction_policy_value();
    let policy_ref = crate::preserves_rail::canonical_hash(&policy)?;
    Ok(crate::preserves_rail::record("redaction-gate-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(policy_ref)]),
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::record("scan-root-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(report_value)?,
        )]),
        redaction_gate_checks_value(),
    ]))
}

fn redaction_gate_checks_value() -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        [
            "redaction-policy",
            "canonical-report-scan",
            "no-secret-markers",
            "no-confidential-markers",
            "no-credential-markers",
            "no-private-markers",
            "no-unvalidated-encrypted-refs",
        ]
        .iter()
        .map(|name| {
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(*name),
                crate::preserves_rail::string("pass"),
            ])
        })
        .collect(),
    )])
}

pub(super) fn validate_redaction_evidence(
    report_value: &preserves::IOValue,
    report: &super::model::Report,
    policy_value: &preserves::IOValue,
    gate_value: &preserves::IOValue,
) -> crate::error::Result<(String, String)> {
    let expected_policy = redaction_policy_value();
    let expected_policy_ref = crate::preserves_rail::canonical_hash(&expected_policy)?;
    let actual_policy_ref = crate::preserves_rail::canonical_hash(policy_value)?;
    if actual_policy_ref != expected_policy_ref || policy_value != &expected_policy {
        return Err(crate::error::Failure::invalid_harness(format!(
            "redaction policy evidence mismatch: policy hashes to {actual_policy_ref}, expected {expected_policy_ref}"
        )));
    }
    parse_redaction_policy(policy_value)?;
    let expected_gate = redaction_gate_value(report_value, report)?;
    let expected_gate_ref = crate::preserves_rail::canonical_hash(&expected_gate)?;
    let actual_gate_ref = crate::preserves_rail::canonical_hash(gate_value)?;
    if actual_gate_ref != expected_gate_ref || gate_value != &expected_gate {
        return Err(crate::error::Failure::invalid_harness(format!(
            "redaction gate evidence mismatch: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    parse_redaction_gate(gate_value, report, &expected_policy_ref, report_value)?;
    Ok((actual_policy_ref, actual_gate_ref))
}

pub(super) fn parse_redaction_policy(value: &preserves::IOValue) -> crate::error::Result<()> {
    let policy = super::fields::simple_record(value, "redaction-policy-v1", 3)?;
    let schema = super::fields::required_string(&policy[0], "redaction policy schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported redaction policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_POLICY_SCHEMA
        )));
    }
    let mode = super::fields::required_record_string(&policy[1], "mode", "redaction policy mode")?;
    if mode != "deny-sensitive-markers" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported redaction policy mode {mode}")));
    }
    let markers =
        super::fields::required_record_sequence(&policy[2], "forbidden-markers", "redaction forbidden markers")?;
    let actual = markers
        .iter()
        .map(|marker| super::fields::required_string(&marker, "redaction marker"))
        .collect::<crate::error::Result<Vec<_>>>()?;
    if actual != FORBIDDEN_REDACTION_MARKERS {
        return Err(crate::error::Failure::invalid_harness("redaction policy forbidden marker set is not canonical"));
    }
    Ok(())
}

fn parse_redaction_gate(
    value: &preserves::IOValue,
    report: &super::model::Report,
    policy_ref: &str,
    report_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let gate = super::fields::simple_record(value, "redaction-gate-v1", 7)?;
    let schema = super::fields::required_string(&gate[0], "redaction gate schema")?;
    if schema != crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported redaction gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REDACTION_GATE_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&gate[1], "decision", "redaction gate decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported redaction gate decision {decision}")));
    }
    let actual_policy_ref = super::fields::required_record_hash(&gate[2], "policy-ref", "redaction policy ref")?;
    if actual_policy_ref != policy_ref {
        return Err(crate::error::Failure::invalid_harness("redaction gate policy ref does not match policy evidence"));
    }
    let report_ref = super::fields::required_record_hash(&gate[3], "report-ref", "redaction gate report ref")?;
    if report_ref != report.report_ref {
        return Err(crate::error::Failure::invalid_harness("redaction gate report ref does not match embedded report"));
    }
    let suite_ref = super::fields::required_record_hash(&gate[4], "suite-ref", "redaction gate suite ref")?;
    if suite_ref != report.suite_ref {
        return Err(crate::error::Failure::invalid_harness("redaction gate suite ref does not match embedded report"));
    }
    let scan_root_ref = super::fields::required_record_hash(&gate[5], "scan-root-ref", "redaction gate scan root ref")?;
    let actual_scan_root_ref = crate::preserves_rail::canonical_hash(report_value)?;
    if scan_root_ref != actual_scan_root_ref {
        return Err(crate::error::Failure::invalid_harness(
            "redaction gate scan root ref does not match embedded report",
        ));
    }
    let checks = parse_redaction_gate_checks(&gate[6])?;
    require_redaction_check(&checks, "redaction-policy")?;
    require_redaction_check(&checks, "canonical-report-scan")?;
    require_redaction_check(&checks, "no-secret-markers")?;
    require_redaction_check(&checks, "no-confidential-markers")?;
    require_redaction_check(&checks, "no-credential-markers")?;
    require_redaction_check(&checks, "no-private-markers")?;
    require_redaction_check(&checks, "no-unvalidated-encrypted-refs")?;
    Ok(())
}

pub(super) fn parse_redaction_gate_checks(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "redaction gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "redaction gate check name")?;
        let status = super::fields::required_string(&check[1], "redaction gate check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!(
                "redaction gate check {name} status is {status}"
            )));
        }
        checks.push(name);
    }
    Ok(checks)
}

pub(super) fn require_redaction_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("redaction gate missing {expected} check")))
    }
}

pub(super) fn is_sensitive_record_label(label: &str) -> bool {
    FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label)
}

fn first_sensitive_marker(value: &preserves::IOValue) -> Option<String> {
    let mut stack = vec![value.clone()];
    while let Some(current) = stack.pop() {
        if current.is_record() {
            if let Some(label) = current.label().as_symbol()
                && FORBIDDEN_REDACTION_MARKERS.iter().any(|marker| marker == &label.as_ref())
            {
                return Some(label.into_owned());
            }
            stack.push(crate::preserves_rail::value_to_iovalue(&current.label()));
        }
        match current.value_class() {
            preserves::ValueClass::Atomic(_) | preserves::ValueClass::Embedded => {}
            preserves::ValueClass::Compound(preserves::CompoundClass::Record)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Sequence)
            | preserves::ValueClass::Compound(preserves::CompoundClass::Set) => {
                for child in current.iter() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&child));
                }
            }
            preserves::ValueClass::Compound(preserves::CompoundClass::Dictionary) => {
                for (key, value) in current.entries() {
                    stack.push(crate::preserves_rail::value_to_iovalue(&key));
                    stack.push(crate::preserves_rail::value_to_iovalue(&value));
                }
            }
        }
    }
    None
}
