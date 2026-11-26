use std::path::Path;
use std::path::PathBuf;

use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use molten::world_promotion::SignedIntentClosure;
use molten::world_promotion::SignedPromotionAuthority;
use molten::world_promotion::WORLD_PROMOTION_GRANT_SCHEMA;
use molten::world_promotion::WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA;
use molten::world_promotion::canonical_intent_closure;
use molten::world_promotion::canonical_promotion_grant;
use serde_json::json;

use super::WorldHeadPolicyRef;
use super::publish_intent;
use super::read;
use super::reference;
use super::reference_for_adapter;
use super::write;

/// Far-future grant expiry (2100-01-01T00:00:00Z) in Unix seconds.
pub const GRANT_EXPIRES_AT: u64 = 4_102_444_800;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[11_u8; 32])
}

pub fn nonce(label: &str) -> String {
    blake3::hash(label.as_bytes()).to_hex().to_string()
}

/// Signs the canonical record of `authority`'s typed grant with the trusted operator key.
pub fn sign_authority(authority: &mut SignedPromotionAuthority) {
    let grant = authority.grant().expect("typed grant");
    let canonical = canonical_promotion_grant(&grant).expect("canonical grant");
    authority.signer = hex::encode(signing_key().verifying_key().to_bytes());
    authority.signature = hex::encode(signing_key().sign(&canonical.bytes).to_bytes());
}

fn sign_closure(closure: &mut SignedIntentClosure) {
    let canonical = canonical_intent_closure(closure).expect("canonical closure");
    closure.signer = hex::encode(signing_key().verifying_key().to_bytes());
    closure.signature = hex::encode(signing_key().sign(&canonical.bytes).to_bytes());
}

fn write_authority(path: &Path, authority: &SignedPromotionAuthority) {
    write(path, serde_json::to_value(authority).expect("authority json"));
}

fn read_authority(path: &Path) -> SignedPromotionAuthority {
    serde_json::from_value(read(path)).expect("authority document")
}

/// Signed promotion fixtures whose intent material is published.
pub fn fixtures(root: &Path, policy_ref: &WorldHeadPolicyRef) -> (String, PathBuf, PathBuf, PathBuf) {
    let fixtures = fixtures_naming_material(root, policy_ref, "adapter");
    publish_intent(root, "adapter");
    fixtures
}

/// Signed promotion fixtures whose request declares adapter `adapter` for the
/// release intent while the intent reference (bound by the request and the
/// signed closure) addresses the material for `material_adapter`. Nothing is published.
pub fn fixtures_naming_material(
    root: &Path,
    policy_ref: &WorldHeadPolicyRef,
    material_adapter: &str,
) -> (String, PathBuf, PathBuf, PathBuf) {
    let signer_hex = hex::encode(signing_key().verifying_key().to_bytes());
    let authority_ref = reference("authority");
    let operation_ref = reference("operation");
    let intent_ref = reference_for_adapter(material_adapter);
    let candidate = reference("candidate");
    let mut authority = SignedPromotionAuthority {
        schema: WORLD_PROMOTION_GRANT_SCHEMA.to_string(),
        action: "promote".to_string(),
        branch_id: "release".to_string(),
        authority_ref: authority_ref.clone(),
        policy_ref: policy_ref.as_str().to_string(),
        generation: 1,
        admitted: true,
        candidate_head: Some(candidate.clone()),
        reservation_ref: None,
        attempt_ref: None,
        nonce: nonce("promote"),
        expires_at: GRANT_EXPIRES_AT,
        signer: String::new(),
        signature: String::new(),
    };
    sign_authority(&mut authority);
    let mut closure = SignedIntentClosure {
        schema: WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA.to_string(),
        operation_ref: operation_ref.clone(),
        branch_id: "release".to_string(),
        candidate_head: candidate.clone(),
        expected_generation: 1,
        intent_refs: vec![intent_ref.clone()],
        complete: true,
        signer: String::new(),
        signature: String::new(),
    };
    sign_closure(&mut closure);
    let request = json!({
        "operation_ref": operation_ref,
        "branch_id": "release",
        "expected_head": reference("active"),
        "candidate_head": candidate,
        "expected_generation": 1,
        "policy_ref": policy_ref.as_str(),
        "authority_ref": authority_ref,
        "authority_admitted": false,
        "intent_closure_complete": false,
        "simulation_only": false,
        "intents": [{
            "intent_ref": intent_ref,
            "semantic_ref": reference("semantic"),
            "handler_ref": reference("handler"),
            "adapter_ref": reference("adapter"),
            "release_class": "release",
        }],
    });
    let request_path = root.join("request.json");
    let authority_path = root.join("authority.json");
    let closure_path = root.join("closure.json");
    write(&request_path, request);
    write_authority(&authority_path, &authority);
    write(&closure_path, serde_json::to_value(&closure).expect("closure json"));
    (signer_hex, request_path, authority_path, closure_path)
}

/// Re-signs the grant at `path` with `admitted`.
pub fn resign_authority(path: &Path, admitted: bool) {
    let mut authority = read_authority(path);
    authority.admitted = admitted;
    sign_authority(&mut authority);
    write_authority(path, &authority);
}

/// Edits the grant at `path` with `edit`, re-signs it, and writes it back.
pub fn edit_authority(path: &Path, edit: impl FnOnce(&mut SignedPromotionAuthority)) {
    let mut authority = read_authority(path);
    edit(&mut authority);
    sign_authority(&mut authority);
    write_authority(path, &authority);
}

/// One operator grant derived from the promotion grant: its action, exact target, generation,
/// admission and single-use nonce.
pub struct OperatorGrant<'a> {
    pub action: &'a str,
    pub reservation_ref: &'a str,
    pub attempt_ref: Option<&'a str>,
    pub generation: u64,
    pub admitted: bool,
    pub nonce_label: &'a str,
}

/// Writes to `target` the grant at `source` re-issued and re-signed by the trusted operator key
/// as `grant`.
pub fn write_operator_authority(source: &Path, target: &Path, grant: &OperatorGrant<'_>) {
    let mut authority = read_authority(source);
    authority.action = grant.action.to_string();
    authority.candidate_head = None;
    authority.reservation_ref = Some(grant.reservation_ref.to_string());
    authority.attempt_ref = grant.attempt_ref.map(ToString::to_string);
    authority.generation = grant.generation;
    authority.admitted = grant.admitted;
    authority.nonce = nonce(grant.nonce_label);
    sign_authority(&mut authority);
    write_authority(target, &authority);
}

pub fn resign_closure(path: &Path, complete: bool) {
    let mut closure: SignedIntentClosure = serde_json::from_value(read(path)).expect("closure document");
    closure.complete = complete;
    sign_closure(&mut closure);
    write(path, serde_json::to_value(&closure).expect("closure json"));
}
