
pub fn parse_policy_gate(value: &IoValue) -> Result<PolicyGateEvidence> {
    let gate = simple_record(value, "policy-gate-v1", 8)?;
    let schema = required_string(&gate[0], "policy gate schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_GATE_SCHEMA
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "policy gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported policy gate decision {decision}")));
    }
    let policy_ref = required_record_hash(&gate[2], "policy-ref", "policy gate policy ref")?;
    let nickel_source = parse_nickel_source_evidence(&gate[3])?;
    let nickel_contract = parse_nickel_contract_evidence(&gate[4])?;
    let basalt_preflight = parse_basalt_policy_preflight_evidence(&gate[5])?;
    if nickel_source.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Nickel source policy ref does not match policy gate ref"));
    }
    if nickel_contract.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Nickel contract normalized source ref does not match Nickel source evidence",
        ));
    }
    if basalt_preflight.policy_ref != policy_ref {
        return Err(MoltenError::invalid_harness("Basalt policy preflight policy ref does not match policy gate ref"));
    }
    if basalt_preflight.envelope_ref != nickel_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight envelope ref does not match Nickel contract envelope",
        ));
    }
    if basalt_preflight.normalized_source_ref != nickel_source.source_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt policy preflight source ref does not match Nickel source evidence",
        ));
    }
    let steel_predicates = required_record_sequence(&gate[6], "steel-predicates", "policy gate Steel predicates")?;
    if !steel_predicates.is_empty() {
        return Err(MoltenError::invalid_harness(
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
    Ok(PolicyGateEvidence {
        value: value.clone(),
        policy_ref,
        nickel_source_ref: nickel_source.source_ref,
        nickel_export_ref: nickel_source.export_ref,
        basalt_preflight_ref: basalt_preflight.receipt_ref,
        checks,
    })
}

pub fn validate_policy_gate_evidence(suite: &Suite, policy_gate: Option<&PolicyGateEvidence>) -> Result<()> {
    let policy_gate = policy_gate.ok_or_else(|| {
        MoltenError::invalid_harness("missing policy gate evidence; policy must pass preflight before side effects")
    })?;
    let expected_ref = canonical_hash(&policy_value(&suite.policy))?;
    if policy_gate.policy_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate ref mismatch: gate has {}, embedded policy hashes to {expected_ref}",
            policy_gate.policy_ref
        )));
    }
    let expected_gate = policy_gate_value(&suite.policy)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&policy_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "policy gate evidence does not match embedded suite policy preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct PolicyPreflightMaterial {
    policy_ref: String,
    nickel_source_value: IoValue,
    nickel_contract_value: IoValue,
    basalt_preflight_value: IoValue,
}

struct NickelSourceEvidence {
    source_ref: String,
    export_ref: String,
    policy_ref: String,
}
