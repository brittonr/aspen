pub struct PolicyPreflightMaterial {
    pub(super) policy_ref: String,
    pub(super) nickel_source_value: preserves::IOValue,
    pub(super) nickel_contract_value: preserves::IOValue,
    pub(super) basalt_preflight_value: preserves::IOValue,
}

pub struct NickelSourceEvidence {
    pub(super) source_ref: String,
    pub(super) export_ref: String,
    pub(super) policy_ref: String,
}

pub struct NickelContractEvidence {
    pub(super) envelope_ref: String,
    pub(super) normalized_source_ref: String,
}

pub struct BasaltPolicyPreflightEvidence {
    pub(super) receipt_ref: String,
    pub(super) envelope_ref: String,
    pub(super) policy_ref: String,
    pub(super) normalized_source_ref: String,
}

const POLICY_CONTRACT_ID: &str = "molten.harness.admission-policy";

const POLICY_CONTRACT_VERSION: &str = "v1";

const POLICY_INPUT_ID: &str = "molten.runtime.admission-request.v1";

pub(super) fn policy_preflight_material(
    policy: &crate::runtime::AdmissionPolicy,
) -> crate::error::Result<PolicyPreflightMaterial> {
    let policy_snapshot = super::rules::policy_value(policy);
    let policy_ref = crate::preserves_rail::canonical_hash(&policy_snapshot)?;
    let source = super::evaluator::nickel_policy_source(policy, &policy_ref)?;
    let source_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&source))?;
    let export_json = super::evaluator::nickel_export_json(&source)?;
    let export_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&export_json))?;
    let nickel_source_value = nickel_source_value(&source, &source_ref, &export_json, &export_ref, &policy_ref);

    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        POLICY_CONTRACT_ID,
        POLICY_CONTRACT_VERSION,
        source_ref.clone(),
        POLICY_INPUT_ID,
        crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA,
        crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt policy preflight denied Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    let nickel_contract_value = crate::preserves_rail::record("nickel-contract", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA),
        envelope_value,
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(&envelope_ref)]),
    ]);
    let basalt_preflight_value = crate::preserves_rail::record("basalt-preflight", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("backend", vec![crate::preserves_rail::string("nickel")]),
        crate::preserves_rail::record("contract-id", vec![crate::preserves_rail::string(POLICY_CONTRACT_ID)]),
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(envelope_ref)]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(&policy_ref)]),
        crate::preserves_rail::record("normalized-source-ref", vec![crate::preserves_rail::string(source_ref)]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(receipt.reason)]),
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
) -> preserves::IOValue {
    crate::preserves_rail::record("nickel-source", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(source)]),
        crate::preserves_rail::record("source-ref", vec![crate::preserves_rail::string(source_ref)]),
        crate::preserves_rail::record("export-json", vec![crate::preserves_rail::string(export_json)]),
        crate::preserves_rail::record("export-ref", vec![crate::preserves_rail::string(export_ref)]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(policy_ref)]),
    ])
}

pub(super) fn contract_envelope_value(envelope: &basalt::ContractEnvelope) -> preserves::IOValue {
    crate::preserves_rail::record("contract-envelope", vec![
        crate::preserves_rail::string(&envelope.backend),
        crate::preserves_rail::string(&envelope.contract_id),
        crate::preserves_rail::string(&envelope.contract_version),
        crate::preserves_rail::string(&envelope.normalized_source_hash),
        crate::preserves_rail::string(&envelope.input_schema),
        crate::preserves_rail::string(&envelope.output_schema),
        crate::preserves_rail::string(&envelope.receipt_schema_version),
    ])
}

