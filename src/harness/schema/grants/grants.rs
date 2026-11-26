use preserves::ValueImpl;

pub fn capabilities_value(capabilities: &crate::runtime::CapabilityContext) -> preserves::IOValue {
    crate::preserves_rail::record("capabilities-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA),
        crate::preserves_rail::sequence(capabilities.grants().iter().map(capability_grant_value).collect()),
    ])
}

pub fn capability_gate_value(
    capabilities: &crate::runtime::CapabilityContext,
) -> crate::error::Result<preserves::IOValue> {
    let preflight = super::warrant::capability_preflight_material(capabilities)?;
    Ok(crate::preserves_rail::record("capability-gate-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(&preflight.capability_ref)]),
        preflight.authority_contract_value,
        preflight.authority_preflight_value,
        preflight.proofset_value,
        capability_gate_checks_value(),
    ]))
}

pub fn parse_capability_gate(
    value: &preserves::IOValue,
) -> crate::error::Result<super::observed::CapabilityGateEvidence> {
    let gate = super::fields::simple_record(value, "capability-gate-v1", 7)?;
    let schema = super::fields::required_string(&gate[0], "capability gate schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capability gate schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITY_GATE_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&gate[1], "decision", "capability gate decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported capability gate decision {decision}")));
    }
    let capability_ref =
        super::fields::required_record_hash(&gate[2], "capability-ref", "capability gate capability ref")?;
    let authority_contract = super::warrant::parse_authority_contract_evidence(&gate[3])?;
    let authority_preflight = super::warrant::parse_basalt_authority_preflight_evidence(&gate[4])?;
    let proofset = super::proofs::parse_ucan_proofset_evidence(&gate[5])?;
    if authority_contract.normalized_capability_ref != capability_ref {
        return Err(crate::error::Failure::invalid_harness(
            "authority contract normalized capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.capability_ref != capability_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt authority preflight capability ref does not match capability gate ref",
        ));
    }
    if authority_preflight.envelope_ref != authority_contract.envelope_ref {
        return Err(crate::error::Failure::invalid_harness(
            "Basalt authority preflight envelope ref does not match authority contract envelope",
        ));
    }
    if authority_preflight.proofset_ref != proofset.proofset_ref {
        return Err(crate::error::Failure::invalid_harness(
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
    require_capability_gate_check(&checks, "ucan-verification-receipt-binding")?;
    require_capability_gate_check(&checks, "basalt-enforcement-receipt-binding")?;
    require_capability_gate_check(&checks, "grant-ref-binding")?;
    require_capability_gate_check(&checks, "derived-grant-ref-binding")?;
    require_capability_gate_check(&checks, "fixture-authority-evidence-only")?;
    let derived_grant_refs = if proofset.derived_grant_refs.is_empty() {
        authority_preflight.grant_refs.clone()
    } else {
        proofset.derived_grant_refs.clone()
    };
    Ok(super::observed::CapabilityGateEvidence {
        value: value.clone(),
        capability_ref,
        authority_preflight_ref: authority_preflight.receipt_ref,
        proofset_ref: proofset.proofset_ref,
        grant_refs: authority_preflight.grant_refs,
        ucan_verification_receipt_refs: proofset.verification_receipt_refs,
        derived_grant_refs,
        checks,
    })
}

pub fn validate_capability_gate_evidence(
    suite: &super::model::Suite,
    capability_gate: Option<&super::observed::CapabilityGateEvidence>,
) -> crate::error::Result<()> {
    if !suite.capabilities_explicit {
        return Err(crate::error::Failure::invalid_harness(
            "missing explicit capability fixture; implicit authority cannot satisfy evidence gates",
        ));
    }
    let capability_gate = capability_gate.ok_or_else(|| {
        crate::error::Failure::invalid_harness(
            "missing capability gate evidence; authority context must pass preflight before side effects",
        )
    })?;
    let expected_ref = crate::preserves_rail::canonical_hash(&capabilities_value(&suite.capabilities))?;
    require_capability_gate_check(&capability_gate.checks, "explicit-capability-fixture")?;
    require_capability_gate_check(&capability_gate.checks, "no-implicit-authority")?;
    if capability_gate.capability_ref != expected_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability gate ref mismatch: gate has {}, embedded capabilities hash to {expected_ref}",
            capability_gate.capability_ref
        )));
    }
    let expected_grant_refs = capability_grant_refs(&suite.capabilities)?;
    if capability_gate.grant_refs != expected_grant_refs {
        return Err(crate::error::Failure::invalid_harness(
            "capability gate grant refs do not match embedded capabilities",
        ));
    }
    if capability_gate.ucan_verification_receipt_refs.is_empty()
        && capability_gate.derived_grant_refs != expected_grant_refs
    {
        return Err(crate::error::Failure::invalid_harness(
            "local fixture derived grant refs do not match embedded capabilities",
        ));
    }
    let expected_gate = capability_gate_value(&suite.capabilities)?;
    let expected_gate_ref = crate::preserves_rail::canonical_hash(&expected_gate)?;
    let actual_gate_ref = crate::preserves_rail::canonical_hash(&capability_gate.value)?;
    if actual_gate_ref != expected_gate_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "capability gate evidence does not match embedded authority preflight: gate hashes to {actual_gate_ref}, expected {expected_gate_ref}"
        )));
    }
    Ok(())
}

