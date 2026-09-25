
pub fn parse_capabilities(value: &IoValue) -> Result<crate::runtime::CapabilityContext> {
    let capabilities = simple_record(value, "capabilities-v1", 2)?;
    let schema = required_string(&capabilities[0], "capabilities schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capabilities schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA
        )));
    }
    let grant_values = required_sequence(&capabilities[1], "capability grants")?;
    let mut grants = Vec::with_capacity(grant_values.len());
    for grant in grant_values.iter() {
        let grant_value = value_to_iovalue(&grant);
        let grant = simple_record(&grant_value, "grant", 4)?;
        grants.push(crate::runtime::CapabilityGrant {
            actor: optional_string(&grant[0], "capability grant actor")?,
            action: optional_action(&grant[1], "capability grant action")?,
            target: optional_string(&grant[2], "capability grant target")?,
            value: optional_runtime_match_value(&grant[3])?,
        });
    }
    Ok(crate::runtime::CapabilityContext::from_grants(grants))
}

fn capability_grant_value(grant: &crate::runtime::CapabilityGrant) -> IoValue {
    record("grant", vec![
        optional_policy_string(grant.actor.as_deref()),
        optional_policy_action(grant.action.as_ref()),
        optional_policy_string(grant.target.as_deref()),
        optional_policy_runtime_value(grant.value.as_ref()),
    ])
}

fn capability_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "capability-schema",
            "canonical-capability-context",
            "deny-by-default",
            "explicit-capability-fixture",
            "no-implicit-authority",
            "basalt-authority-preflight",
            "basalt-authority-receipt",
            "capability-proofset-binding",
            "ucan-verification-receipt-binding",
            "basalt-enforcement-receipt-binding",
            "grant-ref-binding",
            "derived-grant-ref-binding",
            "fixture-authority-evidence-only",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_capability_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "capability gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "capability gate check name")?;
        let status = required_string(&check[1], "capability gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("capability gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_capability_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("capability gate missing {expected} check")))
    }
}

fn deny_rule_value(rule: &crate::runtime::AdmissionDenyRule) -> IoValue {
    record("deny", vec![
        optional_policy_string(rule.actor.as_deref()),
        optional_policy_action(rule.action.as_ref()),
        optional_policy_string(rule.target.as_deref()),
        optional_policy_runtime_value(rule.value.as_ref()),
        string(&rule.reason),
    ])
}

fn optional_policy_string(value: Option<&str>) -> IoValue {
    value.map_or_else(|| bool_value(false), string)
}

fn optional_policy_action(value: Option<&crate::runtime::AdmissionAction>) -> IoValue {
    value.map_or_else(|| bool_value(false), |action| string(action.as_str()))
}

fn optional_policy_runtime_value(value: Option<&super::core::RuntimeValue>) -> IoValue {
    value.map_or_else(|| bool_value(false), |value| value.as_iovalue().clone())
}

fn policy_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "policy-schema",
            "canonical-policy-snapshot",
            "nickel-static-boundary",
            "nickel-policy-source",
            "nickel-export-normalization",
            "basalt-preflight",
            "basalt-receipt-binding",
            "steel-predicate-review",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_policy_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "policy gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "policy gate check name")?;
        let status = required_string(&check[1], "policy gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("policy gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_policy_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("policy gate missing {expected} check")))
    }
}