pub(super) fn parse_nickel_source_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<NickelSourceEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let source = super::fields::simple_record(&value, "nickel-source", 6)?;
    let schema = super::fields::required_string(&source[0], "Nickel source schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Nickel source schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_NICKEL_STATIC_SCHEMA
        )));
    }
    let source_text = super::fields::required_record_string(&source[1], "source", "Nickel policy source")?;
    let source_ref = super::fields::required_record_hash(&source[2], "source-ref", "Nickel policy source ref")?;
    let actual_source_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&source_text))?;
    if source_ref != actual_source_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Nickel policy source ref mismatch: evidence has {source_ref}, source hashes to {actual_source_ref}"
        )));
    }
    let export_json = super::fields::required_record_string(&source[3], "export-json", "Nickel policy export JSON")?;
    let export_ref = super::fields::required_record_hash(&source[4], "export-ref", "Nickel policy export ref")?;
    let actual_export_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&export_json))?;
    if export_ref != actual_export_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Nickel policy export ref mismatch: evidence has {export_ref}, export hashes to {actual_export_ref}"
        )));
    }
    let actual_export = super::evaluator::nickel_export_json(&source_text)?;
    if actual_export != export_json {
        return Err(crate::error::Failure::invalid_harness(
            "Nickel policy export JSON does not match source normalization",
        ));
    }
    let policy_ref = super::fields::required_record_hash(&source[5], "policy-ref", "Nickel policy source policy ref")?;
    Ok(NickelSourceEvidence {
        source_ref,
        export_ref,
        policy_ref,
    })
}

pub(super) fn parse_nickel_contract_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<NickelContractEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let contract = super::fields::simple_record(&value, "nickel-contract", 3)?;
    let schema = super::fields::required_string(&contract[0], "Nickel contract schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Nickel contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = crate::preserves_rail::value_to_iovalue(&contract[1]);
    let envelope = parse_contract_envelope(&envelope_value)?;
    let envelope_ref =
        super::fields::required_record_hash(&contract[2], "envelope-ref", "Nickel contract envelope ref")?;
    let actual_envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Nickel contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt rejected Nickel contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(NickelContractEvidence {
        envelope_ref,
        normalized_source_ref: envelope.normalized_source_hash,
    })
}

fn parse_contract_envelope(value: &preserves::IOValue) -> crate::error::Result<basalt::ContractEnvelope> {
    let envelope = super::fields::simple_record(value, "contract-envelope", 7)?;
    let backend = super::fields::required_string(&envelope[0], "policy contract backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "policy preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = super::fields::required_string(&envelope[1], "policy contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let contract_version = super::fields::required_string(&envelope[2], "policy contract version")?;
    if contract_version != POLICY_CONTRACT_VERSION {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy contract version {contract_version}; expected {POLICY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = super::fields::required_hash(&envelope[3], "policy contract normalized source ref")?;
    let input_schema = super::fields::required_string(&envelope[4], "policy contract input schema")?;
    if input_schema != POLICY_INPUT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy contract input schema {input_schema}; expected {POLICY_INPUT_ID}"
        )));
    }
    let output_schema = super::fields::required_string(&envelope[5], "policy contract output schema")?;
    if output_schema != crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported policy contract output schema {output_schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA
        )));
    }
    let receipt_schema_version = super::fields::required_string(&envelope[6], "policy contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
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

pub(super) fn parse_basalt_policy_preflight_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<BasaltPolicyPreflightEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let receipt = super::fields::simple_record(&value, "basalt-preflight", 8)?;
    let schema = super::fields::required_string(&receipt[0], "Basalt policy preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt policy preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_POLICY_PREFLIGHT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "Basalt policy preflight decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt policy preflight decision {decision}"
        )));
    }
    let backend = super::fields::required_record_string(&receipt[2], "backend", "Basalt policy preflight backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt policy preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id =
        super::fields::required_record_string(&receipt[3], "contract-id", "Basalt policy preflight contract id")?;
    if contract_id != POLICY_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt policy preflight contract id {contract_id}; expected {POLICY_CONTRACT_ID}"
        )));
    }
    let envelope_ref =
        super::fields::required_record_hash(&receipt[4], "envelope-ref", "Basalt policy preflight envelope ref")?;
    let policy_ref =
        super::fields::required_record_hash(&receipt[5], "policy-ref", "Basalt policy preflight policy ref")?;
    let normalized_source_ref = super::fields::required_record_hash(
        &receipt[6],
        "normalized-source-ref",
        "Basalt policy preflight source ref",
    )?;
    let reason = super::fields::required_record_string(&receipt[7], "reason", "Basalt policy preflight reason")?;
    if reason != "accepted" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt policy preflight reason {reason}"
        )));
    }
    Ok(BasaltPolicyPreflightEvidence {
        receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
        envelope_ref,
        policy_ref,
        normalized_source_ref,
    })
}
