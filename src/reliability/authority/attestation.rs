#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub nonce: [u8; 32],
    pub node_id: String,
    pub endpoint: String,
    pub source_binding_ref: String,
    pub startup_ref: String,
    pub health_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeState {
    pub grant_ref: String,
    pub generation: u64,
    pub group_ref: String,
    pub membership_ref: String,
    pub transport_session_ref: String,
    pub bootstrap_ref: String,
    pub ready: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProbe {
    pub challenge: Challenge,
    pub state: ProbeState,
    pub signature: Vec<u8>,
}

pub fn sign_probe(
    key: &iroh::SecretKey,
    grant: &super::VerifiedGrant,
    challenge: &Challenge,
    state: ProbeState,
) -> crate::error::Result<SignedProbe> {
    if key.public().to_string() != grant.probe_public_key() {
        return Err(super::denied("probe signer not bound to verified grant"));
    }
    validate(grant, challenge, &state)?;
    let payload = (challenge, &state);
    let signature = super::canonical::sign("live-readiness-probe", &payload, key)?;
    Ok(SignedProbe {
        challenge: challenge.clone(),
        state,
        signature,
    })
}

pub fn verify_probe(
    grant: &super::VerifiedGrant,
    signed: &SignedProbe,
    challenge: &Challenge,
) -> crate::error::Result<ProbeState> {
    validate(grant, challenge, &signed.state)?;
    verify_peer_probe(grant.probe_public_key(), signed, challenge)
}

/// Cryptographic proof only: the caller must independently admit this key and every peer-state
/// binding. A valid signature alone grants neither quorum authority nor permission to perform
/// effects.
pub fn verify_peer_probe(
    public_key: &str,
    signed: &SignedProbe,
    challenge: &Challenge,
) -> crate::error::Result<ProbeState> {
    if challenge.nonce == [0; 32] || &signed.challenge != challenge {
        return Err(super::denied("peer probe challenge replay or identity mismatch"));
    }
    super::canonical::verify("live-readiness-probe", &(challenge, &signed.state), public_key, &signed.signature)?;
    Ok(signed.state.clone())
}

fn validate(grant: &super::VerifiedGrant, challenge: &Challenge, state: &ProbeState) -> crate::error::Result<()> {
    let g = grant.grant();
    if challenge.nonce == [0; 32]
        || challenge.node_id != g.node_id
        || challenge.endpoint != g.endpoint
        || challenge.source_binding_ref != g.source_binding_ref
        || challenge.startup_ref.is_empty()
        || challenge.health_ref.is_empty()
        || state.grant_ref != grant.grant_ref()
        || state.generation != g.generation
        || state.group_ref != g.group_ref
        || state.membership_ref != g.membership_ref
        || state.transport_session_ref != g.transport_session_ref
        || state.bootstrap_ref.is_empty()
    {
        return Err(super::denied("probe state differs from verified authority or challenge"));
    }
    Ok(())
}
