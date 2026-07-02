
fn parse_capability_contract_envelope(value: &IoValue) -> Result<basalt::ContractEnvelope> {
    let envelope = simple_record(value, "contract-envelope", 7)?;
    let backend = required_string(&envelope[0], "capability contract backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "capability authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_string(&envelope[1], "capability contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let contract_version = required_string(&envelope[2], "capability contract version")?;
    if contract_version != CAPABILITY_CONTRACT_VERSION {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract version {contract_version}; expected {CAPABILITY_CONTRACT_VERSION}"
        )));
    }
    let normalized_source_hash = required_hash(&envelope[3], "capability contract normalized context ref")?;
    let input_schema = required_string(&envelope[4], "capability contract input schema")?;
    if input_schema != CAPABILITY_INPUT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract input schema {input_schema}; expected {CAPABILITY_INPUT_SCHEMA}"
        )));
    }
    let output_schema = required_string(&envelope[5], "capability contract output schema")?;
    if output_schema != crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capability contract output schema {output_schema}; expected {}",
            crate::preserves_rail::RUNTIME_CAPABILITY_AUTHORIZATION_SCHEMA
        )));
    }
    let receipt_schema_version = required_string(&envelope[6], "capability contract receipt schema")?;
    if receipt_schema_version != crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
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

fn parse_basalt_authority_preflight_evidence(value: &Value<IoValue>) -> Result<BasaltAuthorityPreflightEvidence> {
    let value = value_to_iovalue(value);
    let receipt = simple_record(&value, "basalt-authority-preflight", 9)?;
    let schema = required_string(&receipt[0], "Basalt authority preflight schema")?;
    if schema != crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_BASALT_AUTHORITY_PREFLIGHT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Basalt authority preflight decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight decision {decision}"
        )));
    }
    let backend = required_record_string(&receipt[2], "backend", "Basalt authority preflight backend")?;
    if backend != "nickel" {
        return Err(MoltenError::invalid_harness(format!(
            "Basalt authority preflight requires Nickel backend, got {backend}"
        )));
    }
    let contract_id = required_record_string(&receipt[3], "contract-id", "Basalt authority preflight contract id")?;
    if contract_id != CAPABILITY_CONTRACT_ID {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Basalt authority preflight contract id {contract_id}; expected {CAPABILITY_CONTRACT_ID}"
        )));
    }
    let envelope_ref = required_record_hash(&receipt[4], "envelope-ref", "Basalt authority preflight envelope ref")?;
    let capability_ref =
        required_record_hash(&receipt[5], "capability-ref", "Basalt authority preflight capability ref")?;
    let proofset_ref = required_record_hash(&receipt[6], "proofset-ref", "Basalt authority preflight proofset ref")?;
    let grant_refs = required_record_hash_sequence(&receipt[7], "grant-refs", "Basalt authority preflight grant refs")?;
    let reason = required_record_string(&receipt[8], "reason", "Basalt authority preflight reason")?;
    if reason != "accepted" {
        return Err(MoltenError::invalid_harness(format!("unsupported Basalt authority preflight reason {reason}")));
    }
    Ok(BasaltAuthorityPreflightEvidence {
        receipt_ref: canonical_hash(&value)?,
        envelope_ref,
        capability_ref,
        proofset_ref,
        grant_refs,
    })
}

fn parse_ucan_proofset_evidence(value: &Value<IoValue>) -> Result<UcanProofsetEvidence> {
    let value = value_to_iovalue(value);
    let proofset = simple_record(&value, "ucan-proofset-v1", 2)?;
    let schema = required_string(&proofset[0], "UCAN proofset schema")?;
    if schema != crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported UCAN proofset schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_UCAN_PROOFSET_SCHEMA
        )));
    }
    let proofs = required_sequence(&proofset[1], "UCAN proofset refs")?;
    if !proofs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "UCAN proof refs require Basalt/UCAN proof validation and are disabled in local harness capability gates",
        ));
    }
    Ok(UcanProofsetEvidence {
        proofset_ref: canonical_hash(&value)?,
    })
}

