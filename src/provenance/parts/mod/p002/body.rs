
pub fn summary(value: &IoValue) -> Result<String> {
    if let Ok(record) = parse_record(value) {
        return Ok(format!(
            "provenance record artifact={} trust_state={} build_records={} record={}",
            record.artifact_ref,
            record.trust_state,
            record.build_record_refs.len(),
            record.record_ref
        ));
    }
    if let Ok(record) = parse_build_record(value) {
        return Ok(format!(
            "provenance build record expected_artifact={} sources={} toolchains={} params={} record={}",
            record.expected_artifact_ref,
            record.source_refs.len(),
            record.toolchain_refs.len(),
            record.build_params.len(),
            record.record_ref
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(10)) {
        require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        return Ok(format!(
            "provenance receipt decision={} operation={} artifact={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_ref(&fields[4], "artifact")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-receipt-v1", Some(9)) {
        require_schema(&fields[0], crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA, "provenance receipt")?;
        return Ok(format!(
            "provenance receipt decision={} operation={} artifact={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_ref(&fields[4], "artifact")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("provenance-build-verify-receipt-v1", Some(8)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA,
            "provenance build verify receipt",
        )?;
        return Ok(format!(
            "provenance build verify receipt decision={} expected={} actual={}",
            record_string(&fields[1], "decision")?,
            record_ref(&fields[2], "expected-artifact")?,
            record_ref(&fields[3], "actual-artifact")?
        ));
    }
    Err(MoltenError::invalid_harness("unsupported provenance artifact"))
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> Result<IoValue> {
    let reproducible_check = if input.trust_state == TRUST_STATE_REPRODUCIBLE_VERIFIED {
        input.decision
    } else {
        "pass"
    };
    Ok(record("provenance-receipt-v1", vec![
        string(crate::preserves_rail::PROVENANCE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("profile", vec![string(input.profile)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("trust-state", vec![string(input.trust_state)]),
        record("provenance", vec![optional_ref_value(input.record_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("hash-is-not-trust"), string("pass")]),
            record("check", vec![string("artifact-ref-bound"), string("pass")]),
            record("check", vec![string("trust-state-admitted"), string(input.decision)]),
            record("check", vec![string("reproducible-build-verification"), string(reproducible_check)]),
            record("check", vec![string("canonical-provenance-receipt"), string("pass")]),
        ])]),
        record("build-verifications", vec![refs_sequence(input.build_verification_refs)]),
    ]))
}

fn build_verify_receipt_value(input: &BuildVerifyReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(record("provenance-build-verify-receipt-v1", vec![
        string(crate::preserves_rail::PROVENANCE_BUILD_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("expected-artifact", vec![string(input.expected_artifact_ref)]),
        record("actual-artifact", vec![string(input.actual_artifact_ref)]),
        record("build-record", vec![string(input.build_record_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("build-record-bound"), string("pass")]),
            record("check", vec![string("expected-artifact-bound"), string("pass")]),
            record("check", vec![string("actual-artifact-bound"), string("pass")]),
            record("check", vec![string("artifact-match"), string(input.decision)]),
            record("check", vec![string("canonical-build-verify-receipt"), string("pass")]),
        ])]),
        record("boundary", vec![sequence(vec![
            record("does-not-grant", vec![string("authority")]),
            record("does-not-grant", vec![string("policy")]),
            record("does-not-grant", vec![string("resource")]),
            record("does-not-grant", vec![string("transport")]),
            record("does-not-grant", vec![string("execution")]),
            record("does-not-grant", vec![string("source-gate")]),
        ])]),
    ]))
}

fn reproducible_build_binding_diagnostics(
    record: &Record,
    artifact_ref: &str,
    receipts: &[BuildVerificationReceipt],
) -> Vec<String> {
    if receipts.is_empty() {
        return vec![format!(
            "reproducible-verified provenance for {artifact_ref} requires a passing build verification receipt"
        )];
    }
    if record.build_record_refs.is_empty() {
        return vec![format!(
            "reproducible-verified provenance for {artifact_ref} must bind at least one build record ref"
        )];
    }
    let mut candidate_diagnostics = Vec::with_capacity(receipts.len().saturating_mul(3));
    for receipt in receipts {
        let mut receipt_diagnostics = Vec::with_capacity(3);
        if receipt.decision != "pass" {
            receipt_diagnostics
                .push(format!("build verification receipt {} decision is {}", receipt.receipt_ref, receipt.decision));
        }
        if receipt.expected_artifact_ref != artifact_ref || receipt.actual_artifact_ref != artifact_ref {
            receipt_diagnostics.push(format!(
                "build verification receipt {} does not match artifact {}: expected {} actual {}",
                receipt.receipt_ref, artifact_ref, receipt.expected_artifact_ref, receipt.actual_artifact_ref
            ));
        }
        if !record.build_record_refs.iter().any(|reference| reference == &receipt.build_record_ref) {
            receipt_diagnostics.push(format!(
                "build verification receipt {} build record {} is not bound by provenance record {}",
                receipt.receipt_ref, receipt.build_record_ref, record.record_ref
            ));
        }
        if receipt_diagnostics.is_empty() {
            return Vec::new();
        }
        candidate_diagnostics.extend(receipt_diagnostics);
    }
    candidate_diagnostics
}

fn is_trust_state_admitted(trust_state: &str, profile: &str) -> bool {
    matches!(trust_state, TRUST_STATE_REVIEWED | TRUST_STATE_REPRODUCIBLE_VERIFIED | TRUST_STATE_POLICY_TRUSTED)
        || (trust_state == TRUST_STATE_SANDBOX_ONLY && profile == PROFILE_LOCAL_TEST)
}

fn stronger_diagnostics(record: &Record, operation: &str, profile: &str) -> Vec<String> {
    let has_strong_trust = is_strong_trust_state(&record.trust_state);
    if operation_requires_strong_provenance(operation) {
        if has_strong_trust {
            Vec::new()
        } else {
            vec![format!(
                "operation {operation} under profile {profile} requires stronger provenance than {} for artifact {}",
                record.trust_state, record.artifact_ref
            )]
        }
    } else {
        Vec::new()
    }
}

fn operation_requires_strong_provenance(operation: &str) -> bool {
    matches!(
        operation,
        "install-policy-artifact"
            | "install-migration-recipe"
            | "install-production-executable"
            | "remote-sync-execute"
    )
}

fn is_strong_trust_state(trust_state: &str) -> bool {
    matches!(trust_state, TRUST_STATE_REPRODUCIBLE_VERIFIED | TRUST_STATE_POLICY_TRUSTED)
}

fn validate_trust_state(trust_state: &str) -> Result<()> {
    if matches!(
        trust_state,
        TRUST_STATE_UNKNOWN
            | TRUST_STATE_SOURCE_KNOWN
            | TRUST_STATE_BUILDER_ATTESTED
            | TRUST_STATE_REVIEWED
            | TRUST_STATE_REPRODUCIBLE_VERIFIED
            | TRUST_STATE_SANDBOX_ONLY
            | TRUST_STATE_POLICY_TRUSTED
            | TRUST_STATE_DENIED
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance trust state `{trust_state}`")))
    }
}

fn validate_profile(profile: &str) -> Result<()> {
    if matches!(profile, "node-control" | PROFILE_LOCAL_TEST) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance evaluation profile `{profile}`")))
    }
}

fn validate_build_params(params: &[BuildParam]) -> Result<()> {
    ensure_ref_bound(params.len(), MAX_BUILD_PARAMS, "provenance build params")?;
    for param in params {
        validate_build_param(param)?;
    }
    Ok(())
}

fn validate_build_param(param: &BuildParam) -> Result<()> {
    validate_build_param_token(&param.key, "provenance build param key")?;
    validate_build_param_token(&param.value, "provenance build param value")
}

fn validate_build_param_token(value: &str, context: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{context} must not be empty")));
    }
    if value.len() > MAX_BUILD_PARAM_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{context} is too long: {} > {MAX_BUILD_PARAM_BYTES}",
            value.len()
        )));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(MoltenError::invalid_harness(format!("{context} must not contain newlines")));
    }
    Ok(())
}

fn build_params_sequence(params: &[BuildParam]) -> IoValue {
    let mut sorted = params.to_vec();
    sorted.sort();
    sequence(
        sorted
            .iter()
            .map(|param| record("build-param", vec![string(&param.key), string(&param.value)]))
            .collect(),
    )
}

fn record_build_params_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<BuildParam>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_BUILD_PARAMS, tag)?;
    let mut params = Vec::with_capacity(items.len());
    for item in items.iter() {
        params.push(required_build_param(item, tag)?);
    }
    validate_build_params(&params)?;
    Ok(params)
}

fn required_build_param(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<BuildParam> {
    let item_value = value_to_iovalue(value);
    let fields = item_value
        .collect_simple_record("build-param", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} item must be <build-param key value>")))?;
    let key = fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param key must be a string")))?;
    let value = fields[1]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param value must be a string")))?;
    Ok(BuildParam { key, value })
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}
