use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use molten_core::world_promotion::WorldPromotionGrantTarget;

use super::current::*;
use super::records::*;

const EXPIRES_AT: u64 = 4_102_444_800;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn sign_authority(key: &SigningKey, authority: &mut SignedPromotionAuthority) {
    let grant = authority.grant().expect("typed grant");
    let canonical = canonical_promotion_grant(&grant).expect("canonical grant");
    authority.signature = hex(&key.sign(&canonical.bytes).to_bytes());
}

fn sign_closure(key: &SigningKey, closure: &mut SignedIntentClosure) {
    let canonical = canonical_intent_closure(closure).expect("canonical closure");
    closure.signature = hex(&key.sign(&canonical.bytes).to_bytes());
}

fn unsigned_authority(key: &SigningKey) -> SignedPromotionAuthority {
    SignedPromotionAuthority {
        schema: WORLD_PROMOTION_GRANT_SCHEMA.to_string(),
        action: "promote".to_string(),
        branch_id: "release".to_string(),
        authority_ref: reference("authority"),
        policy_ref: reference("policy"),
        generation: 1,
        admitted: true,
        candidate_head: Some(reference("candidate")),
        reservation_ref: None,
        attempt_ref: None,
        nonce: blake3::hash(b"nonce").to_hex().to_string(),
        expires_at: EXPIRES_AT,
        signer: hex(key.verifying_key().as_bytes()),
        signature: String::new(),
    }
}

fn authority(key: &SigningKey) -> SignedPromotionAuthority {
    let mut authority = unsigned_authority(key);
    sign_authority(key, &mut authority);
    authority
}

fn closure_with(key: &SigningKey, operation_ref: &str, branch_id: &str, intent_refs: &[&str]) -> SignedIntentClosure {
    let mut closure = SignedIntentClosure {
        schema: WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA.to_string(),
        operation_ref: operation_ref.to_string(),
        branch_id: branch_id.to_string(),
        candidate_head: reference("candidate"),
        expected_generation: 1,
        intent_refs: intent_refs.iter().map(ToString::to_string).collect(),
        complete: true,
        signer: hex(key.verifying_key().as_bytes()),
        signature: String::new(),
    };
    sign_closure(key, &mut closure);
    closure
}

fn closure(key: &SigningKey) -> SignedIntentClosure {
    closure_with(key, &reference("operation"), "release", &[reference("intent").as_str()])
}

fn write(directory: &std::path::Path, name: &str, value: &impl serde::Serialize) -> std::path::PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(value).expect("serialize fixture")).expect("write fixture");
    path
}

