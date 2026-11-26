use preserves::ValueImpl;

pub fn validate_budget_fixture_evidence(suite: &super::model::Suite) -> crate::error::Result<()> {
    if suite.budget_explicit {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ))
    }
}

pub fn budget_gate_value(budget: &super::model::Budget) -> crate::error::Result<preserves::IOValue> {
    let preflight = super::allotment::budget_preflight_material(budget)?;
    Ok(crate::preserves_rail::record("budget-gate-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(&preflight.budget_ref)]),
        preflight.nickel_source_value,
        preflight.resource_contract_value,
        preflight.resource_preflight_value,
        budget_gate_checks_value(),
    ]))
}

pub fn parse_budget_gate(value: &preserves::IOValue) -> crate::error::Result<super::observed::BudgetGateEvidence> {
    let gate = super::fields::simple_record(value, "budget-gate-v1", 7)?;
    let schema = super::fields::required_string(&gate[0], "budget gate schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&gate[1], "decision", "budget gate decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported budget gate decision {decision}")));
    }
    let budget_ref = super::fields::required_record_hash(&gate[2], "budget-ref", "budget gate budget ref")?;
    let nickel_source = super::allotment::parse_budget_nickel_source_evidence(&gate[3])?;
    let resource_contract = super::allotment::parse_resource_contract_evidence(&gate[4])?;
    let resource_preflight = super::allotment::parse_basalt_resource_preflight_evidence(&gate[5])?;
    if nickel_source.budget_ref != budget_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Nickel resource policy budget ref does not match budget gate ref",
        ));
    }
    if resource_contract.normalized_budget_ref != nickel_source.source_ref {
        return Err(crate::error::Failure::invalid_harness(
            "resource contract normalized budget source ref does not match Nickel resource policy evidence",
        ));
    }
    if resource_preflight.budget_ref != budget_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt resource preflight budget ref does not match budget gate ref",
        ));
    }
    if resource_preflight.envelope_ref != resource_contract.envelope_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt resource preflight envelope ref does not match resource contract envelope",
        ));
    }
    if resource_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt resource preflight source ref does not match Nickel resource policy evidence",
        ));
    }
    let checks = parse_budget_gate_checks(&gate[6])?;
    require_budget_gate_check(&checks, "budget-schema")?;
    require_budget_gate_check(&checks, "canonical-budget-snapshot")?;
    require_budget_gate_check(&checks, "explicit-budget-fixture")?;
    require_budget_gate_check(&checks, "no-default-resource-policy")?;
    require_budget_gate_check(&checks, "resource-policy-preflight")?;
    require_budget_gate_check(&checks, "nickel-resource-policy")?;
    require_budget_gate_check(&checks, "nickel-resource-export")?;
    require_budget_gate_check(&checks, "basalt-resource-receipt")?;
    require_budget_gate_check(&checks, "budget-usage-binding")?;
    Ok(super::observed::BudgetGateEvidence {
        value: value.clone(),
        budget_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: resource_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_budget_gate_evidence(
    suite: &super::model::Suite,
    budget_gate: Option<&super::observed::BudgetGateEvidence>,
) -> crate::error::Result<()> {
    if !suite.budget_explicit {
        return Err(crate::error::Failure::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ));
    }
    let budget_gate = budget_gate.ok_or_else(|| {
        crate::error::Failure::invalid_harness(
            "missing budget gate evidence; resource policy must pass preflight before side effects",
        )
    })?;
    let expected_ref = crate::preserves_rail::canonical_hash(&super::allowance::budget_limits_value(&suite.budget))?;
    if budget_gate.budget_ref != expected_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "budget gate ref mismatch: gate has {}, embedded budget hashes to {expected_ref}",
            budget_gate.budget_ref
        )));
    }
    let expected_gate = budget_gate_value(&suite.budget)?;
    let expected_gate_ref = crate::preserves_rail::canonical_hash(&expected_gate)?;
    let actual_gate_ref = crate::preserves_rail::canonical_hash(&budget_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "budget gate evidence does not match embedded resource preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

pub(super) fn nickel_budget_source(budget: &super::model::Budget, budget_ref: &str) -> String {
    format!(
        "{{\n  schema_version = {},\n  budget_schema = {},\n  budget_ref = {},\n  limits = {{\n    max_steps = {},\n    max_effects = {},\n    max_events = {},\n    max_report_bytes = {},\n  }},\n}}",
        super::evaluator::nickel_string(crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        super::evaluator::nickel_string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        super::evaluator::nickel_string(budget_ref),
        budget.max_steps,
        budget.max_effects,
        budget.max_events,
        budget.max_report_bytes,
    )
}

fn budget_gate_checks_value() -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        [
            "budget-schema",
            "canonical-budget-snapshot",
            "explicit-budget-fixture",
            "no-default-resource-policy",
            "resource-policy-preflight",
            "nickel-resource-policy",
            "nickel-resource-export",
            "basalt-resource-preflight",
            "basalt-resource-receipt",
            "budget-usage-binding",
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

fn parse_budget_gate_checks(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "budget gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "budget gate check name")?;
        let status = super::fields::required_string(&check[1], "budget gate check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!("budget gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_budget_gate_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("budget gate missing {expected} check")))
    }
}
