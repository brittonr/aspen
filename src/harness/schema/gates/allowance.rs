pub fn budget_limits_value(budget: &super::model::Budget) -> preserves::IOValue {
    crate::preserves_rail::record("budget-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        limits_value(budget),
    ])
}

pub fn budget_value(budget: &super::model::Budget, usage: &super::model::BudgetUsage) -> preserves::IOValue {
    crate::preserves_rail::record("budget-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        limits_value(budget),
        crate::preserves_rail::record("usage", vec![
            crate::preserves_rail::u64_value(usage.steps),
            crate::preserves_rail::u64_value(usage.effects),
            crate::preserves_rail::u64_value(usage.events),
            crate::preserves_rail::u64_value(usage.report_bytes),
        ]),
    ])
}

pub fn parse_budget(value: &preserves::IOValue) -> crate::error::Result<super::model::BudgetEvidence> {
    let budget = super::fields::simple_record(value, "budget-v1", 3)?;
    let limits = parse_budget_limits_record(&budget)?;
    let usage_value = crate::preserves_rail::value_to_iovalue(&budget[2]);
    let usage = super::fields::simple_record(&usage_value, "usage", 4)?;
    let usage = super::model::BudgetUsage {
        steps: super::fields::required_u64(&usage[0], "budget used steps")?,
        effects: super::fields::required_u64(&usage[1], "budget used effects")?,
        events: super::fields::required_u64(&usage[2], "budget used events")?,
        report_bytes: super::fields::required_u64(&usage[3], "budget used report bytes")?,
    };
    Ok(super::model::BudgetEvidence { limits, usage })
}

pub(super) fn parse_budget_limits(value: &preserves::IOValue) -> crate::error::Result<super::model::Budget> {
    let budget = super::fields::simple_record(value, "budget-v1", 2)?;
    parse_budget_limits_record(&budget)
}

fn limits_value(budget: &super::model::Budget) -> preserves::IOValue {
    crate::preserves_rail::record("limits", vec![
        crate::preserves_rail::u64_value(budget.max_steps),
        crate::preserves_rail::u64_value(budget.max_effects),
        crate::preserves_rail::u64_value(budget.max_events),
        crate::preserves_rail::u64_value(budget.max_report_bytes),
    ])
}

fn parse_budget_limits_record(
    budget: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<super::model::Budget> {
    let schema = super::fields::required_string(&budget[0], "budget schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_SCHEMA
        )));
    }
    let limits_value = crate::preserves_rail::value_to_iovalue(&budget[1]);
    let limits = super::fields::simple_record(&limits_value, "limits", 4)?;
    Ok(super::model::Budget {
        max_steps: super::fields::required_u64(&limits[0], "budget max steps")?,
        max_effects: super::fields::required_u64(&limits[1], "budget max effects")?,
        max_events: super::fields::required_u64(&limits[2], "budget max events")?,
        max_report_bytes: super::fields::required_u64(&limits[3], "budget max report bytes")?,
    })
}
