//! Canonical operator trust and exact-invocation credential producer contract.

pub struct Trust {
    pub issuer: String,
    pub issuer_key: [u8; 32],
    pub holder: String,
    pub holder_key: [u8; 32],
    pub policy: String,
    pub not_before: u64,
    pub expires: u64,
    pub revoked: Vec<[u8; 32]>,
}

pub struct Credential {
    pub token: String,
    pub nonce: [u8; 32],
    pub signature: [u8; 64],
    pub proofs: Vec<String>,
}

pub fn encode_trust(input: &Trust) -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("node-service-host-trust-v1", vec![
        crate::preserves_rail::string(&input.issuer),
        preserves::IOValue::bytes(input.issuer_key),
        crate::preserves_rail::string(&input.holder),
        preserves::IOValue::bytes(input.holder_key),
        crate::preserves_rail::string(&input.policy),
        crate::preserves_rail::u64_value(input.not_before),
        crate::preserves_rail::u64_value(input.expires),
        crate::preserves_rail::sequence(input.revoked.iter().map(|value| preserves::IOValue::bytes(*value)).collect()),
    ]))
}

pub fn decode_trust(bytes: &[u8]) -> crate::error::Result<Trust> {
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    let f = crate::preserves_rail::simple_record_fields(&value, "node-service-host-trust-v1", 8)?;
    let revoked = crate::preserves_rail::required_sequence_field(&f[7], "revoked")?;
    if revoked.len() > 4096 {
        return Err(invalid());
    }
    let result = Trust {
        issuer: text(&f[0], 1024)?,
        issuer_key: fixed(&f[1])?,
        holder: text(&f[2], 1024)?,
        holder_key: fixed(&f[3])?,
        policy: text(&f[4], 65536)?,
        not_before: integer(&f[5])?,
        expires: integer(&f[6])?,
        revoked: revoked.iter().map(fixed).collect::<crate::error::Result<_>>()?,
    };
    if result.not_before >= result.expires {
        return Err(invalid());
    }
    Ok(result)
}

pub fn encode_credential(input: &Credential) -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("node-service-host-credential-v1", vec![
        crate::preserves_rail::string(&input.token),
        preserves::IOValue::bytes(input.nonce),
        preserves::IOValue::bytes(input.signature),
        crate::preserves_rail::sequence(input.proofs.iter().map(crate::preserves_rail::string).collect()),
    ]))
}

pub fn decode_credential(bytes: &[u8]) -> crate::error::Result<Credential> {
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    let f = crate::preserves_rail::simple_record_fields(&value, "node-service-host-credential-v1", 4)?;
    let proofs = crate::preserves_rail::required_sequence_field(&f[3], "proofs")?;
    if proofs.len() > 32 {
        return Err(invalid());
    }
    Ok(Credential {
        token: text(&f[0], 65536)?,
        nonce: fixed(&f[1])?,
        signature: fixed(&f[2])?,
        proofs: proofs.iter().map(|v| text(v, 65536)).collect::<crate::error::Result<_>>()?,
    })
}

fn text(value: &preserves::Value<preserves::IOValue>, maximum: usize) -> crate::error::Result<String> {
    let text = crate::preserves_rail::required_string_field(value, "authority text")?;
    if text.is_empty() || text.len() > maximum {
        return Err(invalid());
    }
    Ok(text)
}

fn fixed<const N: usize>(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<[u8; N]> {
    value.as_bytestring().ok_or_else(invalid)?.as_ref().try_into().map_err(|_| invalid())
}

fn integer(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value.as_u64().ok_or_else(invalid)?.map_err(|_| invalid())
}

fn invalid() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("invalid service host authority wire artifact")
}