pub fn admission_authority_evidence(
    capabilities: &crate::runtime::CapabilityContext,
    request: &super::core::AdmissionRequest,
) -> Result<AdmissionAuthorityEvidence> {
    let authorization = capabilities.authorize(request);
    let grant_ref = authorization
        .grant
        .as_ref()
        .map(|grant| canonical_hash(&capability_grant_value(grant)))
        .transpose()?;
    Ok(AdmissionAuthorityEvidence {
        capability_ref: canonical_hash(&capabilities_value(capabilities))?,
        authorized: authorization.authorized,
        grant_ref,
    })
}

pub fn parse_capabilities(value: &IoValue) -> Result<crate::runtime::CapabilityContext> {
    let capabilities = simple_record(value, "capabilities-v1", 2)?;
    let schema = required_string(&capabilities[0], "capabilities schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported capabilities schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA
        )));
    }
    let grant_values = required_sequence(&capabilities[1], "capability grants")?;
    let mut grants = Vec::with_capacity(grant_values.len());
    for grant in grant_values.iter() {
        let grant_value = value_to_iovalue(&grant);
        let grant = simple_record(&grant_value, "grant", 4)?;
        grants.push(crate::runtime::CapabilityGrant {
            actor: optional_string(&grant[0], "capability grant actor")?,
            action: optional_action(&grant[1], "capability grant action")?,
            target: optional_string(&grant[2], "capability grant target")?,
            value: optional_runtime_match_value(&grant[3])?,
        });
    }
    Ok(crate::runtime::CapabilityContext::from_grants(grants))
}

fn capability_grant_value(grant: &crate::runtime::CapabilityGrant) -> IoValue {
    record("grant", vec![
        optional_policy_string(grant.actor.as_deref()),
        optional_policy_action(grant.action.as_ref()),
        optional_policy_string(grant.target.as_deref()),
        optional_policy_runtime_value(grant.value.as_ref()),
    ])
}

fn capability_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "capability-schema",
            "canonical-capability-context",
            "deny-by-default",
            "explicit-capability-fixture",
            "no-implicit-authority",
            "basalt-authority-preflight",
            "basalt-authority-receipt",
            "capability-proofset-binding",
            "grant-ref-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_capability_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "capability gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "capability gate check name")?;
        let status = required_string(&check[1], "capability gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("capability gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_capability_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("capability gate missing {expected} check")))
    }
}

fn deny_rule_value(rule: &crate::runtime::AdmissionDenyRule) -> IoValue {
    record("deny", vec![
        optional_policy_string(rule.actor.as_deref()),
        optional_policy_action(rule.action.as_ref()),
        optional_policy_string(rule.target.as_deref()),
        optional_policy_runtime_value(rule.value.as_ref()),
        string(&rule.reason),
    ])
}

fn optional_policy_string(value: Option<&str>) -> IoValue {
    value.map_or_else(|| bool_value(false), string)
}

fn optional_policy_action(value: Option<&crate::runtime::AdmissionAction>) -> IoValue {
    value.map_or_else(|| bool_value(false), |action| string(action.as_str()))
}

fn optional_policy_runtime_value(value: Option<&super::core::RuntimeValue>) -> IoValue {
    value.map_or_else(|| bool_value(false), |value| value.as_iovalue().clone())
}

fn policy_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "policy-schema",
            "canonical-policy-snapshot",
            "nickel-static-boundary",
            "nickel-policy-source",
            "nickel-export-normalization",
            "basalt-preflight",
            "basalt-receipt-binding",
            "steel-predicate-review",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_policy_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "policy gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "policy gate check name")?;
        let status = required_string(&check[1], "policy gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("policy gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_policy_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("policy gate missing {expected} check")))
    }
}
