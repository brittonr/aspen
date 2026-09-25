
fn validate_checks(value: &preserves::Value<IoValue>, decision: &str) -> Result<()> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks = crate::preserves_rail::simple_record_fields(&value, "checks", 1)?;
    let entries = crate::preserves_rail::required_sequence_field(&checks[0], "UCAN verification check sequence")?;
    let mut is_saw_decision = false;
    for entry in entries.as_ref() {
        let entry_value = crate::preserves_rail::value_to_iovalue(entry);
        let check = crate::preserves_rail::simple_record_fields(&entry_value, "check", 2)?;
        let name = required_string(&check[0], "UCAN verification check name")?;
        let status = required_string(&check[1], "UCAN verification check status")?;
        if name == "decision-bound" {
            is_saw_decision = status == decision;
        } else if decision == DECISION_PASS && status != CHECK_STATUS_PASS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "passing UCAN verification receipt has failing check {name}"
            )));
        }
    }
    if is_saw_decision {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("UCAN verification receipt missing decision-bound check"))
    }
}

fn admission_value(
    decision: &str,
    diagnostics: &[String],
    admitted_token_refs: &[String],
    request: &CapabilityRequest,
) -> IoValue {
    crate::preserves_rail::record("capability-admission-receipt-v1", vec![
        crate::preserves_rail::string(CAPABILITY_ADMISSION_SCHEMA),
        field("decision", decision),
        field("holder-ref", &request.holder_ref),
        field("session-ref", &request.session_ref),
        field("resource-ref", &request.resource_ref),
        field("ability", &request.ability),
        field("scope", &request.scope),
        string_list_field("admitted-token-refs", admitted_token_refs),
        string_list_field("diagnostics", diagnostics),
        field("evidence-only", "capability-admission-does-not-grant-subsystem-trust"),
    ])
}

fn validate_request_refs(request: &CapabilityRequest) -> Result<()> {
    validate_refs([
        request.holder_ref.as_str(),
        request.session_ref.as_str(),
        request.context_ref.as_str(),
        request.resource_ref.as_str(),
    ])
}

fn validate_refs<'a>(refs: impl IntoIterator<Item = &'a str>) -> Result<()> {
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn push_mismatch(diagnostics: &mut impl crate::bounded::VecSink<String>, label: &str, actual: &str, expected: &str) {
    if actual != expected {
        diagnostics.push_item(format!("{label} mismatch expected {expected} actual {actual}"));
    }
}

fn record_string(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::record_string_field(value, label, label)
}

fn record_ref(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::record_content_ref_string(value, label, label)
}

fn record_string_sequence(value: &preserves::Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = crate::preserves_rail::simple_record_fields(&value, label, 1)?;
    let sequence = crate::preserves_rail::required_sequence_field(&record[0], label)?;
    sequence
        .as_ref()
        .iter()
        .map(|entry| crate::preserves_rail::required_string_field(entry, label))
        .collect()
}

fn record_ref_sequence(value: &preserves::Value<IoValue>, label: &str) -> Result<Vec<String>> {
    crate::preserves_rail::record_content_ref_strings(value, label, label, MAX_UCAN_RECEIPT_REFS)
}

fn require_string(value: &preserves::Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported {label} {actual}; expected {expected}"
        )))
    }
}

fn required_string(value: &preserves::Value<IoValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(value, label)
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::string(value)])
}

