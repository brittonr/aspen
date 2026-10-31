pub fn run(root: &std::path::Path, revision: &str, program: Option<&std::path::Path>) -> super::Result<()> {
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("provider revision must be the actual reviewed full 40-hex source pin".into());
    }
    let witness_policy = super::index::policy(revision.to_owned())?;
    let witness_policy_ref = molten::world_witness::policy_identity(&witness_policy)?;
    std::fs::create_dir(root)?;
    let consumer = molten::node_state::DirectoryRoot::open(&root.join("consumer"))?;
    consumer.create_layout()?;
    super::payload::materialize(&consumer)?;
    let operator = molten::node_state::DirectoryRoot::open(&root.join("operator"))?;
    operator.create_layout()?;
    let secrets = consumer.namespace(molten::node_state::NamespaceKind::Secrets)?;
    let mut signer =
        molten::world_head::LocalWorldHeadSigningAdapter::new(&secrets, molten::world_head::SignerIdentity {
            profile_ref: super::domain::crypto_config_ref("private-pilot-ed25519")?,
            entropy_profile_ref: super::domain::crypto_config_ref("private-pilot-entropy")?,
            backend_ref: super::domain::crypto_config_ref("private-pilot-backend")?,
            producer_id: "private-world-witness-pilot".into(),
            allow_generation: true,
        })?;
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
    let audience = basalt::ucan::AudienceDid::from(basalt::ucan::TokenSigner::issuer(&holder)?);
    let now = super::domain::now()?;
    let expires = now.checked_add(3600).ok_or("pilot validity overflow")?;
    let capabilities = basalt::ucan::CapabilitySet::new(vec![basalt::ucan::CapabilityDocument::new(
        molten::world_witness::authority::resource(&super::domain::branch()?),
        "mutate".into(),
    )?])?;
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(now, expires)?,
    )?;
    let mut trust = None;
    for generation in 1..=super::payload::GENERATIONS {
        let (claim, carrier, _) = molten::world_head::sign_world_head_claim(
            &mut signer,
            &super::domain::claim(generation)?,
            molten_core::world_head::WorldHeadSignerRole::Maintainer,
        )?;
        if trust.is_none() {
            trust = Some(operator_trust(&issuer, &audience, &carrier, (now, expires), witness_policy_ref.clone())?);
        }
        let credential = molten::world_witness::authority::sign_credential(
            molten::world_witness::authority::SigningInput {
                claim: &claim,
                token: token.as_str(),
                audience: &audience,
                nonce: *blake3::hash(&claim.bytes).as_bytes(),
                artifact_signature: carrier.signature_bytes,
            },
            &holder,
        )?;
        molten::world_witness::authority::wire::install_credential(&operator, claim.claim_ref.as_str(), &credential)?;
    }
    let trust = trust.ok_or("missing pilot trust")?;
    molten::world_witness::authority::wire::install_trust(&operator, &trust)?;
    super::authority::initialize(&root.join("operator"), &trust)?;
    super::index::provision(root, program)?;
    std::fs::write(root.join("provider-revision"), revision)?;
    println!("private same-host nonproduction trust provisioned; fixed fixture issuer is not production authority");
    Ok(())
}

fn operator_trust(
    issuer: &basalt::ucan::Ed25519InMemorySigner,
    audience: &basalt::ucan::AudienceDid,
    carrier: &molten::world_head::WorldHeadSignatureCarrier,
    validity: (u64, u64),
    witness_policy_ref: String,
) -> super::Result<molten::world_witness::authority::wire::Trust> {
    Ok(molten::world_witness::authority::wire::Trust {
        issuer: basalt::ucan::TokenSigner::issuer(issuer)?.as_str().into(),
        issuer_key: basalt::ucan::derive_ed25519_public_key(&[71; 32])?,
        holder: audience.as_str().into(),
        holder_key: basalt::ucan::derive_ed25519_public_key(&[73; 32])?,
        policy: r#"{"backends":{},"contracts":{"world-head-witness":{"abilities":["mutate"],"description":"private pilot operator policy","id":"world-head-witness","resource_prefixes":["molten://world-head/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#.into(),
        head_policy_ref: super::domain::policy_ref()?.as_str().into(), witness_policy_ref,
        not_before: validity.0, expires: validity.1, authority_generation: 1,
        revoked: Vec::new(), producer_id: carrier.producer_id.clone(), key_id: carrier.key_id.clone(),
        artifact_public_key: carrier.public_key_bytes.as_slice().try_into().map_err(|_| "artifact key width")?,
        key_generation: carrier.key_generation, role: "maintainer".into(),
    })
}
