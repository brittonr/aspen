fn fixture() -> (iroh::SecretKey, iroh::SecretKey, super::SignedPolicy, super::SignedGrant) {
    let operator = iroh::SecretKey::from_bytes(&[7; 32]);
    let probe = iroh::SecretKey::from_bytes(&[9; 32]);
    let policy = super::Policy {
        schema: "molten.coordination-policy.v1".into(),
        generation: 2,
        operator_public_key: operator.public().to_string(),
        allowed_nodes: ["node-a".into()].into(),
        allowed_groups: ["group-a".into()].into(),
        allowed_source_bindings: ["source-a".into()].into(),
        allowed_operations: ["read".into(), "write".into(), "probe".into()].into(),
        not_before_ms: 100,
        expires_at_ms: 1000,
        max_grant_lifetime_ms: 500,
    };
    let grant = super::Grant {
        schema: "molten.coordination-grant.v1".into(),
        grant_id: "test-grant".into(),
        policy_generation: 2,
        generation: 3,
        node_id: "node-a".into(),
        group_ref: "group-a".into(),
        membership_ref: "members-a".into(),
        source_binding_ref: "source-a".into(),
        endpoint: "127.0.0.1:9000".into(),
        transport_session_ref: "session-a".into(),
        allowed_operations: ["read".into(), "probe".into()].into(),
        not_before_ms: 200,
        expires_at_ms: 600,
        probe_public_key: probe.public().to_string(),
        client_public_key: iroh::SecretKey::from_bytes(&[11; 32]).public().to_string(),
        client_scope_ref: "client-scope-a".into(),
        configuration: serde_json::json!({"bindings": ["transport", "time"], "timer": 50}),
    };
    let policy = super::SignedPolicy {
        signature: super::canonical::sign("operator-policy", &policy, &operator).unwrap(),
        policy,
    };
    let grant = super::SignedGrant {
        signature: super::canonical::sign("operator-grant", &grant, &operator).unwrap(),
        grant,
    };
    (operator, probe, policy, grant)
}

fn challenge() -> super::Challenge {
    super::Challenge {
        nonce: [13; 32],
        node_id: "node-a".into(),
        endpoint: "127.0.0.1:9000".into(),
        source_binding_ref: "source-a".into(),
        startup_ref: "startup-a".into(),
        health_ref: "health-a".into(),
    }
}

fn state(grant: &super::VerifiedGrant) -> super::ProbeState {
    super::ProbeState {
        grant_ref: grant.grant_ref().into(),
        generation: 3,
        group_ref: "group-a".into(),
        membership_ref: "members-a".into(),
        transport_session_ref: "session-a".into(),
        bootstrap_ref: "bootstrap-a".into(),
        ready: true,
    }
}

#[test]
fn explicit_issuer_admits_grant_and_dedicated_probe_verifies() {
    let (operator, probe, policy, signed) = fixture();
    let grant = super::issuance::verify(&policy, &signed, &operator.public().to_string(), 300).unwrap();
    assert!(grant.allows("read"));
    assert!(!grant.allows("write"));
    let challenge = challenge();
    let signed = super::sign_probe(&probe, &grant, &challenge, state(&grant)).unwrap();
    assert_eq!(super::verify_probe(&grant, &signed, &challenge).unwrap(), state(&grant));
}

#[test]
fn rejects_untrusted_issuer_expired_policy_and_changed_grant_configuration() {
    let (operator, _, policy, mut signed) = fixture();
    let issuer = operator.public().to_string();
    assert!(
        super::issuance::verify(&policy, &signed, &iroh::SecretKey::from_bytes(&[11; 32]).public().to_string(), 300)
            .is_err()
    );
    assert!(super::issuance::verify(&policy, &signed, &issuer, 600).is_err());
    assert!(super::issuance::verify(&policy, &signed, &issuer, 199).is_err());
    signed.grant.configuration["timer"] = serde_json::json!(51);
    assert!(super::issuance::verify(&policy, &signed, &issuer, 300).is_err());
}

#[test]
fn signed_but_outside_policy_cannot_authorize_effects() {
    let (operator, _, mut policy, mut signed) = fixture();
    let issuer = operator.public().to_string();
    signed.grant.allowed_operations.insert("delete".into());
    signed.signature = super::canonical::sign("operator-grant", &signed.grant, &operator).unwrap();
    assert!(super::issuance::verify(&policy, &signed, &issuer, 300).is_err());
    signed.grant.allowed_operations.remove("delete");
    signed.signature = super::canonical::sign("operator-grant", &signed.grant, &operator).unwrap();
    policy.policy.generation += 1;
    policy.signature = super::canonical::sign("operator-policy", &policy.policy, &operator).unwrap();
    assert!(super::issuance::verify(&policy, &signed, &issuer, 300).is_err());
}

