
fn nickel_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => escaped.push_str(&format!("\\u{{{:x}}}", character as u32)),
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn nickel_export_json(source: &str) -> Result<String> {
    let mut context = nickel_lang::Context::new();
    let expression = context.eval_deep_for_export(source).map_err(nickel_error)?;
    context.expr_to_json(&expression).map_err(nickel_error)
}

fn nickel_error(error: nickel_lang::Error) -> MoltenError {
    let mut message = Vec::new();
    if error.format(&mut message, nickel_lang::ErrorFormat::Text).is_ok() {
        MoltenError::invalid_harness(format!(
            "Nickel static policy normalization failed: {}",
            String::from_utf8_lossy(&message).trim()
        ))
    } else {
        MoltenError::invalid_harness(format!("Nickel static policy normalization failed: {error:?}"))
    }
}

pub fn capabilities_value(capabilities: &crate::runtime::CapabilityContext) -> IoValue {
    record("capabilities-v1", vec![
        string(crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA),
        sequence(capabilities.grants().iter().map(capability_grant_value).collect()),
    ])
}

pub fn capability_gate_value(capabilities: &crate::runtime::CapabilityContext) -> Result<IoValue> {
    let preflight = capability_preflight_material(capabilities)?;
    Ok(record("capability-gate-v1", vec![
        string(crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA),
        record("decision", vec![string("pass")]),
        record("capability-ref", vec![string(&preflight.capability_ref)]),
        preflight.authority_contract_value,
        preflight.authority_preflight_value,
        preflight.proofset_value,
        capability_gate_checks_value(),
    ]))
}

pub fn parse_capability_gate(value: &IoValue) -> Result<CapabilityGateEvidence> {
    let gate = simple_record(value, "capability-gate-v1", 7)?;
    let schema = required_string(&gate[0], "capability gate schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA
        )));
    }
    let decision = required_record_string(&gate[1], "decision", "capability gate decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported capability gate decision {decision}")));
    }
    let capability_ref = required_record_hash(&gate[2], "capability-ref", "capability gate capability ref")?;
    let authority_contract = parse_authority_contract_evidence(&gate[3])?;
    let authority_preflight = parse_basalt_authority_preflight_evidence(&gate[4])?;
    let proofset = parse_ucan_proofset_evidence(&gate[5])?;
    if authority_contract.normalized_capability_ref != capability_ref {
        return Err(MoltenError::invalid_harness(
            "authority contract normalized capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.capability_ref != capability_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.envelope_ref != authority_contract.envelope_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight envelope ref does not match authority contract envelope",
        ));
    }
    if authority_preflight.proofset_ref != proofset.proofset_ref {
        return Err(MoltenError::invalid_harness(
            "Basalt authority preflight proofset ref does not match UCAN proofset evidence",
        ));
    }
    let checks = parse_capability_gate_checks(&gate[6])?;
    require_capability_gate_check(&checks, "capability-schema")?;
    require_capability_gate_check(&checks, "canonical-capability-context")?;
    require_capability_gate_check(&checks, "deny-by-default")?;
    require_capability_gate_check(&checks, "explicit-capability-fixture")?;
    require_capability_gate_check(&checks, "no-implicit-authority")?;
    require_capability_gate_check(&checks, "basalt-authority-preflight")?;
    require_capability_gate_check(&checks, "basalt-authority-receipt")?;
    require_capability_gate_check(&checks, "capability-proofset-binding")?;
    require_capability_gate_check(&checks, "grant-ref-binding")?;
    Ok(CapabilityGateEvidence {
        value: value.clone(),
        capability_ref,
        authority_preflight_ref: authority_preflight.receipt_ref,
        proofset_ref: proofset.proofset_ref,
        grant_refs: authority_preflight.grant_refs,
        checks,
    })
}

