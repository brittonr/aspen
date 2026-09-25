
// r[impl molten.crypto_identity.redaction]
pub fn canonical_crypto_status(input: &AdapterDiagnosticInput) -> crate::error::Result<CryptoStatusReadback> {
    let status = redact_adapter_status(input).map_err(|issues| validation_error("crypto status", &issues))?;
    let value = crate::preserves_rail::record(CRYPTO_STATUS_RECORD, vec![
        crate::preserves_rail::string(CRYPTO_STATUS_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&status.profile_ref)),
        field("purpose", crate::preserves_rail::string(status.purpose.as_str())),
        field("generation", optional_u64(status.generation)),
        field("currentness", optional_string(status.currentness.map(KeyCurrentness::as_str))),
        field("permission-status", crate::preserves_rail::string(status.permission_status.as_str())),
        field("backend-class", crate::preserves_rail::string(status.backend_class.as_str())),
        field("public-key-ref", optional_string(status.public_key_ref.as_deref())),
        field("receipt-refs", strings_value(status.receipt_refs.iter().map(String::as_str))),
        field("backend-locator-redacted", crate::preserves_rail::bool_value(status.has_redacted_backend_locator)),
        field("raw-error-redacted", crate::preserves_rail::bool_value(status.has_redacted_error)),
        field("bearer-token-redacted", crate::preserves_rail::bool_value(status.has_redacted_bearer_token)),
        checks(&[
            "private-key-excluded",
            "backend-locator-excluded",
            "credentials-excluded",
            "public-status-bounded",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CryptoStatusReadback {
        status,
        status_ref,
        value,
    })
}

pub fn fabric_crypto_port_descriptors(profile: &CanonicalCryptoProfile) -> Vec<crate::fabric::FabricPortDescriptor> {
    let definitions = [
        (
            FABRIC_CRYPTO_KEY_PORT_ID,
            vec!["generate", "resolve", "public-key", "rotate", "revoke", "status"],
            vec![
                crate::fabric::FabricAuthority::DurableState,
                crate::fabric::FabricAuthority::Time,
                crate::fabric::FabricAuthority::Policy,
            ],
        ),
        (FABRIC_CRYPTO_SIGNATURE_PORT_ID, vec!["sign", "verify"], vec![
            crate::fabric::FabricAuthority::Evidence,
            crate::fabric::FabricAuthority::Policy,
        ]),
    ];
    let mut descriptors = Vec::with_capacity(FABRIC_CRYPTO_PORT_COUNT);
    for (port_id, operations, authorities) in definitions {
        descriptors.push(crate::fabric::FabricPortDescriptor {
            schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_CRYPTO_PORT_VERSION.to_string(),
            class: crate::fabric::FabricPortClass::Authority,
            operation_classes: operations.into_iter().map(str::to_string).collect(),
            input_schema_refs: vec![CRYPTO_OPERATION_SCHEMA.to_string()],
            output_schema_refs: vec![CRYPTO_OUTCOME_SCHEMA.to_string()],
            authority_requirements: authorities,
            resource_requirements: vec![
                crate::fabric::FabricResource::StorageBytes,
                crate::fabric::FabricResource::Memory,
            ],
            determinism: crate::fabric::DeterminismClass::ExternalEffect,
            replay: crate::fabric::ReplayClass::RecordedEffectRequired,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.admission_ref.clone(), profile.profile.profile_ref.clone()],
            non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

fn issue_label(issue: &CryptoIdentityIssue) -> &'static str {
    match issue {
        CryptoIdentityIssue::PurposeMismatch => "purpose-mismatch",
        CryptoIdentityIssue::PayloadRefMismatch => "payload-ref-mismatch",
        CryptoIdentityIssue::SignerPublicRefMismatch => "signer-public-ref-mismatch",
        CryptoIdentityIssue::VerifierContextMismatch => "verifier-context-mismatch",
        CryptoIdentityIssue::CryptographicVerificationFailed => "cryptographic-verification-failed",
        CryptoIdentityIssue::HandleNotCurrent(_) => "handle-not-current",
        CryptoIdentityIssue::HandleGenerationStale { .. } => "handle-generation-stale",
        CryptoIdentityIssue::SignatureTooLarge { .. } => "signature-size-invalid",
        CryptoIdentityIssue::SignatureMalformed => "signature-malformed",
        _ => "crypto-admission-issue",
    }
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::sequence(Vec::new()),
        |value| crate::preserves_rail::sequence(vec![crate::preserves_rail::string(value)]),
    )
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::sequence(Vec::new()),
        |value| crate::preserves_rail::sequence(vec![crate::preserves_rail::u64_value(value)]),
    )
}

fn bytes_value(bytes: &[u8]) -> preserves::IOValue {
    crate::preserves_rail::sequence(
        bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
    )
}

fn checks(values: &[&str]) -> preserves::IOValue {
    strings_value(values.iter().copied())
}