#[test]
fn probe_rejects_replay_state_tamper_wrong_key_and_purpose_confusion() {
    let (operator, probe, policy, signed) = fixture();
    let grant = super::issuance::verify(&policy, &signed, &operator.public().to_string(), 300).unwrap();
    let challenge = challenge();
    let mut signed = super::sign_probe(&probe, &grant, &challenge, state(&grant)).unwrap();
    let mut other = challenge.clone();
    other.nonce[0] += 1;
    assert!(super::verify_probe(&grant, &signed, &other).is_err());
    assert!(super::sign_probe(&operator, &grant, &challenge, state(&grant)).is_err());
    signed.state.ready = false;
    assert!(super::verify_probe(&grant, &signed, &challenge).is_err());
    signed.state.ready = true;
    signed.signature = super::canonical::sign("operator-grant", &(&challenge, &signed.state), &probe).unwrap();
    assert!(super::verify_probe(&grant, &signed, &challenge).is_err());
}

#[test]
fn client_invocation_binds_exact_request_and_distinct_key() {
    let (operator, probe, policy, signed) = fixture();
    let grant = super::issuance::verify(&policy, &signed, &operator.public().to_string(), 300).unwrap();
    let client = iroh::SecretKey::from_bytes(&[11; 32]);
    let request = serde_json::json!({"phase": "invoke", "operation": {"id": "a", "value": 42}});
    let signed = super::sign_request(&client, &grant, &request, [15; 32]).unwrap();
    assert!(super::verify_request(&grant, &request, &signed).is_ok());
    let changed = serde_json::json!({"phase": "invoke", "operation": {"id": "a", "value": 43}});
    assert!(super::verify_request(&grant, &changed, &signed).is_err());
    assert!(super::sign_request(&probe, &grant, &request, [15; 32]).is_err());
    let mut tampered = signed;
    tampered.nonce[0] += 1;
    assert!(super::verify_request(&grant, &request, &tampered).is_err());
}

#[test]
fn forwarding_uses_explicit_shared_scope_not_node_grant_hash() {
    let (operator, _, policy, mut signed) = fixture();
    let issuer = operator.public().to_string();
    let first = super::issuance::verify(&policy, &signed, &issuer, 300).unwrap();
    let client = iroh::SecretKey::from_bytes(&[11; 32]);
    let request = serde_json::json!({"endpoint": "127.0.0.1:9000", "operation": "read"});
    let auth = super::sign_request(&client, &first, &request, [17; 32]).unwrap();
    signed.grant.grant_id = "second-node-grant".into();
    signed.grant.endpoint = "127.0.0.1:9001".into();
    signed.signature = super::canonical::sign("operator-grant", &signed.grant, &operator).unwrap();
    let second = super::issuance::verify(&policy, &signed, &issuer, 300).unwrap();
    assert_ne!(first.grant_ref(), second.grant_ref());
    assert!(super::verify_request(&second, &request, &auth).is_ok());
    signed.grant.client_scope_ref = "other-client-authority".into();
    signed.signature = super::canonical::sign("operator-grant", &signed.grant, &operator).unwrap();
    let unrelated = super::issuance::verify(&policy, &signed, &issuer, 300).unwrap();
    assert!(super::verify_request(&unrelated, &request, &auth).is_err());
}

#[test]
fn peer_crypto_proof_rejects_wrong_key_nonce_and_state_tamper() {
    let (operator, probe, policy, signed) = fixture();
    let grant = super::issuance::verify(&policy, &signed, &operator.public().to_string(), 300).unwrap();
    let challenge = challenge();
    let mut proof = super::sign_probe(&probe, &grant, &challenge, state(&grant)).unwrap();
    assert_eq!(super::verify_peer_probe(&probe.public().to_string(), &proof, &challenge).unwrap(), state(&grant));
    assert!(super::verify_peer_probe(&operator.public().to_string(), &proof, &challenge).is_err());
    let mut changed = challenge.clone();
    changed.nonce[0] += 1;
    assert!(super::verify_peer_probe(&probe.public().to_string(), &proof, &changed).is_err());
    proof.state.bootstrap_ref = "other-bootstrap".into();
    assert!(super::verify_peer_probe(&probe.public().to_string(), &proof, &challenge).is_err());
}