pub fn validate_capability_gate_evidence(
    suite: &Suite,
    capability_gate: Option<&CapabilityGateEvidence>,
) -> Result<()> {
    if !suite.capabilities_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit capability fixture; implicit authority cannot satisfy evidence gates",
        ));
    }
    let capability_gate = capability_gate.ok_or_else(|| {
        MoltenError::invalid_harness(
            "missing capability gate evidence; authority context must pass preflight before side effects",
        )
    })?;
    let expected_ref = canonical_hash(&capabilities_value(&suite.capabilities))?;
    require_capability_gate_check(&capability_gate.checks, "explicit-capability-fixture")?;
    require_capability_gate_check(&capability_gate.checks, "no-implicit-authority")?;
    if capability_gate.capability_ref != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "capability gate ref mismatch: gate has {}, embedded capabilities hash to {expected_ref}",
            capability_gate.capability_ref
        )));
    }
    let expected_grant_refs = capability_grant_refs(&suite.capabilities)?;
    if capability_gate.grant_refs != expected_grant_refs {
        return Err(MoltenError::invalid_harness("capability gate grant refs do not match embedded capabilities"));
    }
    let expected_gate = capability_gate_value(&suite.capabilities)?;
    let expected_gate_ref = canonical_hash(&expected_gate)?;
    let actual_gate_ref = canonical_hash(&capability_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(MoltenError::invalid_harness(format!(
            "capability gate evidence does not match embedded authority preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

struct CapabilityPreflightMaterial {
    capability_ref: String,
    authority_contract_value: IoValue,
    authority_preflight_value: IoValue,
    proofset_value: IoValue,
}

struct AuthorityContractEvidence {
    envelope_ref: String,
    normalized_capability_ref: String,
}

struct BasaltAuthorityPreflightEvidence {
    receipt_ref: String,
    envelope_ref: String,
    capability_ref: String,
    proofset_ref: String,
    grant_refs: Vec<String>,
}

struct UcanProofsetEvidence {
    proofset_ref: String,
}

const CAPABILITY_CONTRACT_ID: &str = "molten.harness.capability-context";
const CAPABILITY_CONTRACT_VERSION: &str = "v1";
const CAPABILITY_INPUT_SCHEMA: &str = "molten.runtime.admission-request.v1";

fn capability_preflight_material(
    capabilities: &crate::runtime::CapabilityContext,
) -> Result<CapabilityPreflightMaterial> {
    let capability_snapshot = capabilities_value(capabilities);
    let capability_ref = canonical_hash(&capability_snapshot)?;
    let grant_refs: Vec<String> = capability_grant_refs(capabilities)?;
    let proofset_value = ucan_proofset_value();
    let proofset_ref = canonical_hash(&proofset_value)?;
    let envelope = basalt::ContractEnvelope::new(
        "nickel",
        CAPABILITY_CONTRACT_ID,
        CAPABILITY_CONTRACT_VERSION,
        capability_ref.clone(),
        CAPABILITY_INPUT_SCHEMA,
        crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA,
        crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA,
    );
    let envelope_value = contract_envelope_value(&envelope);
    let envelope_ref = canonical_hash(&envelope_value)?;
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt authority preflight denied capability contract envelope: {}",
            receipt.reason
        )));
    }
    let authority_contract_value = record("authority-contract", vec![
        string(crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA),
        envelope_value,
        record("envelope-ref", vec![string(&envelope_ref)]),
    ]);
    let mut grant_ref_values = Vec::with_capacity(grant_refs.len());
    for grant_ref in &grant_refs {
        grant_ref_values.push(string(grant_ref.as_str()));
    }
    let authority_preflight_value = record("basalt-authority-preflight", vec![
        string(crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("backend", vec![string("nickel")]),
        record("contract-id", vec![string(CAPABILITY_CONTRACT_ID)]),
        record("envelope-ref", vec![string(envelope_ref)]),
        record("capability-ref", vec![string(&capability_ref)]),
        record("proofset-ref", vec![string(proofset_ref)]),
        record("grant-refs", vec![sequence(grant_ref_values)]),
        record("reason", vec![string(receipt.reason)]),
    ]);
    Ok(CapabilityPreflightMaterial {
        capability_ref,
        authority_contract_value,
        authority_preflight_value,
        proofset_value,
    })
}

fn capability_grant_refs(capabilities: &crate::runtime::CapabilityContext) -> Result<Vec<String>> {
    capabilities.grants().iter().map(|grant| canonical_hash(&capability_grant_value(grant))).collect()
}

fn ucan_proofset_value() -> IoValue {
    record("ucan-proofset-v1", vec![
        string(crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA),
        sequence(Vec::new()),
    ])
}

fn parse_authority_contract_evidence(value: &Value<IoValue>) -> Result<AuthorityContractEvidence> {
    let value = value_to_iovalue(value);
    let contract = simple_record(&value, "authority-contract", 3)?;
    let schema = required_string(&contract[0], "authority contract schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported authority contract schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITY_CONTRACT_SCHEMA
        )));
    }
    let envelope_value = value_to_iovalue(&contract[1]);
    let envelope = parse_capability_contract_envelope(&envelope_value)?;
    let envelope_ref = required_record_hash(&contract[2], "envelope-ref", "authority contract envelope ref")?;
    let actual_envelope_ref = canonical_hash(&envelope_value)?;
    if envelope_ref != actual_envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "authority contract envelope ref mismatch: evidence has {envelope_ref}, envelope hashes to {actual_envelope_ref}"
        )));
    }
    let receipt = basalt::validate_contract_envelope(&envelope);
    if !receipt.is_accepted() {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt rejected authority contract envelope: {}",
            receipt.reason
        )));
    }
    Ok(AuthorityContractEvidence {
        envelope_ref,
        normalized_capability_ref: envelope.normalized_source_hash,
    })
}
