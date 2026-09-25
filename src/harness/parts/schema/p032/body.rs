
fn parse_runtime_predicate_receipt(value: &IoValue) -> Result<String> {
    let receipt = value
        .collect_simple_record("runtime-predicate-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <runtime-predicate-receipt-v1 ...>"))?;
    let schema = required_string(&receipt[0], "runtime predicate receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate receipt schema {schema}")));
    }
    let predicate = required_string(&receipt[1], "runtime predicate name")?;
    if !matches!(
        predicate.as_str(),
        TURN_COMMIT_ROLLBACK_PREDICATE
            | ASSERTION_VISIBILITY_PREDICATE
            | OBSERVE_DELIVERY_PREDICATE
            | PRESERVES_PATTERN_PREDICATE
            | PROMISE_STATE_PREDICATE
            | PROMISE_PIPELINE_PREDICATE
            | REVOCATION_CLEANUP_PREDICATE
            | ACTORMAP_TRANSACTION_PREDICATE
            | NEAR_FAR_REFS_PREDICATE
            | SNAPSHOT_AUTHORITY_PREDICATE
            | SERVICE_DEPENDENCIES_PREDICATE
    ) {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported runtime predicate receipt predicate {predicate}"
        )));
    }
    let engine = required_string(&receipt[2], "runtime predicate engine")?;
    if engine != RUNTIME_PREDICATE_ENGINE {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate engine {engine}")));
    }
    required_record_hash(&receipt[3], "input-ref", "runtime predicate input ref")?;
    let decision = required_string(&receipt[4], "runtime predicate decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate decision {decision}")));
    }
    let state_refs = sequence_strings(&receipt[5], "runtime predicate state refs")?;
    if state_refs.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing state refs"));
    }
    for state_ref in &state_refs {
        validate_content_ref(state_ref)?;
    }
    let checks = sequence_strings(&receipt[6], "runtime predicate checks")?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing checks"));
    }
    sequence_strings(&receipt[7], "runtime predicate diagnostics")?;
    Ok(predicate)
}

fn sequence_strings(value: &Value<IoValue>, field: &str) -> Result<Vec<String>> {
    let values = required_sequence(value, field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

#[derive(Clone, Copy)]
struct BoundaryEvidence<'a> {
    suite: &'a Suite,
    policy_gate: &'a PolicyGateEvidence,
    capability_gate: &'a CapabilityGateEvidence,
    budget_gate: &'a BudgetGateEvidence,
}
