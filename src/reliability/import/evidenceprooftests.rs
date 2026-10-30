fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(crate) fn fixture(endpoint: &str) -> (super::ExpectedReadiness, super::ReadinessEvidence) {
    let key = iroh::SecretKey::from_bytes(&[91; 32]);
    let challenge = crate::live_reliability::authority::Challenge {
        nonce: [13; 32],
        node_id: format!("node-{endpoint}"),
        endpoint: endpoint.into(),
        source_binding_ref: reference("source-binding"),
        startup_ref: reference("startup"),
        health_ref: reference("health"),
    };
    let state = crate::live_reliability::authority::ProbeState {
        grant_ref: reference("grant"),
        generation: 3,
        group_ref: reference("group"),
        membership_ref: reference("membership"),
        transport_session_ref: reference("session"),
        bootstrap_ref: reference("bootstrap"),
        ready: true,
    };
    let signed_probe = crate::live_reliability::authority::signed_probe_fixture(&key, challenge.clone(), state.clone());
    let expected = super::ExpectedReadiness {
        probe_public_key: key.public().to_string(),
        challenge: challenge.clone(),
        state: state.clone(),
    };
    let proof = super::ReadinessEvidence {
        schema: "molten.experimental-readiness.v1".into(),
        node_id: challenge.node_id,
        endpoint: challenge.endpoint,
        grant_ref: state.grant_ref,
        generation: state.generation,
        source_binding_ref: challenge.source_binding_ref,
        startup_ref: challenge.startup_ref,
        health_ref: challenge.health_ref,
        probe_ref: crate::live_reliability::profile::canonical_ref(&signed_probe).expect("probe ref"),
        signed_probe,
        experimental_ready: true,
        production_admitted: false,
    };
    (expected, proof)
}

fn verify(
    expected: super::ExpectedReadiness,
    proof: super::ReadinessEvidence,
) -> Result<(), crate::live_reliability::evidence::Issue> {
    let endpoint = expected.challenge.endpoint.clone();
    super::validate(&[endpoint.clone()].into(), &[(endpoint.clone(), expected)].into(), &[(endpoint, proof)].into())
}

// r[verify molten.testing.live_reliability_evidence]
#[test]
fn independently_expected_signed_current_readiness_is_evidence_not_authority() {
    let (expected, proof) = fixture("127.0.0.1:9000");
    assert!(!proof.production_admitted);
    assert_eq!(verify(expected, proof), Ok(()));
}

#[test]
fn forged_readiness_and_signed_state_tampering_are_rejected() {
    let (expected, mut proof) = fixture("127.0.0.1:9000");
    proof.signed_probe.signature[0] ^= 1;
    assert!(verify(expected, proof).is_err());
    let (expected, mut proof) = fixture("127.0.0.1:9000");
    proof.signed_probe.state.membership_ref = reference("forged-membership");
    assert!(verify(expected, proof).is_err());
}

#[test]
fn valid_signature_cannot_choose_its_own_nonce_or_key() {
    let (mut expected, proof) = fixture("127.0.0.1:9000");
    expected.challenge.nonce[0] ^= 1;
    assert!(verify(expected, proof).is_err());
    let (mut expected, proof) = fixture("127.0.0.1:9000");
    expected.probe_public_key = iroh::SecretKey::from_bytes(&[92; 32]).public().to_string();
    assert!(verify(expected, proof).is_err());
}

#[test]
fn valid_signature_for_old_generation_startup_or_session_does_not_match_current_admission() {
    let (expected, proof) = fixture("127.0.0.1:9000");
    let mut changed = expected.clone();
    changed.state.generation += 1;
    assert!(verify(changed, proof.clone()).is_err());
    let mut changed = expected.clone();
    changed.challenge.startup_ref = reference("new-startup");
    assert!(verify(changed, proof.clone()).is_err());
    let mut changed = expected;
    changed.state.transport_session_ref = reference("new-session");
    assert!(verify(changed, proof).is_err());
}

#[test]
fn wrapper_endpoint_probe_identity_and_production_promotion_cannot_replace_signed_facts() {
    let (expected, proof) = fixture("127.0.0.1:9000");
    let mut changed = proof.clone();
    changed.endpoint = "127.0.0.1:9001".into();
    assert!(verify(expected.clone(), changed).is_err());
    let mut changed = proof.clone();
    changed.probe_ref = reference("substitute-probe");
    assert!(verify(expected.clone(), changed).is_err());
    let mut changed = proof;
    changed.production_admitted = true;
    assert!(verify(expected, changed).is_err());
}

#[test]
fn all_endpoints_require_independent_admission_and_complete_signed_observation() {
    let (expected, proof) = fixture("127.0.0.1:9000");
    let endpoints = ["127.0.0.1:9000".into(), "127.0.0.1:9001".into()].into();
    assert!(
        super::validate(
            &endpoints,
            &[("127.0.0.1:9000".into(), expected)].into(),
            &[("127.0.0.1:9000".into(), proof)].into()
        )
        .is_err()
    );
}
