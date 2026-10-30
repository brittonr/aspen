pub(super) fn authorize(
    args: &super::Provision,
    selection: &molten::fabric_observability::nodeadmission::NodeObservationSelection,
) -> super::Result<(Vec<u8>, Vec<u8>)> {
    let now = molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)?;
    if args.not_before > now || args.expires <= now || args.not_before >= args.expires {
        return Err("explicit operator validity interval does not include current time".into());
    }
    let mut issuer_seed = super::inputs::seed(&args.issuer_key)?;
    let mut holder_seed = super::inputs::seed(&args.holder_key)?;
    if issuer_seed == holder_seed {
        issuer_seed.fill(0);
        holder_seed.fill(0);
        return Err("observer issuer and holder must be independently supplied distinct keys".into());
    }
    let issuer_key: [u8; 32] =
        basalt::ucan::derive_ed25519_public_key(&issuer_seed).map_err(|_| "issuer public key derivation denied")?;
    let holder_key: [u8; 32] =
        basalt::ucan::derive_ed25519_public_key(&holder_seed).map_err(|_| "holder public key derivation denied")?;
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(issuer_seed);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(holder_seed);
    issuer_seed.fill(0);
    holder_seed.fill(0);
    let audience = basalt::ucan::AudienceDid::from(
        basalt::ucan::TokenSigner::issuer(&holder).map_err(|_| "holder DID derivation denied")?,
    );
    let policy_text = String::from_utf8(super::inputs::read(&args.policy, 262_144)?)
        .map_err(|_| "explicit observer policy must be UTF-8")?;
    let resource = molten::fabric_observability::nodeadmission::node_observation_resource(selection)?;
    let token = token(args, &issuer, &audience, &policy_text, &resource)?;
    let nonce = super::inputs::read(&args.nonce_file, 32)?;
    let credential = molten::fabric_observability::nodeadmission::sign_node_observation_credential(
        molten::fabric_observability::nodeadmission::ObserverSigningInput {
            selection,
            token: token.as_str(),
            audience: &audience,
            nonce: nonce.as_slice().try_into().map_err(|_| "nonce requires exactly 32 raw bytes")?,
        },
        &holder,
    )
    .map_err(|_| "exact holder proof signing denied")?;
    let trust = molten::node_service_host::authority::wire::Trust {
        issuer: basalt::ucan::TokenSigner::issuer(&issuer)
            .map_err(|_| "issuer DID unavailable")?
            .as_str()
            .to_string(),
        issuer_key,
        holder: audience.as_str().to_string(),
        holder_key,
        policy: policy_text,
        not_before: args.not_before,
        expires: args.expires,
        revoked: Vec::new(),
    };
    Ok((
        molten::fabric_observability::nodeadmission::encode_node_observer_trust(&trust)?,
        molten::fabric_observability::nodeadmission::encode_node_observer_credential(&credential)?,
    ))
}

fn token(
    args: &super::Provision,
    issuer: &basalt::ucan::Ed25519InMemorySigner,
    audience: &basalt::ucan::AudienceDid,
    policy_text: &str,
    resource: &str,
) -> super::Result<basalt::ucan::CompactToken> {
    let policy = basalt::parse_policy_json(policy_text).map_err(|_| "explicit observer policy denied")?;
    let request = basalt::EnforcementRequest::new("linux-observation-reader", resource, "read")
        .with_capabilities(vec![basalt::CapabilityGrant::new(resource, "read")]);
    if !basalt::enforce(&policy, &request).map_err(|_| "observer policy evaluation failed")?.is_allowed() {
        return Err("explicit observer policy denies the exact selection read capability".into());
    }
    let capability = basalt::ucan::CapabilityDocument::new(resource.to_owned(), "read".to_owned())
        .map_err(|_| "exact observer capability denied")?;
    let capabilities = basalt::ucan::CapabilitySet::new(vec![capability]).map_err(|_| "capability set denied")?;
    basalt::ucan::issue_token_with_signer(
        issuer,
        audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(args.not_before, args.expires).map_err(|_| "validity denied")?,
    )
    .map_err(|_| "operator UCAN signing denied".into())
}

pub(super) fn revoke(path: &std::path::Path) -> super::Result<()> {
    let root = molten::node_state::DirectoryRoot::open_existing(path)?;
    let startup = root.read(&molten::node_state::RelativePath::parse("linux-observation-startup.json")?, 16_384)?;
    let selection = molten::fabric_observability::nodeadmission::decode_node_observation_selection(&startup)?;
    let locator = molten::node_state::RelativePath::parse("linux-observation-trust")?;
    let mut trust =
        molten::fabric_observability::nodeadmission::decode_node_observer_trust(&root.read(&locator, 262_144)?)?;
    let credential = molten::fabric_observability::nodeadmission::decode_node_observer_credential(
        &root
            .read(&molten::fabric_observability::nodeadmission::node_observer_credential_path(&selection)?, 262_144)?,
    )?;
    let token = basalt::ucan::CompactToken::new(&credential.token).map_err(|_| "installed token syntax denied")?;
    let reference: [u8; 32] = basalt::ucan::proof_reference(&token)
        .as_bytes()
        .try_into()
        .map_err(|_| "installed token reference width")?;
    if !trust.revoked.contains(&reference) {
        trust.revoked.push(reference);
    }
    root.write(&locator, &molten::fabric_observability::nodeadmission::encode_node_observer_trust(&trust)?)?;
    println!(
        "{}",
        serde_json::json!({"schema":"experimental-observer-revoked-v1",
        "selection_ref": molten::fabric_observability::nodeadmission::node_observation_selection_ref(&selection)?})
    );
    Ok(())
}