fn string_list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(
        values.iter().map(crate::preserves_rail::string).collect(),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TICK: u64 = 8;
    const EXPIRED_TICK: u64 = 9;

    #[test]
    fn capability_admission_accepts_scoped_token() {
        let proofset = proofset(token("write-token", "publish", "topic:alerts", VALID_TICK));
        let receipt = admit_capability(&proofset, &request("publish", "topic:alerts", VALID_TICK)).expect("admission");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));
        assert!(capability_taxonomy().contains(&"handoff-bundle"));
    }

    #[test]
    fn capability_admission_denies_wrong_holder_expiry_and_missing_caveat() {
        let mut proofset = proofset(token("write-token", "publish", "topic:alerts", VALID_TICK));
        proofset.tokens[0].holder_ref = test_ref("other-holder");
        proofset.tokens[0].caveats = vec!["mfa".to_string()];
        let receipt =
            admit_capability(&proofset, &request("publish", "topic:alerts", EXPIRED_TICK)).expect("admission");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("holder mismatch")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("expired")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("caveat unsatisfied")));
    }

    #[test]
    fn ucan_verification_receipt_binds_request_and_derived_grants() {
        let input = ucan_input(UcanVerificationChecks::all_pass());
        let receipt = ucan_verification_receipt(&input).expect("UCAN verification receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));
        let parsed = parse_ucan_verification_receipt_value(&receipt.value).expect("parse UCAN receipt");
        assert_eq!(parsed.request_ref, input.request_ref);
        assert_eq!(parsed.proof_refs, input.proof_refs);
        assert_eq!(parsed.derived_grant_refs, input.derived_grant_refs);
    }

    #[test]
    fn ucan_verification_receipt_denies_invalid_signature_holder_replay_and_missing_proofs() {
        let mut checks = UcanVerificationChecks::all_pass();
        checks.signature_valid = false;
        checks.holder_matches = false;
        checks.replay_fresh = false;
        let mut input = ucan_input(checks);
        input.proof_refs.clear();
        let receipt = ucan_verification_receipt(&input).expect("UCAN denial receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("signature-valid")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("holder-bound")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("replay-fresh")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("proof refs are required")));
    }

    #[test]
    fn imported_token_is_not_operation_authority() {
        let denial = imported_token_authority_denial(&test_ref("token"), "node-control").expect("denial");
        assert_eq!(denial.decision, "deny");
        assert!(denial.diagnostics[0].contains("evidence-only"));
    }

    fn ucan_input(checks: UcanVerificationChecks) -> UcanVerificationInput {
        UcanVerificationInput {
            compact_token_ref: test_ref("compact-token"),
            proofset_ref: test_ref("proofset"),
            proof_refs: vec![test_ref("proof")],
            verification_key_refs: vec![test_ref("key")],
            caveat_decision_refs: vec![test_ref("caveat")],
            revocation_fact_refs: vec![test_ref("revocation")],
            replay_fact_refs: vec![test_ref("replay")],
            derived_grant_refs: vec![test_ref("derived-grant")],
            request_ref: test_ref("request"),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: "publish".to_string(),
            scope: "topic:alerts".to_string(),
            checks,
        }
    }

    fn proofset(token: CapabilityToken) -> CapabilityProofset {
        CapabilityProofset {
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            tokens: vec![token],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource-policy")],
            revocation_refs: Vec::new(),
            evidence_refs: vec![test_ref("evidence")],
        }
    }

    fn token(kind: &str, ability: &str, scope: &str, expires_at_tick: u64) -> CapabilityToken {
        CapabilityToken {
            token_kind: kind.to_string(),
            issuer_ref: test_ref("issuer"),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: ability.to_string(),
            scope: scope.to_string(),
            attenuation: "attenuated".to_string(),
            caveats: Vec::new(),
            expires_at_tick,
            revoked_refs: Vec::new(),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource-policy")],
            delegation_refs: vec![test_ref("delegation")],
            evidence_refs: vec![test_ref("token-evidence")],
        }
    }

    fn request(ability: &str, scope: &str, at_tick: u64) -> CapabilityRequest {
        CapabilityRequest {
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            resource_ref: test_ref("resource"),
            ability: ability.to_string(),
            scope: scope.to_string(),
            at_tick,
            required_policy_refs: vec![test_ref("policy")],
            required_resource_refs: vec![test_ref("resource-policy")],
            required_token_kind: Some("write-token".to_string()),
            caveat_context: Vec::new(),
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("capability-test-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }
}