pub(super) fn capability_grant_refs(
    capabilities: &crate::runtime::CapabilityContext,
) -> crate::error::Result<Vec<String>> {
    capabilities
        .grants()
        .iter()
        .map(|grant| crate::preserves_rail::canonical_hash(&capability_grant_value(grant)))
        .collect()
}

pub fn parse_capabilities(value: &preserves::IOValue) -> crate::error::Result<crate::runtime::CapabilityContext> {
    let capabilities = super::fields::simple_record(value, "capabilities-v1", 2)?;
    let schema = super::fields::required_string(&capabilities[0], "capabilities schema")?;
    if schema != crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported capabilities schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_CAPABILITIES_SCHEMA
        )));
    }
    let grant_values = super::fields::required_sequence(&capabilities[1], "capability grants")?;
    let mut grants = Vec::with_capacity(grant_values.len());
    for grant in grant_values.iter() {
        let grant_value = crate::preserves_rail::value_to_iovalue(&grant);
        let grant = super::fields::simple_record(&grant_value, "grant", 4)?;
        grants.push(crate::runtime::CapabilityGrant {
            actor: super::fields::optional_string(&grant[0], "capability grant actor")?,
            action: super::fields::optional_action(&grant[1], "capability grant action")?,
            target: super::fields::optional_string(&grant[2], "capability grant target")?,
            value: super::fields::optional_runtime_match_value(&grant[3])?,
        });
    }
    Ok(crate::runtime::CapabilityContext::from_grants(grants))
}

pub(super) fn capability_grant_value(grant: &crate::runtime::CapabilityGrant) -> preserves::IOValue {
    crate::preserves_rail::record("grant", vec![
        super::rules::optional_policy_string(grant.actor.as_deref()),
        super::rules::optional_policy_action(grant.action.as_ref()),
        super::rules::optional_policy_string(grant.target.as_deref()),
        super::rules::optional_policy_runtime_value(grant.value.as_ref()),
    ])
}

fn capability_gate_checks_value() -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        [
            "capability-schema",
            "canonical-capability-context",
            "deny-by-default",
            "explicit-capability-fixture",
            "no-implicit-authority",
            "basalt-authority-preflight",
            "basalt-authority-receipt",
            "capability-proofset-binding",
            "ucan-verification-receipt-binding",
            "basalt-enforcement-receipt-binding",
            "grant-ref-binding",
            "derived-grant-ref-binding",
            "fixture-authority-evidence-only",
        ]
        .iter()
        .map(|name| {
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(*name),
                crate::preserves_rail::string("pass"),
            ])
        })
        .collect(),
    )])
}

fn parse_capability_gate_checks(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks_record = super::fields::simple_record(&value, "checks", 1)?;
    let check_values = super::fields::required_sequence(&checks_record[0], "capability gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(&check_value);
        let check = super::fields::simple_record(&check_value, "check", 2)?;
        let name = super::fields::required_string(&check[0], "capability gate check name")?;
        let status = super::fields::required_string(&check[1], "capability gate check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!(
                "capability gate check {name} status is {status}"
            )));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_capability_gate_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("capability gate missing {expected} check")))
    }
}
