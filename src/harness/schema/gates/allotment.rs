pub struct BudgetPreflightMaterial {
    pub(super) budget_ref: String,
    pub(super) nickel_source_value: preserves::IOValue,
    pub(super) resource_contract_value: preserves::IOValue,
    pub(super) resource_preflight_value: preserves::IOValue,
}

pub struct BudgetNickelSourceEvidence {
    pub(super) source_ref: String,
    pub(super) export_ref: String,
    pub(super) budget_ref: String,
}

pub struct ResourceContractEvidence {
    pub(super) envelope_ref: String,
    pub(super) normalized_budget_ref: String,
}

pub struct BasaltResourcePreflightEvidence {
    pub(super) receipt_ref: String,
    pub(super) envelope_ref: String,
    pub(super) budget_ref: String,
    pub(super) normalized_source_ref: String,
}

const BUDGET_CONTRACT_ID: &str = "molten.harness.resource-budget";

const BUDGET_CONTRACT_VERSION: &str = "v1";

pub(super) fn budget_preflight_material(
    budget: &super::model::Budget,
) -> crate::error::Result<BudgetPreflightMaterial> {
    let budget_snapshot = super::allowance::budget_limits_value(budget);
    let budget_ref = crate::preserves_rail::canonical_hash(&budget_snapshot)?;
    let source = super::spend::nickel_budget_source(budget, &budget_ref);
    let source_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&source))?;
    let export_json = super::evaluator::nickel_export_json(&source)?;
    let export_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&export_json))?;
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
    let envelope_value = super::covenant::contract_envelope_value(&envelope);
    let envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt resource preflight denied budget contract envelope: {}",
            receipt.reason
        )));
    }
    let resource_contract_value = crate::preserves_rail::record("resource-contract", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA),
        envelope_value,
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(&envelope_ref)]),
    ]);
    let resource_preflight_value = crate::preserves_rail::record("basalt-resource-preflight", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("backend", vec![crate::preserves_rail::string("nickel")]),
        crate::preserves_rail::record("contract-id", vec![crate::preserves_rail::string(BUDGET_CONTRACT_ID)]),
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(envelope_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(&budget_ref)]),
        crate::preserves_rail::record("normalized-source-ref", vec![crate::preserves_rail::string(source_ref)]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(receipt.reason)]),
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
) -> preserves::IOValue {
    crate::preserves_rail::record("budget-source", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(source)]),
        crate::preserves_rail::record("source-ref", vec![crate::preserves_rail::string(source_ref)]),
        crate::preserves_rail::record("export-json", vec![crate::preserves_rail::string(export_json)]),
        crate::preserves_rail::record("export-ref", vec![crate::preserves_rail::string(export_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(budget_ref)]),
    ])
}

pub(super) fn parse_budget_nickel_source_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<BudgetNickelSourceEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let source = super::fields::simple_record(&value, "budget-source", 6)?;
    let schema = super::fields::required_string(&source[0], "Nickel resource policy schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Nickel resource policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA
        )));
    }
    let source_text = super::fields::required_record_string(&source[1], "source", "Nickel resource policy source")?;
    let source_ref =
        super::fields::required_record_hash(&source[2], "source-ref", "Nickel resource policy source ref")?;
    let actual_source_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Nickel resource policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json =
        super::fields::required_record_string(&source[3], "export-json", "Nickel resource policy export JSON")?;
    let export_ref =
        super::fields::required_record_hash(&source[4], "export-ref", "Nickel resource policy export ref")?;
    let actual_export_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Nickel resource policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = super::evaluator::nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(crate::error::Failure::invalid_harness(
            "Nickel resource policy export JSON does not match source normalization",
        ));
    }
    let budget_ref =
        super::fields::required_record_hash(&source[5], "budget-ref", "Nickel resource policy budget ref")?;
    Ok(BudgetNickelSourceEvidence {
        source_ref,
        export_ref,
        budget_ref,
    })
}

pub(super) fn parse_resource_contract_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<ResourceContractEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let contract = super::fields::simple_record(&value, "resource-contract", 3)?;
    let schema = super::fields::required_string(&contract[0], "resource contract schema")?;
    if schema != crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported resource contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = crate::preserves_rail::value_to_iovalue(&contract[1]);
    let envelope = parse_budget_contract_envelope(&envelope_value)?;
    let envelope_ref =
        super::fields::required_record_hash(&contract[2], "envelope-ref", "resource contract envelope ref")?;
    let actual_envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "resource contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt rejected resource contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(ResourceContractEvidence {
        envelope_ref,
        normalized_budget_ref: envelope.normalized_source_hash,
    })
}

fn parse_budget_contract_envelope(value: &preserves::IOValue) -> crate::error::Result<basalt::ContractEnvelope> {
    let envelope = super::fields::simple_record(value, "contract-envelope", 7)?;
    let backend = super::fields::required_string(&envelope[0], "budget contract backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "resource preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = super::fields::required_string(&envelope[1], "budget contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let contract_version = super::fields::required_string(&envelope[2], "budget contract version")?;
    if contract_version != BUDGET_CONTRACT_VERSION {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget contract version {contract_version}; expected {BUDGET_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = super::fields::required_hash(&envelope[3], "budget contract normalized source ref")?;
    let input_schema = super::fields::required_string(&envelope[4], "budget contract input schema")?;
    if input_schema != crate::preserves_rail::HARNESS_BUDGET_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget contract input schema {input_schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_SCHEMA
        )));
    }
    let output_schema = super::fields::required_string(&envelope[5], "budget contract output schema")?;
    if output_schema != crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported budget contract output schema {output_schema}; expected {}",
            crate::preserves_rail::HARNESS_BUDGET_USAGE_SCHEMA
        )));
    }
    let receipt_schema_version = super::fields::required_string(&envelope[6], "budget contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
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

pub(super) fn parse_basalt_resource_preflight_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<BasaltResourcePreflightEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let receipt = super::fields::simple_record(&value, "basalt-resource-preflight", 8)?;
    let schema = super::fields::required_string(&receipt[0], "Basalt resource preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt resource preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_RESOURCE_PREFLIGHT_SCHEMA
        )));
    }
    let decision =
        super::fields::required_record_string(&receipt[1], "decision", "Basalt resource preflight decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt resource preflight decision {decision}"
        )));
    }
    let backend = super::fields::required_record_string(&receipt[2], "backend", "Basalt resource preflight backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt resource preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id =
        super::fields::required_record_string(&receipt[3], "contract-id", "Basalt resource preflight contract id")?;
    if contract_id != BUDGET_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt resource preflight contract id {contract_id}; expected {BUDGET_CONTRACT_ID}"
        )));
    }
    let envelope_ref =
        super::fields::required_record_hash(&receipt[4], "envelope-ref", "Basalt resource preflight envelope ref")?;
    let budget_ref =
        super::fields::required_record_hash(&receipt[5], "budget-ref", "Basalt resource preflight budget ref")?;
    let normalized_source_ref = super::fields::required_record_hash(
        &receipt[6],
        "normalized-source-ref",
        "Basalt resource preflight source ref",
    )?;
    let reason = super::fields::required_record_string(&receipt[7], "reason", "Basalt resource preflight reason")?;
    if reason != "accepted" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt resource preflight reason {reason}"
        )));
    }
    Ok(BasaltResourcePreflightEvidence {
        receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
        envelope_ref,
        budget_ref,
        normalized_source_ref,
    })
}
