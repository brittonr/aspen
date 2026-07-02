
fn validate_turn_journal_verify_receipt(
    value: &IoValue,
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    payload_refs: &[String],
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("turn journal missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "turn journal verify receipt schema")?;
    if schema != EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported turn journal verify receipt schema {schema}; expected {EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "turn journal verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "turn journal verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = required_chain_scope(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness("turn journal verify receipt chain scope mismatch"));
    }
    let anchor = required_record_optional_hash(&receipt[3], "anchor", "turn journal anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("turn journal verify receipt missing anchor"))?;
    let expected_head = required_record_optional_hash(&receipt[4], "expected-head", "turn journal expected head")?
        .ok_or_else(|| MoltenError::invalid_harness("turn journal verify receipt missing expected head"))?;
    if Some(&anchor) != link_refs.first() || Some(&expected_head) != link_refs.last() {
        return Err(MoltenError::invalid_harness("turn journal verify receipt does not bind actor-local anchor/head"));
    }
    if required_record_hash_sequence(&receipt[5], "discovered-heads")? != vec![expected_head] {
        return Err(MoltenError::invalid_harness("turn journal verify receipt discovered head mismatch"));
    }
    if required_record_hash_sequence(&receipt[6], "verified-links")? != link_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt link range mismatch"));
    }
    if required_record_hash_sequence(&receipt[7], "payloads")? != payload_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt payload refs mismatch"));
    }
    if required_record_hash_sequence(&receipt[8], "predicates")? != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt predicate refs mismatch"));
    }
    Ok(())
}

fn require_context_ref(
    context_refs: &[crate::evidence_chain::ChainContextRef],
    label: &str,
    expected: &str,
) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label && context.artifact_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("turn journal link missing {label} context ref {expected}")))
    }
}

fn require_context_ref_kind(context_refs: &[crate::evidence_chain::ChainContextRef], label: &str) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("turn journal link missing {label} context ref")))
    }
}

fn repro_verify_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "sealed-bundle",
            "embedded-report",
            "embedded-gate-receipt",
            "report-validation",
            "deterministic-replay",
            "gate-receipt-recomputed",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn checks_value() -> IoValue {
    record("checks", vec![sequence(
        PASS_CHECKS.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

struct CoreRefs<'a> {
    validation: &'a ValidationReceipt,
    replay: &'a ReplayReceipt,
    report: &'a str,
    suite: &'a str,
    final_state: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationReceipt {
    report_ref: String,
    suite_ref: String,
    final_state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayReceipt {
    expected_report_ref: String,
    actual_report_ref: String,
    final_state_hash: String,
    verify_ref: String,
}

fn parse_validation(value: &Value<IoValue>) -> Result<ValidationReceipt> {
    let value = value_to_iovalue(value);
    let validation = simple_record(&value, "validation", 7)?;
    let status = required_record_string(&validation[0], "status", "gate validation status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate validation status {status}")));
    }
    let report_ref = required_record_hash(&validation[1], "report", "gate validation report ref")?;
    let suite_ref = required_record_hash(&validation[2], "suite", "gate validation suite ref")?;
    let final_state_hash = required_record_hash(&validation[3], "final-state", "gate validation final state hash")?;
    let observations = required_record_u64(&validation[4], "observations", "gate validation observations")?;
    super::schema::parse_actor_registry(&value_to_iovalue(&validation[5]))?;
    let budget = super::schema::parse_budget(&value_to_iovalue(&validation[6]))?;
    if observations != budget.usage.steps {
        return Err(MoltenError::invalid_harness(
            "gate receipt validation observation count does not match budget step usage",
        ));
    }
    Ok(ValidationReceipt {
        report_ref,
        suite_ref,
        final_state_hash,
    })
}

fn parse_replay(value: &Value<IoValue>) -> Result<ReplayReceipt> {
    let value = value_to_iovalue(value);
    let replay = simple_record(&value, "replay", 6)?;
    let status = required_record_string(&replay[0], "status", "gate replay status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate replay status {status}")));
    }
    let expected_report_ref = required_record_hash(&replay[1], "expected-report", "gate replay expected report ref")?;
    let actual_report_ref = required_record_hash(&replay[2], "actual-report", "gate replay actual report ref")?;
    let final_state_hash = required_record_hash(&replay[3], "final-state", "gate replay final state hash")?;
    let verify_ref = required_record_hash(&replay[4], "verify-ref", "gate replay verify ref")?;
    let verify_value = value_to_iovalue(&replay[5]);
    validate_harness_replay_verify_value(
        &verify_value,
        &verify_ref,
        &expected_report_ref,
        &actual_report_ref,
        &final_state_hash,
    )?;
    Ok(ReplayReceipt {
        expected_report_ref,
        actual_report_ref,
        final_state_hash,
        verify_ref,
    })
}

