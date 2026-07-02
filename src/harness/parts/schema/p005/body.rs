
struct GateSet {
    cursor: usize,
    policy_gate: Option<PolicyGateEvidence>,
    capability_gate: Option<CapabilityGateEvidence>,
    budget_gate: Option<BudgetGateEvidence>,
}

pub fn parse_report(report_value: &IoValue) -> Result<Report> {
    let report = report_value
        .collect_simple_record("harness-report-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-report-v1 ...>"))?;
    let arity = valid_report_arity(&report)?;
    let header = parsed_header(&report)?;
    let gates = gate_set(&report, arity)?;
    let (cursor, actors) = registry_after_gates(&report, gates.cursor, &header.suite)?;
    let (cursor, executor_preflights) = preflights_after_registry(&report, cursor, arity)?;
    let (cursor, observations) = observations_after(&report, cursor)?;
    let (effect_log, budget) = log_and_budget(&report, cursor, &header.suite)?;

    Ok(Report {
        report_ref: canonical_hash(report_value)?,
        status: header.status,
        replay_status: header.replay_status,
        profile: header.profile,
        hash_algorithm: header.hash_algorithm,
        suite_ref: header.suite_ref,
        initial_state_hash: header.initial_state_hash,
        final_state_hash: header.final_state_hash,
        suite_value: header.suite_value,
        policy_gate: gates.policy_gate,
        capability_gate: gates.capability_gate,
        budget_gate: gates.budget_gate,
        actors,
        executor_preflights,
        observations,
        effect_log,
        budget,
    })
}

fn valid_report_arity(report: &Record<Value<IoValue>>) -> Result<usize> {
    let arity = report.fields_iter().count();
    if arity != 13 && arity != 14 && arity != 15 && arity != 16 && arity != 17 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <harness-report-v1 ...> with arity 13, 14, 15, 16, or 17, got {arity}"
        )));
    }
    Ok(arity)
}

fn parsed_header(report: &Record<Value<IoValue>>) -> Result<ParsedHeader> {
    let schema = required_string(&report[0], "report schema")?;
    if schema != crate::preserves_rail::HARNESS_REPORT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported report schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPORT_SCHEMA
        )));
    }
    let status = required_string(&report[1], "report status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("evidence-bearing report status must be pass, got {status}")));
    }
    let replay_status = required_string(&report[2], "report replay status")?;
    if !matches!(replay_status.as_str(), "deterministic" | "replay" | "record") {
        return Err(MoltenError::invalid_harness(format!("unsupported evidence replay status {replay_status}")));
    }
    let profile = required_string(&report[3], "report profile")?;
    let hash_algorithm = required_string(&report[4], "report hash algorithm")?;
    if hash_algorithm != crate::preserves_rail::HASH_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported hash algorithm {hash_algorithm}; expected {}",
            crate::preserves_rail::HASH_ALGORITHM
        )));
    }
    let suite_ref = required_string(&report[5], "report suite ref")?;
    let suite_value = value_to_iovalue(&report[8]);
    let suite = parse_suite(&suite_value)?;
    let actual_suite_ref = canonical_hash(&suite_value)?;
    if suite_ref != actual_suite_ref {
        return Err(MoltenError::invalid_harness(format!(
            "suite ref mismatch: report has {suite_ref}, embedded suite hashes to {actual_suite_ref}"
        )));
    }
    Ok(ParsedHeader {
        status,
        replay_status,
        profile,
        hash_algorithm,
        suite_ref,
        initial_state_hash: required_hash(&report[6], "report initial state hash")?,
        final_state_hash: required_hash(&report[7], "report final state hash")?,
        suite_value,
        suite,
    })
}

fn gate_set(report: &Record<Value<IoValue>>, arity: usize) -> Result<GateSet> {
    let mut cursor = 9;
    let policy_gate = optional_policy_gate(report, &mut cursor, arity)?;
    let capability_gate = optional_capability_gate(report, &mut cursor, arity)?;
    let budget_gate = optional_budget_gate(report, &mut cursor, arity)?;
    Ok(GateSet {
        cursor,
        policy_gate,
        capability_gate,
        budget_gate,
    })
}

