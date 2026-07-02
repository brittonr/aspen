
struct NickelContractEvidence {
    envelope_ref: String,
    normalized_source_ref: String,
}

struct BasaltPolicyPreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    policy_ref: String,
    normalized_source_ref: String,
}

const POLICY_CONTRACT_ID: &str = "molten.harness.admission-policy";
const POLICY_CONTRACT_VERSION: &str = "v1";
const POLICY_INPUT_SCHEMA: &str = "molten.runtime.admission-request.v1";

fn policy_preflight_material(policy: &crate::runtime::AdmissionPolicy) -> Result<PolicyPreflightMaterial> {
    let policy_snapshot = policy_value(policy);
    let policy_ref = canonical_hash(&policy_snapshot)?;
    let source = nickel_policy_source(policy, &policy_ref)?;
    let source_ref = canonical_hash(&string(&source))?;
    let export_json = nickel_export_json(&source)?;
    let export_ref = canonical_hash(&string(&export_json))?;
    let nickel_source_value = nickel_source_value(&source, &source_ref, &export_json, &export_ref, &policy_ref);

    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        POLICY_CONTRACT_ID,
        POLICY_CONTRACT_VERSION,
        source_ref.clone(),
        POLICY_INPUT_SCHEMA,
        crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA,
        crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt policy preflight denied Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    let nickel_contract_value = record("nickel-contract", vec![
        string(crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let basalt_preflight_value = record("basalt-preflight", vec![
        string(crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(POLICY_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("policy-ref", vec![string(&policy_ref)]),
        record("normalized-source-ref", vec![string(source_ref)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(PolicyPreflightMaterial {
        policy_ref,
        nickel_source_value,
        nickel_contract_value,
        basalt_preflight_value,
    })
}

fn nickel_source_value(
    source: &str,
    source_ref: &str,
    export_json: &str,
    export_ref: &str,
    policy_ref: &str,
) -> IoValue {
    record("nickel-source", vec![
        string(crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA),
        record("source", vec![string(source)]),
        record("source-ref", vec![string(source_ref)]),
        record("export-json", vec![string(export_json)]),
        record("export-ref", vec![string(export_ref)]),
        record("policy-ref", vec![string(policy_ref)]),
    ])
}

fn contract_envelope_value(envelope: &basalt::ContractEnvelope) -> IoValue {
    record("contract-envelope", vec![
        string(&envelope.backend),
        string(&envelope.contract_id),
        string(&envelope.contract_version),
        string(&envelope.normalized_source_hash),
        string(&envelope.input_schema),
        string(&envelope.output_schema),
        string(&envelope.receipt_schema_version),
    ])
}

fn parse_nickel_source_evidence(value: &Value<IoValue>) -> Result<NickelSourceEvidence> {
    let value = value_to_iovalue(value);
    let source = simple_record(&value, "nickel-source", 6)?;
    let schema = required_string(&source[0], "Nickel source schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel source schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA
        )));
    }
    let source_text = required_record_string(&source[1], "source", "Nickel policy source")?;
    let source_ref = required_record_hash(&source[2], "source-ref", "Nickel policy source ref")?;
    let actual_source_ref = canonical_hash(&string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json = required_record_string(&source[3], "export-json", "Nickel policy export JSON")?;
    let export_ref = required_record_hash(&source[4], "export-ref", "Nickel policy export ref")?;
    let actual_export_ref = canonical_hash(&string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(MoltenError::invalid_harness("Nickel policy export JSON does not match source normalization"));
    }
    let policy_ref = required_record_hash(&source[5], "policy-ref", "Nickel policy source policy ref")?;
    Ok(NickelSourceEvidence {
        source_ref,
        export_ref,
        policy_ref,
    })
}

fn parse_nickel_contract_evidence(value: &Value<IoValue>) -> Result<NickelContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "nickel-contract", 3)?;
    let schema = required_string(&contract[0], "Nickel contract schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Nickel contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "Nickel contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nickel contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(NickelContractEvidence {
        envelope_ref,
        normalized_source_ref: envelope.normalized_source_hash,
    })
}

fn parse_contract_envelope(value: &IoValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "policy contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!("policy preflight requires Nickel backend, got {backend}")));
    }
    let contract_id = required_string(&envelope[1], "policy contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "policy contract version")?;
    if contract_version != POLICY_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract version {contract_version}; expected {POLICY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "policy contract normalized source ref")?;
    let input_schema = required_string(&envelope[4], "policy contract input schema")?;
    if input_schema != POLICY_INPUT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract input schema {input_schema}; expected {POLICY_INPUT_SCHEMA}"
        )));
    }
    let output_schema = required_string(&envelope[5], "policy contract output schema")?;
    if output_schema != crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract output schema {output_schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "policy contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy contract receipt schema {receipt_schema_version}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA
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

fn parse_basalt_policy_preflight_evidence(value: &Value<IoValue>) -> Result<BasaltPolicyPreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-preflight", 8)?;
    let schema = required_string(&receipt[0], "Basalt policy preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt policy preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt policy preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt policy preflight decision {decision}")));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt policy preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt policy preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt policy preflight contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt policy preflight contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt policy preflight envelope ref")?;
    let policy_ref = required_record_hash(&receipt[5], "policy-ref", "Basalt policy preflight policy ref")?;
    let normalized_source_ref =
        required_record_hash(&receipt[6], "normalized-source-ref", "Basalt policy preflight source ref")?;
    let reason = required_record_string(&receipt[7], "reason", "Basalt policy preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt policy preflight reason {reason}")));
    }
    Ok(BasaltPolicyPreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        policy_ref,
        normalized_source_ref,
    })
}

fn nickel_policy_source(policy: &crate::runtime::AdmissionPolicy, policy_ref: &str) -> Result<String> {
    let mut source = String::from("{\n");
    source.push_str(&format!(
        "  schema_version = {},\n",
        nickel_string(crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA)
    ));
    source.push_str(&format!("  policy_schema = {},\n", nickel_string(crate::preserves_rail::HARNESS_POLICY_SCHEMA)));
    source.push_str(&format!("  policy_ref = {},\n", nickel_string(policy_ref)));
    source.push_str("  deny_rules = [\n");
    for rule in policy.deny_rules() {
        source.push_str("    {\n");
        source.push_str(&format!("      actor = {},\n", nickel_optional_string(rule.actor.as_deref())));
        source.push_str(&format!(
            "      action = {},\n",
            nickel_optional_string(rule.action.as_ref().map(crate::runtime::AdmissionAction::as_str))
        ));
        source.push_str(&format!("      target = {},\n", nickel_optional_string(rule.target.as_deref())));
        source.push_str(&format!("      value = {},\n", nickel_optional_runtime_value(rule.value.as_ref())?));
        source.push_str(&format!("      reason = {},\n", nickel_string(&rule.reason)));
        source.push_str("    },\n");
    }
    source.push_str("  ],\n}");
    Ok(source)
}

fn nickel_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), nickel_string)
}

fn nickel_optional_runtime_value(value: Option<&super::core::RuntimeValue>) -> Result<String> {
    match value {
        Some(value) => {
            let text = to_text(value.as_iovalue())?;
            let value_ref = canonical_hash(value.as_iovalue())?;
            Ok(format!("{{ preserves = {}, ref = {} }}", nickel_string(&text), nickel_string(&value_ref)))
        }
        None => Ok("null".to_string()),
    }
}
