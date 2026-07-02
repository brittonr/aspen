
pub fn validate_budget_gate_evidence(suite: &Suite, budget_gate: Option<&BudgetGateEvidence>) -> Result<()> {
    if !suite.budget_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit budget fixture; default resource policy cannot satisfy evidence gates",
        ));
    }
    let budget_gate = budget_gate.ok_or_else(|| {
        MoltenError::invalid_harness(
            "missing budget gate evidence; resource policy must pass preflight before side effects",
        )
    })?;
    let expected_ref = canonical_hash(&budget_limits_value(&suite.budget))?;
    if budget_gate.budget_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "budget gate ref mismatch: gate has {}, embedded budget hashes to {expected_ref}",
            budget_gate.budget_ref
        )));
    }
    let expected_gate = budget_gate_value(&suite.budget)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&budget_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "budget gate evidence does not match embedded resource preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct BudgetPreflightMaterial {
    budget_ref: String,
    nickel_source_value: IoValue,
    resource_contract_value: IoValue,
    resource_preflight_value: IoValue,
}

struct BudgetNickelSourceEvidence {
    source_ref: String,
    export_ref: String,
    budget_ref: String,
}

struct ResourceContractEvidence {
    envelope_ref: String,
    normalized_budget_ref: String,
}

struct BasaltResourcePreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    budget_ref: String,
    normalized_source_ref: String,
}

const BUDGET_CONTRACT_ID: &str = "molten.harness.resource-budget";
const BUDGET_CONTRACT_VERSION: &str = "v1";

fn budget_preflight_material(budget: &Budget) -> Result<BudgetPreflightMaterial> {
    let budget_snapshot = budget_limits_value(budget);
    let budget_ref = canonical_hash(&budget_snapshot)?;
    let source = nickel_budget_source(budget, &budget_ref);
    let source_ref = canonical_hash(&string(&source))?;
    let export_json = nickel_export_json(&source)?;
    let export_ref = canonical_hash(&string(&export_json))?;
    let nickel_source_value = budget_nickel_source_value(&source, &source_ref, &export_json, &export_ref, &budget_ref);
    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        BUDGET_CONTRACT_ID,
        BUDGET_CONTRACT_VERSION,
        source_ref.clone(),
        crate::preserves_rail::HARNESS_BUDGET_SCHEMA,
        crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA,
        crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt resource preflight denied budget contract envelope: {}",
            receipt.reason
        )));
    }
    let resource_contract_value = record("resource-contract", vec![
        string(crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let resource_preflight_value = record("basalt-resource-preflight", vec![
        string(crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(BUDGET_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("budget-ref", vec![string(&budget_ref)]),
        record("normalized-source-ref", vec![string(source_ref)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(BudgetPreflightMaterial {
        budget_ref,
        nickel_source_value,
        resource_contract_value,
        resource_preflight_value,
    })
}

fn budget_nickel_source_value(
    source: &str,
    source_ref: &str,
    export_json: &str,
    export_ref: &str,
    budget_ref: &str,
) -> IoValue {
    record("budget-source", vec![
        string(crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        record("source", vec![string(source)]),
        record("source-ref", vec![string(source_ref)]),
        record("export-json", vec![string(export_json)]),
        record("export-ref", vec![string(export_ref)]),
        record("budget-ref", vec![string(budget_ref)]),
    ])
}

fn parse_budget_nickel_source_evidence(value: &Value<IoValue>) -> Result<BudgetNickelSourceEvidence> {
    let value = value_to_iovalue(value);
    let source = simple_record(&value, "budget-source", 6)?;
    let schema = required_string(&source[0], "Nickel resource policy schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel resource policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA
        )));
    }
    let source_text = required_record_string(&source[1], "source", "Nickel resource policy source")?;
    let source_ref = required_record_hash(&source[2], "source-ref", "Nickel resource policy source ref")?;
    let actual_source_ref = canonical_hash(&string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel resource policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json = required_record_string(&source[3], "export-json", "Nickel resource policy export JSON")?;
    let export_ref = required_record_hash(&source[4], "export-ref", "Nickel resource policy export ref")?;
    let actual_export_ref = canonical_hash(&string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel resource policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(MoltenError::invalid_harness(
            "Nickel resource policy export JSON does not match source normalization",
        ));
    }
    let budget_ref = required_record_hash(&source[5], "budget-ref", "Nickel resource policy budget ref")?;
    Ok(BudgetNickelSourceEvidence {
        source_ref,
        export_ref,
        budget_ref,
    })
}

fn parse_resource_contract_evidence(value: &Value<IoValue>) -> Result<ResourceContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "resource-contract", 3)?;
    let schema = required_string(&contract[0], "resource contract schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported resource contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_budget_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "resource contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "resource contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected resource contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(ResourceContractEvidence {
        envelope_ref,
        normalized_budget_ref: envelope.normalized_source_hash,
    })
}

fn parse_budget_contract_envelope(value: &IoValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "budget contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!("resource preflight requires Nickel backend, got {backend}")));
    }
    let contract_id = required_string(&envelope[1], "budget contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "budget contract version")?;
    if contract_version != BUDGET_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract version {contract_version}; expected {BUDGET_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "budget contract normalized source ref")?;
    let input_schema = required_string(&envelope[4], "budget contract input schema")?;
    if input_schema != crate::preserves_rail::HARNESS_BUDGET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract input schema {input_schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_SCHEMA
        )));
    }
    let output_schema = required_string(&envelope[5], "budget contract output schema")?;
    if output_schema != crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract output schema {output_schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "budget contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported budget contract receipt schema {receipt_schema_version}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA
        )));
    }
    Ok(basalt::ContractEnvelope::new(
        backend,
        contract_id,
        contract_version,
        normalized_source_hash,
        input_schema,
        output_schema,
        receipt_schema_version,
    ))
}

fn parse_basalt_resource_preflight_evidence(value: &Value<IoValue>) -> Result<BasaltResourcePreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-resource-preflight", 8)?;
    let schema = required_string(&receipt[0], "Basalt resource preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt resource preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt resource preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt resource preflight decision {decision}")));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt resource preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt resource preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt resource preflight contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt resource preflight contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt resource preflight envelope ref")?;
    let budget_ref = required_record_hash(&receipt[5], "budget-ref", "Basalt resource preflight budget ref")?;
    let normalized_source_ref =
        required_record_hash(&receipt[6], "normalized-source-ref", "Basalt resource preflight source ref")?;
    let reason = required_record_string(&receipt[7], "reason", "Basalt resource preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt resource preflight reason {reason}")));
    }
    Ok(BasaltResourcePreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        budget_ref,
        normalized_source_ref,
    })
}