#[test]
fn signed_authority_and_closure_verify_with_the_trusted_signer() {
    let key = key();
    let directory = tempfile::tempdir().expect("temporary directory");
    let signer = hex(key.verifying_key().as_bytes());
    let authority_path = write(directory.path(), "authority.json", &authority(&key));
    let closure_path = write(directory.path(), "closure.json", &closure(&key));

    let grant = read_promotion_grant(&authority_path, &signer).expect("authority");
    let closure = read_closure(&closure_path, &signer).expect("closure");
    assert!(grant.admitted);
    assert_eq!(grant.expires_at, EXPIRES_AT);
    assert!(matches!(grant.target, WorldPromotionGrantTarget::Promote { .. }));
    assert!(closure.complete);
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn tampered_action_target_nonce_or_expiry_or_wrong_trusted_signer_is_denied() {
    let key = key();
    let signer = hex(key.verifying_key().as_bytes());
    let signed = authority(&key);
    assert!(verify_promotion_grant(&signed, &signer).is_ok());
    assert!(verify_promotion_grant(&signed, &hex(&[9_u8; 32])).is_err());
    let tampers: [fn(&mut SignedPromotionAuthority); 6] = [
        |authority| authority.generation = 2,
        |authority| authority.candidate_head = Some(reference("other-candidate")),
        |authority| authority.nonce = blake3::hash(b"other-nonce").to_hex().to_string(),
        |authority| authority.expires_at = EXPIRES_AT + 1,
        |authority| authority.admitted = false,
        |authority| {
            authority.action = "deny-reservation".to_string();
            authority.candidate_head = None;
            authority.reservation_ref = Some(reference("reservation"));
        },
    ];
    for tamper in tampers {
        let mut tampered = signed.clone();
        tamper(&mut tampered);
        assert!(verify_promotion_grant(&tampered, &signer).is_err(), "tampered grant {tampered:?} verified");
    }
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn grant_target_fields_must_be_exactly_those_of_its_action() {
    let key = key();
    let signer = hex(key.verifying_key().as_bytes());
    let mut extra = unsigned_authority(&key);
    extra.reservation_ref = Some(reference("reservation"));
    assert!(extra.grant().is_err());
    let mut missing = unsigned_authority(&key);
    missing.action = "abandon-uncertain".to_string();
    missing.candidate_head = None;
    missing.reservation_ref = Some(reference("reservation"));
    assert!(missing.grant().is_err());
    let mut unknown = unsigned_authority(&key);
    unknown.action = "approve".to_string();
    assert!(unknown.grant().is_err());
    let mut bad_nonce = unsigned_authority(&key);
    bad_nonce.nonce = "short".to_string();
    assert!(bad_nonce.grant().is_err());

    // Retry and abandon grants over the same reservation and attempt sign different bytes.
    let mut retry = unsigned_authority(&key);
    retry.action = "plan-retry".to_string();
    retry.candidate_head = None;
    retry.reservation_ref = Some(reference("reservation"));
    retry.attempt_ref = Some(reference("attempt"));
    sign_authority(&key, &mut retry);
    let mut abandon = retry.clone();
    abandon.action = "abandon-uncertain".to_string();
    assert!(verify_promotion_grant(&retry, &signer).is_ok());
    assert!(verify_promotion_grant(&abandon, &signer).is_err());
}

// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
#[test]
fn newline_and_comma_injection_cannot_collide_closure_signing_bytes() {
    let key = key();
    let signer = hex(key.verifying_key().as_bytes());
    // Under newline- and comma-joined messages each pair produced identical signed bytes.
    let pairs = [
        (closure_with(&key, "op", "release", &["a", "b"]), closure_with(&key, "op", "release", &["a,b"])),
        (closure_with(&key, "op\nrelease", "x", &["a"]), closure_with(&key, "op", "release\nx", &["a"])),
    ];
    for (left, right) in pairs {
        let left_bytes = canonical_intent_closure(&left).expect("left bytes").bytes;
        let right_bytes = canonical_intent_closure(&right).expect("right bytes").bytes;
        assert_ne!(left_bytes, right_bytes);
        let mut forged = right.clone();
        forged.signature = left.signature.clone();
        assert!(verify_closure(&left, &signer).is_ok());
        assert!(verify_closure(&forged, &signer).is_err(), "a signature over {left:?} verified {right:?}");
    }
}

#[test]
fn malformed_hex_and_unsupported_schema_are_denied() {
    let key = key();
    let directory = tempfile::tempdir().expect("temporary directory");
    let signer = hex(key.verifying_key().as_bytes());
    let mut bad = authority(&key);
    bad.signer = "zz".to_string();
    let path = write(directory.path(), "bad-signer.json", &bad);
    assert!(read_promotion_grant(&path, &signer).is_err());

    let mut old_schema = authority(&key);
    old_schema.schema = "molten.world-promotion.authority-grant.v1".to_string();
    let path = write(directory.path(), "old-authority.json", &old_schema);
    assert!(read_promotion_grant(&path, &signer).is_err());

    let mut schema = closure(&key);
    schema.schema = "unknown".to_string();
    let path = write(directory.path(), "bad-schema.json", &schema);
    assert!(read_closure(&path, &signer).is_err());
}
