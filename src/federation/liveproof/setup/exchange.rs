pub struct Handoff {
    pub endpoint: crate::fabric_transport::cross_process::Endpoint,
    pub public_key: String,
    pub signer_ref: String,
    pub generation: u64,
    pub process_id: u64,
}
pub fn publish(
    scope: &crate::node_state::Scope,
    role: &str,
    listener: &crate::fabric_transport::cross_process::Listener,
    key: &crate::fabric_crypto_identity::ResolvedProductionKey,
) -> crate::error::Result<()> {
    let value = crate::preserves_rail::record("federation-live-handoff-v1", vec![
        listener.handoff().value.clone(),
        crate::preserves_rail::string(&key.public_key),
        crate::preserves_rail::string(&key.handle.handle.public_key_ref),
        crate::preserves_rail::u64_value(key.handle.handle.generation),
        crate::preserves_rail::u64_value(u64::from(std::process::id())),
    ]);
    publish_value(scope, &format!("{role}-handoff.preserves"), &value)
}
pub fn publish_value(
    scope: &crate::node_state::Scope,
    leaf: &str,
    value: &preserves::IOValue,
) -> crate::error::Result<()> {
    super::dagports::persist(scope, leaf, value)?;
    scope.write(&crate::node_state::RelativePath::parse(&format!("{leaf}.ready"))?, b"ready")
}
pub fn wait(scope: &crate::node_state::Scope, leaf: &str) -> crate::error::Result<preserves::IOValue> {
    let path = crate::node_state::RelativePath::parse(leaf)?;
    let ready = crate::node_state::RelativePath::parse(&format!("{leaf}.ready"))?;
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(super::transport::TIMEOUT)?;
    loop {
        if scope.try_exists(&ready)? {
            return crate::preserves_rail::parse_canonical_bytes(&scope.read(&path, super::transport::LIMIT)?);
        }
        if deadline.is_expired()? {
            return Err(super::super::invalid("fixture public handoff timed out"));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
pub fn handoff(scope: &crate::node_state::Scope, role: &str) -> crate::error::Result<Handoff> {
    let value = wait(scope, &format!("{role}-handoff.preserves"))?;
    let fields = value
        .collect_simple_record("federation-live-handoff-v1", Some(5))
        .ok_or_else(|| super::super::invalid("invalid fixture public handoff"))?;
    Ok(Handoff {
        endpoint: crate::fabric_transport::cross_process::parse_endpoint(&crate::preserves_rail::value_to_iovalue(
            &fields[0],
        ))?,
        public_key: fields[1].as_string().ok_or_else(|| super::super::invalid("missing public key"))?.into_owned(),
        signer_ref: fields[2].as_string().ok_or_else(|| super::super::invalid("missing signer ref"))?.into_owned(),
        generation: fields[3]
            .as_u64()
            .and_then(|value| value.ok())
            .ok_or_else(|| super::super::invalid("invalid key generation"))?,
        process_id: fields[4]
            .as_u64()
            .and_then(|value| value.ok())
            .ok_or_else(|| super::super::invalid("invalid role process"))?,
    })
}
pub fn context(
    provider: &Handoff,
    receiver_key: &str,
    receiver_ref: &str,
    receiver_generation: u64,
) -> super::super::receipt::PublicContext {
    super::super::receipt::PublicContext {
        provider_key: provider.public_key.clone(),
        provider_ref: provider.signer_ref.clone(),
        receiver_key: receiver_key.to_string(),
        receiver_ref: receiver_ref.to_string(),
        provider_generation: provider.generation,
        receiver_generation,
        service_generation: 1,
        context_ref: super::transport::reference("federation-live-signature-context"),
        policy_ref: super::transport::reference("federation-live-signature-policy"),
        observed_at: super::transport::NOW,
        max_age: 100,
    }
}
pub fn verification<'a>(
    adapter: &'a crate::fabric_crypto_identity::IrohEd25519<'_>,
    peer: &'a Handoff,
    purpose: molten_core::federation::RecordPurpose,
    context: &'a str,
    policy: &'a str,
) -> crate::mesh::envelope::signatures::VerificationInput<'a> {
    crate::mesh::envelope::signatures::VerificationInput {
        profile: adapter.profile(),
        public_key: &peer.public_key,
        signer_ref: &peer.signer_ref,
        generation: peer.generation,
        currentness: crate::fabric_crypto_identity::KeyCurrentness::Current,
        context_ref: context,
        policy_ref: policy,
        purpose,
        now: super::transport::NOW,
        max_age: 100,
    }
}
pub fn signing<'a>(
    adapter: &'a crate::fabric_crypto_identity::IrohEd25519<'_>,
    handle: &'a crate::fabric_crypto_identity::OpaqueKeyHandle,
    purpose: molten_core::federation::RecordPurpose,
    context: &'a str,
    policy: &'a str,
) -> crate::mesh::envelope::signatures::SigningInput<'a> {
    crate::mesh::envelope::signatures::SigningInput {
        profile: adapter.profile(),
        handle,
        context_ref: context,
        policy_ref: policy,
        purpose,
        issued_at: super::transport::NOW,
        expires_at: super::transport::UNTIL,
        payload: &EMPTY,
    }
}
static EMPTY: std::sync::LazyLock<preserves::IOValue> = std::sync::LazyLock::new(super::profiles::artifact);
