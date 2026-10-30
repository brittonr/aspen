//! Pure verification of independently admitted public readiness observations; signatures grant no
//! authority.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedReadiness {
    pub probe_public_key: String,
    pub challenge: crate::live_reliability::authority::Challenge,
    pub state: crate::live_reliability::authority::ProbeState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessEvidence {
    pub schema: String,
    pub node_id: String,
    pub endpoint: String,
    pub grant_ref: String,
    pub generation: u64,
    pub source_binding_ref: String,
    pub startup_ref: String,
    pub health_ref: String,
    pub probe_ref: String,
    pub signed_probe: crate::live_reliability::authority::SignedProbe,
    pub experimental_ready: bool,
    pub production_admitted: bool,
}

// r[impl molten.testing.live_reliability_evidence]
// r[impl molten.consensus.live_recovery]
pub fn validate(
    endpoints: &std::collections::BTreeSet<String>,
    expected: &std::collections::BTreeMap<String, ExpectedReadiness>,
    evidence: &std::collections::BTreeMap<String, ReadinessEvidence>,
) -> Result<(), crate::live_reliability::evidence::Issue> {
    let issue = crate::live_reliability::evidence::Issue::Recovery;
    if endpoints.is_empty()
        || endpoints.len() > 64
        || expected.len() != endpoints.len()
        || evidence.len() != endpoints.len()
    {
        return Err(issue);
    }
    for endpoint in endpoints {
        let expected = expected.get(endpoint).ok_or(issue)?;
        let proof = evidence.get(endpoint).ok_or(issue)?;
        validate_endpoint(endpoint, expected, proof)?;
    }
    Ok(())
}

fn validate_endpoint(
    endpoint: &str,
    expected: &ExpectedReadiness,
    proof: &ReadinessEvidence,
) -> Result<(), crate::live_reliability::evidence::Issue> {
    let issue = crate::live_reliability::evidence::Issue::Recovery;
    let challenge = &expected.challenge;
    let state = &expected.state;
    if challenge.endpoint != endpoint
        || challenge.nonce == [0; 32]
        || state.generation == 0
        || !state.ready
        || expected.probe_public_key.is_empty()
        || expected.probe_public_key.len() > 128
        || [
            &challenge.node_id,
            &challenge.endpoint,
            &challenge.source_binding_ref,
            &challenge.startup_ref,
            &challenge.health_ref,
            &state.grant_ref,
            &state.group_ref,
            &state.membership_ref,
            &state.transport_session_ref,
            &state.bootstrap_ref,
        ]
        .iter()
        .any(|v| v.is_empty() || v.len() > 512)
        || proof.signed_probe.signature.len() != 64
    {
        return Err(issue);
    }
    // The key, nonce, current startup/health and state come from the independent expected record,
    // never from the public peer's signed payload or its claimed verifier output.
    let observed = crate::live_reliability::authority::verify_peer_probe(
        &expected.probe_public_key,
        &proof.signed_probe,
        challenge,
    )
    .map_err(|_| issue)?;
    if observed != *state
        || proof.schema != "molten.experimental-readiness.v1"
        || proof.node_id != challenge.node_id
        || proof.endpoint != endpoint
        || proof.source_binding_ref != challenge.source_binding_ref
        || proof.startup_ref != challenge.startup_ref
        || proof.health_ref != challenge.health_ref
        || proof.grant_ref != state.grant_ref
        || proof.generation != state.generation
        || !proof.experimental_ready
        || proof.production_admitted
    {
        return Err(issue);
    }
    let probe_ref = crate::live_reliability::profile::canonical_ref(&proof.signed_probe).map_err(|_| issue)?;
    if proof.probe_ref != probe_ref {
        return Err(issue);
    }
    Ok(())
}

#[cfg(test)]
#[path = "evidenceprooftests.rs"]
pub(crate) mod tests;
