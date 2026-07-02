
fn validate_chain_scope(chain: &ChainScope) -> Result<()> {
    require_non_empty(&chain.scope, "chain scope")?;
    require_non_empty(&chain.id, "chain id")?;
    require_non_empty(&chain.epoch, "chain epoch")
}

fn validate_producer(producer: &ChainProducer) -> Result<()> {
    require_non_empty(&producer.id, "producer id")?;
    require_ref(&producer.key_ref, "producer key ref")
}

fn validate_chain_predicate_receipt_shape(receipt: &ChainPredicateReceipt) -> Result<()> {
    require_ref(&receipt.receipt_ref, "chain predicate receipt ref")?;
    require_non_empty(&receipt.predicate, "chain predicate name")?;
    match receipt.decision.as_str() {
        "pass" | "fail" | "retained" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported chain predicate decision {other}"))),
    }
    for subject_ref in &receipt.subject_refs {
        require_ref(subject_ref, "chain predicate subject ref")?;
    }
    for input_ref in &receipt.input_refs {
        require_ref(input_ref, "chain predicate input ref")?;
    }
    for context_ref in &receipt.context_refs {
        require_ref(context_ref, "chain predicate context ref")?;
    }
    require_pass_check_in(&receipt.checks, "trellis-bounded-predicate")
        .or_else(|_| require_pass_check_in(&receipt.checks, "segment-contiguity"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "fork-policy-profile"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "anchor-descent"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "checkpoint-range-coverage"))
}

fn validate_chain_fork_evidence_shape(fork: &ChainForkEvidence) -> Result<()> {
    require_ref(&fork.evidence_ref, "chain fork evidence ref")?;
    validate_chain_scope(&fork.chain)?;
    if let Some(parent_ref) = &fork.parent_ref {
        require_ref(parent_ref, "fork parent ref")?;
    }
    if fork.child_refs.len() < 2 {
        return Err(MoltenError::invalid_harness("fork evidence must name at least two child/head refs"));
    }
    for child_ref in &fork.child_refs {
        require_ref(child_ref, "fork child ref")?;
    }
    if let Some(selected_head) = &fork.selected_head {
        require_ref(selected_head, "fork selected head ref")?;
    }
    match fork.profile.as_str() {
        "reject-unexpected-forks" | "retain-fork-evidence" => {}
        other => {
            return Err(MoltenError::invalid_harness(format!("unsupported fork policy profile {other}")));
        }
    }
    match fork.decision.as_str() {
        "reject" | "retain" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported fork decision {other}"))),
    }
    require_pass_check_in(&fork.checks, "fork-detected")?;
    require_pass_check_in(&fork.checks, "fork-policy-profile")?;
    require_pass_check_in(&fork.checks, "diagnostic-retention")
}

fn validate_chain_anchor_shape(anchor: &ChainAnchor) -> Result<()> {
    require_ref(&anchor.anchor_ref, "chain anchor ref")?;
    validate_chain_scope(&anchor.chain)?;
    require_ref(&anchor.link_ref, "anchor link ref")?;
    for policy_ref in &anchor.policy_refs {
        require_ref(policy_ref, "anchor policy ref")?;
    }
    validate_producer(&anchor.producer)?;
    require_pass_check_in(&anchor.checks, "trusted-anchor")?;
    require_pass_check_in(&anchor.checks, "anchor-link-available")
}

fn validate_chain_checkpoint_shape(checkpoint: &ChainCheckpoint) -> Result<()> {
    require_ref(&checkpoint.checkpoint_ref, "chain checkpoint ref")?;
    validate_chain_scope(&checkpoint.chain)?;
    if let Some(prior_checkpoint_ref) = &checkpoint.prior_checkpoint_ref {
        require_ref(prior_checkpoint_ref, "prior checkpoint ref")?;
    }
    require_ref(&checkpoint.anchor_link_ref, "checkpoint anchor link ref")?;
    require_ref(&checkpoint.head_ref, "checkpoint head ref")?;
    require_ref(&checkpoint.verify_receipt_ref, "checkpoint verify receipt ref")?;
    require_ref(&checkpoint.range_predicate_ref, "checkpoint range predicate ref")?;
    for policy_ref in &checkpoint.policy_refs {
        require_ref(policy_ref, "checkpoint policy ref")?;
    }
    for membership_ref in &checkpoint.membership_refs {
        require_ref(membership_ref, "checkpoint membership ref")?;
    }
    validate_producer(&checkpoint.producer)?;
    require_pass_check_in(&checkpoint.checks, "raft-control-plane-command")?;
    require_pass_check_in(&checkpoint.checks, "verified-range")?;
    require_pass_check_in(&checkpoint.checks, "checkpoint-freshness")
}

fn require_trellis_pass(link: &ChainLink, expected_predicate: &str) -> Result<()> {
    if link.trellis.predicate != expected_predicate {
        return Err(MoltenError::invalid_harness(format!(
            "chain link trellis predicate {} does not match expected {expected_predicate}",
            link.trellis.predicate
        )));
    }
    if link.trellis.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chain link trellis decision must be pass, got {}",
            link.trellis.decision
        )));
    }
    Ok(())
}

fn require_pass_check(link: &ChainLink, name: &str) -> Result<()> {
    require_pass_check_in(&link.checks, name)
        .map_err(|_| MoltenError::invalid_harness(format!("chain link missing pass check {name}")))
}

fn require_input_pass_check(input: &ChainCheckpointInput, name: &str) -> Result<()> {
    require_pass_check_in(&input.checks, name)
        .map_err(|_| MoltenError::invalid_harness(format!("chain checkpoint missing pass check {name}")))
}

fn require_pass_check_in(checks: &[ChainCheck], name: &str) -> Result<()> {
    if checks.iter().any(|check| check.name == name && check.decision == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("missing pass check {name}")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {field}")))?;
    required_string(&record[0], field)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> optional ref")))?;
    let value = value_to_iovalue(&record[0]);
    if value.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = value.collect_simple_record("some", Some(1)) {
        required_string(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <some ref> or <none> for {label}")))
    }
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> ref sequence")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    sequence.iter().map(|value| required_string(value, label)).collect()
}

fn record_u64(value: &Value<IoValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {field}")))?;
    required_u64(&record[0], field)
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {field} {actual}; expected {expected}")))
    }
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn require_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_ref(value: &str, field: &str) -> Result<()> {
    require_non_empty(value, field)?;
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("unsupported {field} {value}; expected canonical content ref: {error}"))
    })
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/tests/m000/p003/body.rs"));
}
