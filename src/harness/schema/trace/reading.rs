use preserves::ValueImpl;

struct ParsedHeader {
    status: String,
    replay_status: String,
    profile: String,
    hash_algorithm: String,
    suite_ref: String,
    initial_state_hash: String,
    final_state_hash: String,
    suite_value: preserves::IOValue,
    suite: super::model::Suite,
}

struct GateSet {
    cursor: usize,
    policy_gate: Option<super::observed::PolicyGateEvidence>,
    capability_gate: Option<super::observed::CapabilityGateEvidence>,
    budget_gate: Option<super::observed::BudgetGateEvidence>,
}

pub fn parse_report(report_value: &preserves::IOValue) -> crate::error::Result<super::model::Report> {
    let report = report_value
        .collect_simple_record("harness-report-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <harness-report-v1 ...>"))?;
    let arity = valid_report_arity(&report)?;
    let header = parsed_header(&report)?;
    let gates = gate_set(&report, arity)?;
    let (cursor, actors) = registry_after_gates(&report, gates.cursor, &header.suite)?;
    let (cursor, executor_preflights) = preflights_after_registry(&report, cursor, arity)?;
    let (cursor, observations) = observations_after(&report, cursor)?;
    let (effect_log, budget) = log_and_budget(&report, cursor, &header.suite)?;

    Ok(super::model::Report {
        report_ref: crate::preserves_rail::canonical_hash(report_value)?,
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

fn valid_report_arity(report: &preserves::Record<preserves::Value<preserves::IOValue>>) -> crate::error::Result<usize> {
    let arity = report.fields_iter().count();
    if arity != 13 && arity != 14 && arity != 15 && arity != 16 && arity != 17 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <harness-report-v1 ...> with arity 13, 14, 15, 16, or 17, got {arity}"
        )));
    }
    Ok(arity)
}

fn parsed_header(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<ParsedHeader> {
    let schema = super::fields::required_string(&report[0], "report schema")?;
    if schema != crate::preserves_rail::HARNESS_REPORT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported report schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPORT_SCHEMA
        )));
    }
    let status = super::fields::required_string(&report[1], "report status")?;
    if status != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "evidence-bearing report status must be pass, got {status}"
        )));
    }
    let replay_status = super::fields::required_string(&report[2], "report replay status")?;
    if !matches!(replay_status.as_str(), "deterministic" | "replay" | "record") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported evidence replay status {replay_status}"
        )));
    }
    let profile = super::fields::required_string(&report[3], "report profile")?;
    let hash_algorithm = super::fields::required_string(&report[4], "report hash algorithm")?;
    if hash_algorithm != crate::preserves_rail::HASH_ALGORITHM {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported hash algorithm {hash_algorithm}; expected {}",
            crate::preserves_rail::HASH_ALGORITHM
        )));
    }
    let suite_ref = super::fields::required_string(&report[5], "report suite ref")?;
    let suite_value = crate::preserves_rail::value_to_iovalue(&report[8]);
    let suite = super::intake::parse_suite(&suite_value)?;
    let actual_suite_ref = crate::preserves_rail::canonical_hash(&suite_value)?;
    if suite_ref != actual_suite_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "suite ref mismatch: report has {suite_ref}, embedded suite hashes to {actual_suite_ref}"
        )));
    }
    Ok(ParsedHeader {
        status,
        replay_status,
        profile,
        hash_algorithm,
        suite_ref,
        initial_state_hash: super::fields::required_hash(&report[6], "report initial state hash")?,
        final_state_hash: super::fields::required_hash(&report[7], "report final state hash")?,
        suite_value,
        suite,
    })
}

fn gate_set(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    arity: usize,
) -> crate::error::Result<GateSet> {
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
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: &mut usize,
    arity: usize,
) -> crate::error::Result<Option<super::observed::PolicyGateEvidence>> {
    if *cursor < arity && super::fields::value_has_record_label(&report[*cursor], "policy-gate-v1") {
        let parsed = super::rules::parse_policy_gate(&crate::preserves_rail::value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn optional_capability_gate(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: &mut usize,
    arity: usize,
) -> crate::error::Result<Option<super::observed::CapabilityGateEvidence>> {
    if *cursor < arity && super::fields::value_has_record_label(&report[*cursor], "capability-gate-v1") {
        let parsed = super::grants::parse_capability_gate(&crate::preserves_rail::value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn optional_budget_gate(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: &mut usize,
    arity: usize,
) -> crate::error::Result<Option<super::observed::BudgetGateEvidence>> {
    if *cursor < arity && super::fields::value_has_record_label(&report[*cursor], "budget-gate-v1") {
        let parsed = super::spend::parse_budget_gate(&crate::preserves_rail::value_to_iovalue(&report[*cursor]))?;
        *cursor += 1;
        return Ok(Some(parsed));
    }
    Ok(None)
}

fn registry_after_gates(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: usize,
    suite: &super::model::Suite,
) -> crate::error::Result<(usize, Vec<super::model::ActorDecl>)> {
    let actors = super::roster::parse_actor_registry(&crate::preserves_rail::value_to_iovalue(&report[cursor]))?;
    if actors != suite.actors {
        return Err(crate::error::Failure::invalid_harness(
            "report actor registry does not match embedded suite actor registry",
        ));
    }
    Ok((cursor + 1, actors))
}

fn preflights_after_registry(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: usize,
    arity: usize,
) -> crate::error::Result<(usize, Option<super::observed::ExecutorPreflightsEvidence>)> {
    if cursor < arity && super::fields::value_has_record_label(&report[cursor], "executor-preflights-v1") {
        let parsed =
            super::clearance::parse_executor_preflights(&crate::preserves_rail::value_to_iovalue(&report[cursor]))?;
        return Ok((cursor + 1, Some(parsed)));
    }
    Ok((cursor, None))
}

fn observations_after(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: usize,
) -> crate::error::Result<(usize, Vec<super::observed::Observation>)> {
    let observation_values = super::fields::required_sequence(&report[cursor], "report observations")?;
    let mut observations = Vec::with_capacity(observation_values.len());
    for (position, observation) in observation_values.iter().enumerate() {
        let parsed = super::decode::parse_observation(&observation)?;
        if parsed.index != position as u64 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "observation index mismatch at position {position}: got {}",
                parsed.index
            )));
        }
        observations.push(parsed);
    }
    Ok((cursor + 1, observations))
}

fn log_and_budget(
    report: &preserves::Record<preserves::Value<preserves::IOValue>>,
    cursor: usize,
    suite: &super::model::Suite,
) -> crate::error::Result<(Vec<super::observed::EffectLogEntry>, super::model::BudgetEvidence)> {
    let effect_log_value = crate::preserves_rail::value_to_iovalue(&report[cursor]);
    let effect_log = super::tape::parse_effect_log(&effect_log_value)?;
    let budget_value = crate::preserves_rail::value_to_iovalue(&report[cursor + 1]);
    let budget = super::allowance::parse_budget(&budget_value)?;
    if budget.limits != suite.budget {
        return Err(crate::error::Failure::invalid_harness("report budget limits do not match embedded suite budget"));
    }
    Ok((effect_log, budget))
}

pub fn report_suite_value(report_value: &preserves::IOValue) -> crate::error::Result<preserves::IOValue> {
    Ok(parse_report(report_value)?.suite_value)
}
