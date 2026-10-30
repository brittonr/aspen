pub fn value(input: &serde_json::Value) -> crate::error::Result<preserves::IOValue> {
    let record = |label, fields| crate::preserves_rail::record(label, fields);
    Ok(match input {
        serde_json::Value::Null => record("null", vec![]),
        serde_json::Value::Bool(v) => record("boolean", vec![crate::preserves_rail::bool_value(*v)]),
        serde_json::Value::Number(v) => {
            let integer = v
                .as_i64()
                .map(preserves::IOValue::new)
                .or_else(|| v.as_u64().map(preserves::IOValue::new))
                .ok_or_else(|| super::denied("coordination canonical payload forbids floating point"))?;
            record("integer", vec![integer])
        }
        serde_json::Value::String(v) => record("string", vec![crate::preserves_rail::string(v)]),
        serde_json::Value::Array(v) => record("array", v.iter().map(value).collect::<crate::error::Result<_>>()?),
        serde_json::Value::Object(v) => {
            let sorted: std::collections::BTreeMap<_, _> = v.iter().collect();
            record(
                "object",
                sorted
                    .into_iter()
                    .map(|(k, v)| Ok(record("field", vec![crate::preserves_rail::string(k), value(v)?])))
                    .collect::<crate::error::Result<_>>()?,
            )
        }
    })
}

pub fn identity<T: serde::Serialize>(purpose: &str, payload: &T) -> crate::error::Result<String> {
    let json = serde_json::to_value(payload).map_err(|e| super::denied(e.to_string()))?;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten-coordination-authority-v1", vec![
        crate::preserves_rail::string(purpose),
        value(&json)?,
    ]))
}

fn statement<T: serde::Serialize>(
    purpose: &str,
    payload: &T,
    key: &iroh::PublicKey,
) -> crate::error::Result<artifact_auth_core::ArtifactStatement> {
    let digest = identity(purpose, payload)?;
    let reference = artifact_auth_core::ArtifactRef {
        profile: "molten-coordination-canonical-preserves-v1".into(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.into(),
        digest_hex: digest.trim_start_matches("blake3:").into(),
    };
    Ok(artifact_auth_core::ArtifactStatement {
        schema: artifact_auth_core::STATEMENT_SCHEMA_V1.into(),
        scope: artifact_auth_core::AuthenticationScope {
            domain: "onix.molten.experimental-coordination.v1".into(),
            purpose: purpose.into(),
            profile_id: "molten-coordination-canonical-preserves-v1".into(),
            subject: reference.clone(),
            parents: vec![],
            verifier_context: reference,
        },
        producer_id: "explicit-coordination-operator".into(),
        key_id: key.to_string(),
        key_identity: artifact_auth_ed25519::public_key_identity(key.as_bytes()),
    })
}

pub fn sign<T: serde::Serialize>(purpose: &str, payload: &T, key: &iroh::SecretKey) -> crate::error::Result<Vec<u8>> {
    let statement = statement(purpose, payload, &key.public())?;
    let bytes = artifact_auth_core::canonical_statement_bytes(&statement)
        .map_err(|_| super::denied("invalid coordination signing statement"))?;
    Ok(key.sign(&bytes).to_bytes().to_vec())
}

pub fn verify<T: serde::Serialize>(
    purpose: &str,
    payload: &T,
    key: &str,
    signature: &[u8],
) -> crate::error::Result<()> {
    let key: iroh::PublicKey = key.parse().map_err(|_| super::denied("invalid coordination public key"))?;
    let statement = statement(purpose, payload, &key)?;
    if artifact_auth_ed25519::verify_statement(&statement, key.as_bytes(), signature)
        .failure_code
        .is_some()
    {
        return Err(super::denied("coordination signature rejected"));
    }
    Ok(())
}