fn optional_policy_gate(
    report: &Record<Value<IoValue>>,
    cursor: &mut usize,
    arity: usize,
) -> Result<Option<PolicyGateEvidence>> {
    if *cursor < arity && value_has_record_label(&report[*cursor], "policy-gate-v1") {
        let parsed = parse_policy_gate(&value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn optional_capability_gate(
    report: &Record<Value<IoValue>>,
    cursor: &mut usize,
    arity: usize,
) -> Result<Option<CapabilityGateEvidence>> {
    if *cursor < arity && value_has_record_label(&report[*cursor], "capability-gate-v1") {
        let parsed = parse_capability_gate(&value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn optional_budget_gate(
    report: &Record<Value<IoValue>>,
    cursor: &mut usize,
    arity: usize,
) -> Result<Option<BudgetGateEvidence>> {
    if *cursor < arity && value_has_record_label(&report[*cursor], "budget-gate-v1") {
        let parsed = parse_budget_gate(&value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn registry_after_gates(
    report: &Record<Value<IoValue>>,
    cursor: usize,
    suite: &Suite,
) -> Result<(usize, Vec<ActorDecl>)> {
    let actors = parse_actor_registry(&value_to_iovalue(&report[cursor]))?;
    if actors != suite.actors {
        return Err(MoltenError::invalid_harness("report actor registry does not match embedded suite actor registry"));
    }
    Ok((cursor + 1, actors))
}

fn preflights_after_registry(
    report: &Record<Value<IoValue>>,
    cursor: usize,
    arity: usize,
) -> Result<(usize, Option<ExecutorPreflightsEvidence>)> {
    if cursor < arity && value_has_record_label(&report[cursor], "executor-preflights-v1") {
        let parsed = parse_executor_preflights(&value_to_iovalue(&report[cursor]))?;
        return Ok((cursor + 1, Some(parsed)));
    }
    Ok((cursor, None))
}

fn observations_after(report: &Record<Value<IoValue>>, cursor: usize) -> Result<(usize, Vec<Observation>)> {
    let observation_values = required_sequence(&report[cursor], "report observations")?;
    let mut observations = Vec::with_capacity(observation_values.len());
    for (position, observation) in observation_values.iter().enumerate() {
        let parsed = parse_observation(&observation)?;
        if parsed.index != position as u64 {
            return Err(MoltenError::invalid_harness(format!(
                "observation index mismatch at position {position}: got {}",
                parsed.index
            )));
        }
        observations.push(parsed);
    }
    Ok((cursor + 1, observations))
}

fn log_and_budget(
    report: &Record<Value<IoValue>>,
    cursor: usize,
    suite: &Suite,
) -> Result<(Vec<EffectLogEntry>, BudgetEvidence)> {
    let effect_log_value = value_to_iovalue(&report[cursor]);
    let effect_log = parse_effect_log(&effect_log_value)?;
    let budget_value = value_to_iovalue(&report[cursor + 1]);
    let budget = parse_budget(&budget_value)?;
    if budget.limits != suite.budget {
        return Err(MoltenError::invalid_harness("report budget limits do not match embedded suite budget"));
    }
    Ok((effect_log, budget))
}

pub fn validate_budget_fixture_evidence(suite: &Suite) -> Result<()> {
    if suite.budget_explicit {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ))
    }
}

pub fn budget_gate_value(budget: &Budget) -> Result<IoValue> {
    let preflight = budget_preflight_material(budget)?;
    Ok(record("budget-gate-v1", vec![
        string(crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("budget-ref", vec![string(&preflight.budget_ref)]),
        preflight.nickel_source_value,
        preflight.resource_contract_value,
        preflight.resource_preflight_value,
        budget_gate_checks_value(),
    ]))
}

pub fn parse_budget_gate(value: &IoValue) -> Result<BudgetGateEvidence> {
    let gate = simple_record(value, "budget-gate-v1", 7)?;
    let schema = required_string(&gate[0], "budget gate schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_GATE_SCHEMA
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "budget gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported budget gate decision {decision}")));
    }
    let budget_ref = required_record_hash(&gate[2], "budget-ref", "budget gate budget ref")?;
    let nickel_source = parse_budget_nickel_source_evidence(&gate[3])?;
    let resource_contract = parse_resource_contract_evidence(&gate[4])?;
    let resource_preflight = parse_basalt_resource_preflight_evidence(&gate[5])?;
    if nickel_source.budget_ref != budget_ref {
        return Err(MoltenError::invalid_harness("Nickel resource policy budget ref does not match budget gate ref"));
    }
    if resource_contract.normalized_budget_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "resource contract normalized budget source ref does not match Nickel resource policy evidence",
        ));
    }
    if resource_preflight.budget_ref != budget_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt resource preflight budget ref does not match budget gate ref",
        ));
    }
    if resource_preflight.envelope_ref != resource_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt resource preflight envelope ref does not match resource contract envelope",
        ));
    }
    if resource_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
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
    Ok(BudgetGateEvidence {
        value: value.clone(),
        budget_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: resource_preflight.receipt_ref,
        checks,
    })
}
