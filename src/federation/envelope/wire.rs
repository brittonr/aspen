pub fn encode(record: &super::signatures::SignedRecord, maximum: u64) -> crate::error::Result<Vec<u8>> {
    let signature = crate::preserves_rail::sequence(
        record
            .signature
            .signature
            .iter()
            .map(|byte| crate::preserves_rail::u64_value(u64::from(*byte)))
            .collect(),
    );
    let value = crate::preserves_rail::record("federation-wire-v1", vec![
        crate::preserves_rail::string(record.purpose.domain()),
        crate::preserves_rail::string(record.generation.to_string()),
        crate::preserves_rail::string(record.issued_at.to_string()),
        crate::preserves_rail::string(record.expires_at.to_string()),
        record.payload.clone(),
        signature,
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if exceeds(&bytes, maximum) {
        return Err(invalid("federation outgoing record exceeds bound"));
    }
    Ok(bytes)
}

pub fn decode(
    bytes: &[u8],
    input: &super::signatures::VerificationInput<'_>,
    maximum: u64,
) -> crate::error::Result<super::signatures::SignedRecord> {
    if exceeds(bytes, maximum) {
        return Err(invalid("federation incoming record exceeds bound"));
    }
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let fields = value
        .collect_simple_record("federation-wire-v1", Some(6))
        .ok_or_else(|| invalid("malformed federation wire record"))?;
    let purpose = fields[0].as_string().ok_or_else(|| invalid("missing wire purpose"))?;
    if purpose.as_ref() != input.purpose.domain() {
        return Err(invalid("wrong wire purpose"));
    }
    let generation = number(&fields[1])?;
    let issued_at = number(&fields[2])?;
    let expires_at = number(&fields[3])?;
    let payload = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let signature = fields[5].collect_sequence().ok_or_else(|| invalid("missing wire signature"))?;
    if signature.len() != 64 {
        return Err(invalid("malformed Ed25519 wire signature"));
    }
    let signature = signature
        .iter()
        .map(|value| {
            let value = value.as_u64().and_then(|value| value.ok()).and_then(|value| u8::try_from(value).ok());
            value.ok_or_else(|| invalid("invalid wire signature byte"))
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let signed = super::signatures::signed_payload(input.purpose, generation, issued_at, expires_at, &payload);
    let domain =
        super::signatures::payload_domain(input.profile, input.purpose, &signed, input.signer_ref, input.context_ref)?;
    let plan = crate::fabric_crypto_identity::SignPlan {
        operation_id: "federation-wire-decode".to_string(),
        handle_ref: input.signer_ref.to_string(),
        profile_ref: input.profile.profile.profile_ref.clone(),
        algorithm: crate::fabric_crypto_identity::CryptoAlgorithm::Ed25519Iroh,
        purpose: crate::fabric_crypto_identity::KeyPurpose::FederationOrigin,
        generation,
        domain: domain.domain.clone(),
        policy_ref: input.policy_ref.to_string(),
    };
    let signature = crate::fabric_crypto_identity::signature_outcome(input.profile, &plan, &domain, signature)?;
    Ok(super::signatures::SignedRecord {
        purpose: input.purpose,
        generation,
        issued_at,
        expires_at,
        payload,
        signature,
    })
}

fn number(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    let text = value.as_string().ok_or_else(|| invalid("missing wire logical time"))?;
    let value = text.parse::<u64>().map_err(|_| invalid("invalid wire logical time"))?;
    if value.to_string() != text.as_ref() {
        return Err(invalid("noncanonical wire logical time"));
    }
    Ok(value)
}
fn exceeds(bytes: &[u8], maximum: u64) -> bool {
    u64::try_from(bytes.len()).ok().is_none_or(|length| length > maximum)
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