fn validate_harness_replay_verify_value(
    value: &IoValue,
    expected_verify_ref: &str,
    expected_report_ref: &str,
    actual_report_ref: &str,
    final_state_hash: &str,
) -> Result<()> {
    let actual_verify_ref = canonical_hash(value)?;
    if actual_verify_ref != expected_verify_ref {
        return Err(MoltenError::invalid_harness("gate replay verify ref does not match embedded value"));
    }
    let receipt = simple_record(value, "deterministic-replay-verify-v1", 7)?;
    let schema = required_string(&receipt[0], "deterministic replay verify schema")?;
    if schema != DETERMINISTIC_REPLAY_VERIFY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic replay verify schema {schema}; expected {DETERMINISTIC_REPLAY_VERIFY_SCHEMA}"
        )));
    }
    let decision = required_string(&receipt[1], "deterministic replay verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic replay verify decision {decision}"
        )));
    }
    let verify_expected_report =
        required_record_hash(&receipt[2], "expected-report-ref", "deterministic replay expected report")?;
    let verify_actual_report =
        required_record_hash(&receipt[3], "actual-report-ref", "deterministic replay actual report")?;
    let verify_final_state = required_record_hash(&receipt[4], "final-state-ref", "deterministic replay final state")?;
    let divergence = required_record_string(&receipt[5], "divergence", "deterministic replay divergence")?;
    if divergence != "none" {
        return Err(MoltenError::invalid_harness(format!(
            "deterministic replay verify divergence must be none, got {divergence}"
        )));
    }
    let checks = parse_checks(&receipt[6])?;
    require_check(&checks, "report-replayed")?;
    require_check(&checks, "final-state-bound")?;
    require_check(&checks, "no-divergence")?;
    if verify_expected_report != expected_report_ref
        || verify_actual_report != actual_report_ref
        || verify_final_state != final_state_hash
    {
        return Err(MoltenError::invalid_harness("deterministic replay verify refs do not match gate replay refs"));
    }
    Ok(())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "gate check name")?;
        let status = required_string(&check[1], "gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt missing {expected} check")))
    }
}

fn require_all_checks(checks: &[String]) -> Result<()> {
    for expected in PASS_CHECKS.iter().copied() {
        require_check(checks, expected)?;
    }
    Ok(())
}

fn require_core_refs(input: &CoreRefs<'_>) -> Result<()> {
    if input.report != input.validation.report_ref
        || input.report != input.replay.expected_report_ref
        || input.report != input.replay.actual_report_ref
    {
        return Err(MoltenError::invalid_harness("gate receipt report refs are inconsistent"));
    }
    if input.suite != input.validation.suite_ref {
        return Err(MoltenError::invalid_harness("gate receipt suite refs are inconsistent"));
    }
    if input.final_state != input.validation.final_state_hash || input.final_state != input.replay.final_state_hash {
        return Err(MoltenError::invalid_harness("gate receipt final state refs are inconsistent"));
    }
    Ok(())
}

fn require_link_context(
    link: &crate::evidence_chain::ChainLink,
    report_ref: &str,
    suite_ref: &str,
    final_state_hash: &str,
) -> Result<()> {
    if link.payload.artifact_ref != report_ref {
        return Err(MoltenError::invalid_harness("gate chain evidence payload does not bind the gate report ref"));
    }
    if !link
        .context_refs
        .iter()
        .any(|context| context.label == "suite" && context.artifact_ref == suite_ref)
    {
        return Err(MoltenError::invalid_harness("gate chain evidence context does not bind the gate suite ref"));
    }
    if !link
        .context_refs
        .iter()
        .any(|context| context.label == "final-state" && context.artifact_ref == final_state_hash)
    {
        return Err(MoltenError::invalid_harness("gate chain evidence context does not bind the gate final state ref"));
    }
    Ok(())
}
