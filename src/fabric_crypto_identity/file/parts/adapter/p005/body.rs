
/// Revocation records persisted for one key purpose. A record names one exact key handle and
/// generation (and the public key it exposed); it never denies every key of that purpose.
pub(super) fn read_key_revocations(
    namespace: &crate::node_state::Scope,
    purpose: KeyPurpose,
) -> crate::error::Result<Vec<KeyRevocation>> {
    match namespace.observe_file(&revocation_path(purpose)?)? {
        crate::node_state::FileObservation::Missing => Ok(Vec::new()),
        crate::node_state::FileObservation::Regular(file) => {
            require_restricted_permission(permission_status(&file))?;
            let marker = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
            if marker.is_empty() {
                return Err(crate::error::Failure::invalid_harness("cryptographic key revocation marker is empty"));
            }
            parse_key_revocations(&crate::preserves_rail::parse_canonical_bytes(&marker)?)
        }
        crate::node_state::FileObservation::NonRegular(kind) => {
            Err(crate::error::Failure::invalid_harness(format!(
                "cryptographic key revocation marker must be a regular file, got {kind:?}"
            )))
        }
    }
}

fn key_revocation_value(revocation: &KeyRevocation) -> preserves::IOValue {
    crate::preserves_rail::record(KEY_REVOCATION_LABEL, vec![
        crate::preserves_rail::string(&revocation.handle_ref),
        crate::preserves_rail::string(&revocation.public_key_ref),
        crate::preserves_rail::u64_value(revocation.generation),
        crate::preserves_rail::string(&revocation.evidence_ref),
        crate::preserves_rail::string(&revocation.policy_ref),
    ])
}

pub(super) fn key_revocations_value(revocations: &[KeyRevocation]) -> preserves::IOValue {
    crate::preserves_rail::record(KEY_REVOCATIONS_LABEL, vec![crate::preserves_rail::sequence(
        revocations.iter().map(key_revocation_value).collect(),
    )])
}

/// Accepts the revocation-set marker and the earlier single-record marker; anything else fails closed.
fn parse_key_revocations(value: &preserves::IOValue) -> crate::error::Result<Vec<KeyRevocation>> {
    if value.collect_simple_record(KEY_REVOCATION_LABEL, Some(KEY_REVOCATION_ARITY)).is_some() {
        return Ok(vec![parse_key_revocation(value)?]);
    }
    let fields = value
        .collect_simple_record(KEY_REVOCATIONS_LABEL, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness("cryptographic key revocation marker is malformed"))?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("cryptographic key revocation set is malformed"))?;
    if entries.is_empty() || entries.len() > MAX_KEY_REVOCATIONS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "cryptographic key revocation set must hold 1..={MAX_KEY_REVOCATIONS} records"
        )));
    }
    entries
        .iter()
        .map(|entry| parse_key_revocation(&crate::preserves_rail::value_to_iovalue(entry)))
        .collect()
}

fn parse_key_revocation(value: &preserves::IOValue) -> crate::error::Result<KeyRevocation> {
    let fields = value
        .collect_simple_record(KEY_REVOCATION_LABEL, Some(KEY_REVOCATION_ARITY))
        .ok_or_else(|| crate::error::Failure::invalid_harness("cryptographic key revocation record is malformed"))?;
    let text = |index: usize, label: &str| {
        fields[index].as_string().map(|value| value.into_owned()).ok_or_else(|| {
            crate::error::Failure::invalid_harness(format!("cryptographic key revocation {label} is malformed"))
        })
    };
    let revocation = KeyRevocation {
        handle_ref: text(0, "handle ref")?,
        public_key_ref: text(1, "public key ref")?,
        generation: fields[2]
            .as_u64()
            .and_then(|generation| generation.ok())
            .ok_or_else(|| crate::error::Failure::invalid_harness("cryptographic key revocation generation is malformed"))?,
        evidence_ref: text(3, "evidence ref")?,
        policy_ref: text(4, "policy ref")?,
    };
    require_blake3_ref("revoked key handle", &revocation.handle_ref)?;
    require_blake3_ref("revoked public key", &revocation.public_key_ref)?;
    Ok(revocation)
}

/// A revocation applies to the exact revoked handle at its generation, and to any handle that exposes
/// the same revoked public key; a successor key under another handle or backend stays usable.
fn key_revocation_matches(revocations: &[KeyRevocation], subject: KeyRevocationSubject<'_>) -> bool {
    revocations.iter().any(|revocation| {
        (revocation.handle_ref == subject.handle_ref && revocation.generation == subject.generation)
            || revocation.public_key_ref == subject.public_key_ref
    })
}

fn revoked_key_error(purpose: KeyPurpose) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("{} key is revoked", purpose.as_str()))
}

// r[impl molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
pub(super) fn require_key_not_revoked(
    namespace: &crate::node_state::Scope,
    subject: KeyRevocationSubject<'_>,
) -> crate::error::Result<()> {
    if key_revocation_matches(&read_key_revocations(namespace, subject.purpose)?, subject) {
        return Err(revoked_key_error(subject.purpose));
    }
    Ok(())
}

fn require_current_handle(requested: &OpaqueKeyHandle, current: &OpaqueKeyHandle) -> crate::error::Result<()> {
    if requested != current {
        return Err(crate::error::Failure::invalid_harness("stale or mismatched cryptographic key handle denied"));
    }
    Ok(())
}

fn encode_key_record(record: &KeyRecord) -> [u8; KEY_RECORD_BYTES] {
    let mut bytes = [0u8; KEY_RECORD_BYTES];
    bytes[..KEY_GENERATION_START].copy_from_slice(KEY_RECORD_SCHEMA);
    bytes[KEY_GENERATION_START..KEY_SECRET_START].copy_from_slice(&record.generation.to_be_bytes());
    bytes[KEY_SECRET_START..].copy_from_slice(&record.secret_key.to_bytes());
    bytes
}
