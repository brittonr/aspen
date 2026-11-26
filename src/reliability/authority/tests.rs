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
    let signed = super::sign_request(&client, &grant, &request, [15; 32], LEADER_TERM).unwrap();
    assert!(super::verify_request(&grant, &request, &signed).is_ok());
    let changed = serde_json::json!({"phase": "invoke", "operation": {"id": "a", "value": 43}});
    assert!(super::verify_request(&grant, &changed, &signed).is_err());
    assert!(super::sign_request(&probe, &grant, &request, [15; 32], LEADER_TERM).is_err());
    let mut tampered = signed.clone();
    tampered.nonce[0] += 1;
    assert!(super::verify_request(&grant, &request, &tampered).is_err());
    let mut refenced = signed;
    refenced.fence_term += 1;
    assert!(
        super::verify_request(&grant, &request, &refenced).is_err(),
        "the fence term is inside the signature"
    );
}

const LEADER_TERM: u64 = 5;
const FAILOVER_TERM: u64 = 6;

/// The executing leader's admission of one signed write, in host order: exact signature and
/// durable nonce marker (`Host::admit`), then the leader-term fence (`Host::perform`) before any
/// proposal.
fn leader_admits_write(
    grant: &super::VerifiedGrant,
    root: &crate::node_state::DirectoryRoot,
    request: &serde_json::Value,
    invocation: &super::SignedInvocation,
    leader_term: u64,
) -> crate::error::Result<()> {
    super::verify_request(grant, request, invocation)?;
    super::consume_nonce(root, invocation)?;
    super::require_write_fence(invocation, leader_term)
}

// r[verify molten.consensus.reliability_replay_and_failstop]
#[test]
fn signed_write_replays_are_denied_across_failover_and_restart_while_a_resigned_retry_is_admitted() {
    let (operator, _, policy, signed) = fixture();
    let grant = super::issuance::verify(&policy, &signed, &operator.public().to_string(), 300).unwrap();
    let client = iroh::SecretKey::from_bytes(&[11; 32]);
    let request = serde_json::json!({"phase": "invoke", "operation": {"id": "w", "function": "write", "value": 1}});
    let workspace_a = crate::test_support::process_workspace("fenced-write-leader-a").expect("node A root");
    let workspace_b = crate::test_support::process_workspace("fenced-write-leader-b").expect("node B root");
    let node_a = crate::node_state::DirectoryRoot::open_existing(&workspace_a).expect("node A");
    let node_b = crate::node_state::DirectoryRoot::open_existing(&workspace_b).expect("node B");

    let write = super::sign_request(&client, &grant, &request, [21; 32], LEADER_TERM).unwrap();
    leader_admits_write(&grant, &node_a, &request, &write, LEADER_TERM).expect("the term leader admits the write");

    // Same term, same leader, after its node state is reopened: Raft compaction never touches the
    // durable marker, so the replay is denied whatever the request window still holds.
    let reopened = crate::node_state::DirectoryRoot::open_existing(&workspace_a).expect("reopened node A");
    assert!(leader_admits_write(&grant, &reopened, &request, &write, LEADER_TERM).is_err());

    // Failover: node B never saw the nonce, but it leads a later term, so the fence denies the
    // replay before any proposal.
    let replay = leader_admits_write(&grant, &node_b, &request, &write, FAILOVER_TERM).expect_err("failover replay");
    assert!(replay.to_string().contains("fenced to Raft term"));

    // A legitimate retry is re-signed with a fresh nonce for the current term.
    let retry = super::sign_request(&client, &grant, &request, [22; 32], FAILOVER_TERM).unwrap();
    leader_admits_write(&grant, &node_b, &request, &retry, FAILOVER_TERM).expect("re-signed retry");

    // Stale and future fences are denied before any effect.
    for (nonce, fence_term) in [([23; 32], LEADER_TERM), ([24; 32], FAILOVER_TERM + 1)] {
        let fenced = super::sign_request(&client, &grant, &request, nonce, fence_term).unwrap();
        assert!(super::require_write_fence(&fenced, FAILOVER_TERM).is_err(), "fence term {fence_term}");
    }
}

#[test]
fn forwarding_uses_explicit_shared_scope_not_node_grant_hash() {
    let (operator, _, policy, mut signed) = fixture();
    let issuer = operator.public().to_string();
    let first = super::issuance::verify(&policy, &signed, &issuer, 300).unwrap();
    let client = iroh::SecretKey::from_bytes(&[11; 32]);
    let request = serde_json::json!({"endpoint": "127.0.0.1:9000", "operation": "read"});
    let auth = super::sign_request(&client, &first, &request, [17; 32], LEADER_TERM).unwrap();
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
