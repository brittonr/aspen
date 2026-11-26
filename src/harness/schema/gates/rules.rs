use preserves::ValueImpl;

pub fn policy_value(policy: &crate::runtime::AdmissionPolicy) -> preserves::IOValue {
    crate::preserves_rail::record("policy-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_POLICY_SCHEMA),
        crate::preserves_rail::sequence(policy.deny_rules().iter().map(deny_rule_value).collect()),
    ])
}

pub fn policy_gate_value(policy: &crate::runtime::AdmissionPolicy) -> crate::error::Result<preserves::IOValue> {
    let preflight = super::covenant::policy_preflight_material(policy)?;
    Ok(crate::preserves_rail::record("policy-gate-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(&preflight.policy_ref)]),
        preflight.nickel_source_value,
        preflight.nickel_contract_value,
        preflight.basalt_preflight_value,
        crate::preserves_rail::record("steel-predicates", vec![crate::preserves_rail::sequence(Vec::new())]),
        policy_gate_checks_value(),
    ]))
}

pub fn parse_policy_gate(value: &preserves::IOValue) -> crate::error::Result<super::observed::PolicyGateEvidence> {
    let gate = super::fields::simple_record(value, "policy-gate-v1", 8)?;
    let schema = super::fields::required_string(&gate[0], "policy gate schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&gate[1], "decision", "policy gate decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported policy gate decision {decision}")));
    }
    let policy_ref = super::fields::required_record_hash(&gate[2], "policy-ref", "policy gate policy ref")?;
    let nickel_source = super::covenant::parse_nickel_source_evidence(&gate[3])?;
    let nickel_contract = super::covenant::parse_nickel_contract_evidence(&gate[4])?;
    let basalt_preflight = super::covenant::parse_basalt_policy_preflight_evidence(&gate[5])?;
    if nickel_source.policy_ref != policy_ref {
        return Err(crate::error::Failure::invalid_harness("Nickel source policy ref does not match policy gate ref"));
    }
    if nickel_contract.normalized_source_ref != nickel_source.source_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Nickel contract normalized source ref does not match Nickel source evidence",
        ));
    }
    if basalt_preflight.policy_ref != policy_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt policy preflight policy ref does not match policy gate ref",
        ));
    }
    if basalt_preflight.envelope_ref != nickel_contract.envelope_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt policy preflight envelope ref does not match Nickel contract envelope",
        ));
    }
    if basalt_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt policy preflight source ref does not match Nickel source evidence",
        ));
    }
    let steel_predicates =
        super::fields::required_record_sequence(&gate[6], "steel-predicates", "policy gate Steel predicates")?;
    if !steel_predicates.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "Steel predicates require reviewed callable receipts and are disabled in local harness policy gates",
        ));
    }
    let checks = parse_policy_gate_checks(&gate[7])?;
    require_policy_gate_check(&checks, "policy-schema")?;
    require_policy_gate_check(&checks, "canonical-policy-snapshot")?;
    require_policy_gate_check(&checks, "nickel-static-boundary")?;
    require_policy_gate_check(&checks, "nickel-policy-source")?;
    require_policy_gate_check(&checks, "nickel-export-normalization")?;
    require_policy_gate_check(&checks, "basalt-preflight")?;
    require_policy_gate_check(&checks, "basalt-receipt-binding")?;
    require_policy_gate_check(&checks, "steel-predicate-review")?;
    Ok(super::observed::PolicyGateEvidence {
        value: value.clone(),
        policy_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: basalt_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_policy_gate_evidence(
    suite: &super::model::Suite,
    policy_gate: Option<&super::observed::PolicyGateEvidence>,
) -> crate::error::Result<()> {
    let policy_gate = policy_gate.ok_or_else(|| {
        crate::error::Failure::invalid_harness(
            "missing policy gate evidence; policy must pass preflight before side effects",
        )
    })?;
    let expected_ref = crate::preserves_rail::canonical_hash(&policy_value(&suite.policy))?;
    if policy_gate.policy_ref != expected_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "policy gate ref mismatch: gate has {}, embedded policy hashes to {expected_ref}",
            policy_gate.policy_ref
        )));
    }
    let expected_gate = policy_gate_value(&suite.policy)?;
    let expected_gate_ref = crate::preserves_rail::canonical_hash(&expected_gate)?;
    let actual_gate_ref = crate::preserves_rail::canonical_hash(&policy_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "policy gate evidence does not match embedded suite policy preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

fn deny_rule_value(rule: &crate::runtime::AdmissionDenyRule) -> preserves::IOValue {
    crate::preserves_rail::record("deny", vec![
        optional_policy_string(rule.actor.as_deref()),
        optional_policy_action(rule.action.as_ref()),
        optional_policy_string(rule.target.as_deref()),
        optional_policy_runtime_value(rule.value.as_ref()),
        crate::preserves_rail::string(&rule.reason),
    ])
}

pub(super) fn optional_policy_string(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(|| crate::preserves_rail::bool_value(false), crate::preserves_rail::string)
}

pub(super) fn optional_policy_action(value: Option<&crate::runtime::AdmissionAction>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::bool_value(false),
        |action| crate::preserves_rail::string(action.as_str()),
    )
}

pub(super) fn optional_policy_runtime_value(
    value: Option<&crate::harness::nucleus::RuntimeValue>,
) -> preserves::IOValue {
    value.map_or_else(|| crate::preserves_rail::bool_value(false), |value| value.as_iovalue().clone())
}

fn policy_gate_checks_value() -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
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
        .map(|name| {
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(*name),
                crate::preserves_rail::string("pass"),
            ])
        })
        .collect(),
    )])
}

fn parse_policy_gate_checks(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "policy gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "policy gate check name")?;
        let status = super::fields::required_string(&check[1], "policy gate check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!("policy gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_policy_gate_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("policy gate missing {expected} check")))
    }
}

pub fn parse_policy(value: &preserves::IOValue) -> crate::error::Result<crate::runtime::AdmissionPolicy> {
    let policy = super::fields::simple_record(value, "policy-v1", 2)?;
    let schema = super::fields::required_string(&policy[0], "policy schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_SCHEMA
        )));
    }
    let rule_values = super::fields::required_sequence(&policy[1], "policy deny rules")?;
    let mut rules = Vec::with_capacity(rule_values.len());
    for rule in rule_values.iter() {
        let rule_value = crate::preserves_rail::value_to_iovalue(&rule);
        if rule_value.collect_simple_record("steel-predicate", None).is_some()
            || rule_value.collect_simple_record("dynamic-predicate", None).is_some()
        {
            return Err(crate::error::Failure::invalid_harness(
                "Steel predicates require reviewed callable receipts and are disabled in local harness policy fixtures",
            ));
        }
        let rule = super::fields::simple_record(&rule_value, "deny", 5)?;
        let actor = super::fields::optional_string(&rule[0], "policy deny actor")?;
        let action = super::fields::optional_action(&rule[1], "policy deny action")?;
        let target = super::fields::optional_string(&rule[2], "policy deny target")?;
        let value = super::fields::optional_runtime_match_value(&rule[3])?;
        let reason = super::fields::required_string(&rule[4], "policy deny reason")?;
        if reason.is_empty() {
            return Err(crate::error::Failure::invalid_harness("policy deny reason must not be empty"));
        }
        rules.push(crate::runtime::AdmissionDenyRule {
            actor,
            action,
            target,
            value,
            reason,
        });
    }
    Ok(crate::runtime::AdmissionPolicy::from_deny_rules(rules))
}
