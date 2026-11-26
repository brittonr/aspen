pub struct CapabilityPreflightMaterial {
    pub(super) capability_ref: String,
    pub(super) authority_contract_value: preserves::IOValue,
    pub(super) authority_preflight_value: preserves::IOValue,
    pub(super) proofset_value: preserves::IOValue,
}

pub struct AuthorityContractEvidence {
    pub(super) envelope_ref: String,
    pub(super) normalized_capability_ref: String,
}

pub struct BasaltAuthorityPreflightEvidence {
    pub(super) receipt_ref: String,
    pub(super) envelope_ref: String,
    pub(super) capability_ref: String,
    pub(super) proofset_ref: String,
    pub(super) grant_refs: Vec<String>,
}

const CAPABILITY_CONTRACT_ID: &str = "molten.harness.capability-context";

const CAPABILITY_CONTRACT_VERSION: &str = "v1";

const CAPABILITY_INPUT_ID: &str = "molten.runtime.admission-request.v1";

pub(super) fn capability_preflight_material(
    capabilities: &crate::runtime::CapabilityContext,
) -> crate::error::Result<CapabilityPreflightMaterial> {
    let capability_snapshot = super::grants::capabilities_value(capabilities);
    let capability_ref = crate::preserves_rail::canonical_hash(&capability_snapshot)?;
    let grant_refs: Vec<String> = super::grants::capability_grant_refs(capabilities)?;
    let proofset_value = super::proofs::ucan_proofset_value();
    let proofset_ref = crate::preserves_rail::canonical_hash(&proofset_value)?;
    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        CAPABILITY_CONTRACT_ID,
        CAPABILITY_CONTRACT_VERSION,
        capability_ref.clone(),
        CAPABILITY_INPUT_ID,
        crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA,
        crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = super::covenant::contract_envelope_value(&envelope);
    let envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt authority preflight denied capability contract envelope: {}",
            receipt.reason
        )));
    }
    let authority_contract_value = crate::preserves_rail::record("authority-contract", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA),
        envelope_value,
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(&envelope_ref)]),
    ]);
    let mut grant_ref_values = Vec::with_capacity(grant_refs.len());
    for grant_ref in &grant_refs {
        grant_ref_values.push(crate::preserves_rail::string(grant_ref.as_str()));
    }
    let authority_preflight_value = crate::preserves_rail::record("basalt-authority-preflight", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("backend", vec![crate::preserves_rail::string("nickel")]),
        crate::preserves_rail::record("contract-id", vec![crate::preserves_rail::string(CAPABILITY_CONTRACT_ID)]),
        crate::preserves_rail::record("envelope-ref", vec![crate::preserves_rail::string(envelope_ref)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(&capability_ref)]),
        crate::preserves_rail::record("proofset-ref", vec![crate::preserves_rail::string(proofset_ref)]),
        crate::preserves_rail::record("grant-refs", vec![crate::preserves_rail::sequence(grant_ref_values)]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(receipt.reason)]),
    ]);
    Ok(CapabilityPreflightMaterial {
        capability_ref,
        authority_contract_value,
        authority_preflight_value,
        proofset_value,
    })
}

pub(super) fn parse_authority_contract_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<AuthorityContractEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let contract = super::fields::simple_record(&value, "authority-contract", 3)?;
    let schema = super::fields::required_string(&contract[0], "authority contract schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported authority contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = crate::preserves_rail::value_to_iovalue(&contract[1]);
    let envelope = parse_capability_contract_envelope(&envelope_value)?;
    let envelope_ref =
        super::fields::required_record_hash(&contract[2], "envelope-ref", "authority contract envelope ref")?;
    let actual_envelope_ref = crate::preserves_rail::canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "authority contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt rejected authority contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(AuthorityContractEvidence {
        envelope_ref,
        normalized_capability_ref: envelope.normalized_source_hash,
    })
}

fn parse_capability_contract_envelope(value: &preserves::IOValue) -> crate::error::Result<basalt::ContractEnvelope> {
    let envelope = super::fields::simple_record(value, "contract-envelope", 7)?;
    let backend = super::fields::required_string(&envelope[0], "capability contract backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = super::fields::required_string(&envelope[1], "capability contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let contract_version = super::fields::required_string(&envelope[2], "capability contract version")?;
    if contract_version != CAPABILITY_CONTRACT_VERSION {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability contract version {contract_version}; expected {CAPABILITY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash =
        super::fields::required_hash(&envelope[3], "capability contract normalized context ref")?;
    let input_schema = super::fields::required_string(&envelope[4], "capability contract input schema")?;
    if input_schema != CAPABILITY_INPUT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability contract input schema {input_schema}; expected {CAPABILITY_INPUT_ID}"
        )));
    }
    let output_schema = super::fields::required_string(&envelope[5], "capability contract output schema")?;
    if output_schema != crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability contract output schema {output_schema}; expected {}",
            crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA
        )));
    }
    let receipt_schema_version = super::fields::required_string(&envelope[6], "capability contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability contract receipt schema {receipt_schema_version}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA
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

pub(super) fn parse_basalt_authority_preflight_evidence(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<BasaltAuthorityPreflightEvidence> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let receipt = super::fields::simple_record(&value, "basalt-authority-preflight", 9)?;
    let schema = super::fields::required_string(&receipt[0], "Basalt authority preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt authority preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA
        )));
    }
    let decision =
        super::fields::required_record_string(&receipt[1], "decision", "Basalt authority preflight decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt authority preflight decision {decision}"
        )));
    }
    let backend = super::fields::required_record_string(&receipt[2], "backend", "Basalt authority preflight backend")?;
    if backend != "nickel" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Basalt authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id =
        super::fields::required_record_string(&receipt[3], "contract-id", "Basalt authority preflight contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt authority preflight contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let envelope_ref =
        super::fields::required_record_hash(&receipt[4], "envelope-ref", "Basalt authority preflight envelope ref")?;
    let capability_ref = super::fields::required_record_hash(
        &receipt[5],
        "capability-ref",
        "Basalt authority preflight capability ref",
    )?;
    let proofset_ref =
        super::fields::required_record_hash(&receipt[6], "proofset-ref", "Basalt authority preflight proofset ref")?;
    let grant_refs = super::fields::required_record_hash_sequence(
        &receipt[7],
        "grant-refs",
        "Basalt authority preflight grant refs",
    )?;
    let reason = super::fields::required_record_string(&receipt[8], "reason", "Basalt authority preflight reason")?;
    if reason != "accepted" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Basalt authority preflight reason {reason}"
        )));
    }
    Ok(BasaltAuthorityPreflightEvidence {
        receipt_ref: crate::preserves_rail::canonical_hash(&value)?,
        envelope_ref,
        capability_ref,
        proofset_ref,
        grant_refs,
    })
}
