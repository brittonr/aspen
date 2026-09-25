
fn decode_key_record(bytes: &[u8]) -> crate::error::Result<KeyRecord> {
    let bytes: [u8; KEY_RECORD_BYTES] = bytes
        .try_into()
        .map_err(|_| crate::error::MoltenError::invalid_harness("production key record has an invalid length"))?;
    if &bytes[..KEY_GENERATION_START] != KEY_RECORD_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("production key record schema is malformed"));
    }
    let generation_bytes: [u8; KEY_GENERATION_BYTES] = bytes[KEY_GENERATION_START..KEY_SECRET_START]
        .try_into()
        .map_err(|_| crate::error::MoltenError::invalid_harness("production key generation is malformed"))?;
    let generation = u64::from_be_bytes(generation_bytes);
    if generation == 0 {
        return Err(crate::error::MoltenError::invalid_harness("production key generation must be positive"));
    }
    let secret_bytes: [u8; ED25519_SECRET_BYTES] = bytes[KEY_SECRET_START..]
        .try_into()
        .map_err(|_| crate::error::MoltenError::invalid_harness("production secret key bytes are malformed"))?;
    Ok(KeyRecord {
        generation,
        secret_key: iroh::SecretKey::from_bytes(&secret_bytes),
    })
}

fn decode_secret_hex(secret: &str) -> crate::error::Result<[u8; ED25519_SECRET_BYTES]> {
    const HEX_CHARS_PER_BYTE: usize = 2;
    const HEX_RADIX: u32 = 16;
    const EXPECTED_HEX_CHARS: usize = ED25519_SECRET_BYTES * HEX_CHARS_PER_BYTE;
    let secret = secret.trim();
    if secret.len() != EXPECTED_HEX_CHARS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "explicit Ed25519 secret must contain exactly {EXPECTED_HEX_CHARS} lowercase hexadecimal characters"
        )));
    }
    let mut bytes = [0u8; ED25519_SECRET_BYTES];
    for (index, slot) in bytes.iter_mut().enumerate() {
        let offset = index
            .checked_mul(HEX_CHARS_PER_BYTE)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("secret hex offset overflow"))?;
        let pair = &secret[offset..offset + HEX_CHARS_PER_BYTE];
        if !pair.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f')) {
            return Err(crate::error::MoltenError::invalid_harness(
                "explicit Ed25519 secret must use lowercase hexadecimal characters",
            ));
        }
        *slot = u8::from_str_radix(pair, HEX_RADIX).map_err(|_| {
            crate::error::MoltenError::invalid_harness("explicit Ed25519 secret contains malformed hex")
        })?;
    }
    Ok(bytes)
}

fn currentness_evidence_ref(
    profile_ref: &str,
    purpose: KeyPurpose,
    generation: u64,
    public_key_ref: &str,
    backend_ref: &str,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("crypto-key-currentness-v1", vec![
        crate::preserves_rail::string(profile_ref),
        crate::preserves_rail::string(purpose.as_str()),
        crate::preserves_rail::u64_value(generation),
        crate::preserves_rail::string(public_key_ref),
        crate::preserves_rail::string(backend_ref),
        crate::preserves_rail::string("current"),
    ]))
}

fn permission_status(file: &crate::node_state::NodeStateFile) -> KeyPermissionStatus {
    #[cfg(unix)]
    {
        match file.unix_mode() {
            Some(mode) if mode & GROUP_OR_OTHER_PERMISSION_BITS == 0 => KeyPermissionStatus::Restricted,
            Some(_) => KeyPermissionStatus::Unsafe,
            None => KeyPermissionStatus::Unsupported,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        KeyPermissionStatus::Unsupported
    }
}

fn require_canonical_domain(
    profile: &CanonicalCryptoProfile,
    supplied: &CanonicalSignatureDomain,
) -> crate::error::Result<()> {
    let rebuilt = canonical_signature_domain(profile, &supplied.domain)?;
    if &rebuilt != supplied {
        return Err(crate::error::MoltenError::invalid_harness(
            "signature domain does not match its canonical Preserves identity",
        ));
    }
    Ok(())
}

fn require_canonical_signature(supplied: &CanonicalSignatureOutcome) -> crate::error::Result<()> {
    let signature_bytes = u64::try_from(supplied.signature.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("signature length does not fit u64"))?;
    let signature_ref = crate::preserves_rail::content_ref_from_bytes(&supplied.signature);
    let value = canonical_signature_value(&supplied.metadata, &supplied.signature);
    let outcome_ref = crate::preserves_rail::canonical_hash(&value)?;
    if supplied.metadata.signature_bytes != signature_bytes
        || supplied.metadata.signature_ref != signature_ref
        || supplied.value != value
        || supplied.outcome_ref != outcome_ref
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "signature outcome does not match its canonical Preserves identity",
        ));
    }
    Ok(())
}

fn require_restricted_permission(status: KeyPermissionStatus) -> crate::error::Result<()> {
    match status {
        KeyPermissionStatus::Restricted => Ok(()),
        KeyPermissionStatus::Unsafe => {
            Err(crate::error::MoltenError::invalid_harness("production key permissions are not owner-only"))
        }
        KeyPermissionStatus::Unsupported => {
            Err(crate::error::MoltenError::invalid_harness("production key permission verification is unavailable"))
        }
    }
}

fn require_blake3_ref(label: &str, value: &str) -> crate::error::Result<()> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let is_valid = value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    });
    if is_valid {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("{label} ref is malformed")))
    }
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}
