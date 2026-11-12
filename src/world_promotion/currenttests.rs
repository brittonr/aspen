use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;

use super::current::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn authority(key: &SigningKey) -> SignedPromotionAuthority {
    let mut authority = SignedPromotionAuthority {
        schema: "molten.world-promotion.authority-grant.v1".to_string(),
        branch_id: "release".to_string(),
        authority_ref: "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        policy_ref: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        generation: 1,
        admitted: true,
        signer: hex(key.verifying_key().as_bytes()),
        signature: String::new(),
    };
    authority.signature = hex(&key.sign(&authority_message(&authority)).to_bytes());
    authority
}

fn closure(key: &SigningKey) -> SignedIntentClosure {
    let mut closure = SignedIntentClosure {
        schema: "molten.world-promotion.intent-closure.v1".to_string(),
        operation_ref: "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string(),
        branch_id: "release".to_string(),
        candidate_head: "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string(),
        expected_generation: 1,
        intent_refs: vec!["blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string()],
        complete: true,
        signer: hex(key.verifying_key().as_bytes()),
        signature: String::new(),
    };
    closure.signature = hex(&key.sign(&closure_message(&closure)).to_bytes());
    closure
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

    let authority = read_authority(&authority_path, &signer).expect("authority");
    let closure = read_closure(&closure_path, &signer).expect("closure");
    assert!(authority.admitted);
    assert!(closure.complete);
}

#[test]
fn tampered_authority_or_wrong_trusted_signer_is_denied() {
    let key = key();
    let directory = tempfile::tempdir().expect("temporary directory");
    let signer = hex(key.verifying_key().as_bytes());
    let mut tampered = authority(&key);
    tampered.generation = 2;
    let path = write(directory.path(), "tampered.json", &tampered);
    assert!(read_authority(&path, &signer).is_err());
    assert!(read_authority(&path, &hex(&[9_u8; 32])).is_err());
}

#[test]
fn malformed_hex_and_unsupported_schema_are_denied() {
    let key = key();
    let directory = tempfile::tempdir().expect("temporary directory");
    let signer = hex(key.verifying_key().as_bytes());
    let mut bad = authority(&key);
    bad.signer = "zz".to_string();
    let path = write(directory.path(), "bad-signer.json", &bad);
    assert!(read_authority(&path, &signer).is_err());

    let mut schema = closure(&key);
    schema.schema = "unknown".to_string();
    let path = write(directory.path(), "bad-schema.json", &schema);
    assert!(read_closure(&path, &signer).is_err());
}
