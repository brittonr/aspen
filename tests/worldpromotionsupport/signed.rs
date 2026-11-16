use std::path::Path;
use std::path::PathBuf;

use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use serde_json::json;

use super::WorldHeadPolicyRef;
use super::publish_intent;
use super::read;
use super::reference;
use super::reference_for_adapter;
use super::write;

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
    let signer = SigningKey::from_bytes(&[11_u8; 32]);
    let signer_hex = hex::encode(signer.verifying_key().to_bytes());
    let authority_ref = reference("authority");
    let operation_ref = reference("operation");
    let intent_ref = reference_for_adapter(material_adapter);
    let candidate = reference("candidate");
    let authority_message = format!(
        "molten.world-promotion.authority-grant.v1\nrelease\n{authority_ref}\n{}\n1\ntrue",
        policy_ref.as_str()
    );
    let authority = json!({
        "schema": "molten.world-promotion.authority-grant.v1",
        "branch_id": "release",
        "authority_ref": authority_ref,
        "policy_ref": policy_ref.as_str(),
        "generation": 1,
        "admitted": true,
        "signer": signer_hex,
        "signature": hex::encode(signer.sign(authority_message.as_bytes()).to_bytes()),
    });
    let closure_message = format!(
        "molten.world-promotion.intent-closure.v1\n{operation_ref}\nrelease\n{candidate}\n1\ntrue\n{intent_ref}"
    );
    let closure = json!({
        "schema": "molten.world-promotion.intent-closure.v1",
        "operation_ref": operation_ref,
        "branch_id": "release",
        "candidate_head": candidate,
        "expected_generation": 1,
        "complete": true,
        "intent_refs": [intent_ref],
        "signer": signer_hex,
        "signature": hex::encode(signer.sign(closure_message.as_bytes()).to_bytes()),
    });
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
    write(&authority_path, authority);
    write(&closure_path, closure);
    (signer_hex, request_path, authority_path, closure_path)
}

pub fn resign_authority(path: &Path, admitted: bool) {
    let generation = read(path)["generation"].as_u64().expect("generation");
    write_operator_authority(path, path, generation, admitted);
}

/// Writes to `target` the grant at `source` re-signed by the trusted operator key
/// for `generation` and `admitted`.
pub fn write_operator_authority(source: &Path, target: &Path, generation: u64, admitted: bool) {
    let mut authority = read(source);
    authority["generation"] = serde_json::Value::from(generation);
    authority["admitted"] = serde_json::Value::Bool(admitted);
    let message = format!(
        "molten.world-promotion.authority-grant.v1\n{}\n{}\n{}\n{}\n{}",
        authority["branch_id"].as_str().expect("branch"),
        authority["authority_ref"].as_str().expect("authority"),
        authority["policy_ref"].as_str().expect("policy"),
        generation,
        admitted
    );
    let signer = SigningKey::from_bytes(&[11_u8; 32]);
    authority["signature"] = serde_json::Value::String(hex::encode(signer.sign(message.as_bytes()).to_bytes()));
    write(target, authority);
}

pub fn resign_closure(path: &Path, complete: bool) {
    let mut closure = read(path);
    closure["complete"] = serde_json::Value::Bool(complete);
    let intents = closure["intent_refs"]
        .as_array()
        .expect("intents")
        .iter()
        .map(|value| value.as_str().expect("intent").to_string())
        .collect::<Vec<_>>()
        .join(",");
    let message = format!(
        "molten.world-promotion.intent-closure.v1\n{}\n{}\n{}\n{}\n{}\n{}",
        closure["operation_ref"].as_str().expect("operation"),
        closure["branch_id"].as_str().expect("branch"),
        closure["candidate_head"].as_str().expect("candidate"),
        closure["expected_generation"].as_u64().expect("generation"),
        complete,
        intents
    );
    let signer = SigningKey::from_bytes(&[11_u8; 32]);
    closure["signature"] = serde_json::Value::String(hex::encode(signer.sign(message.as_bytes()).to_bytes()));
    write(path, closure);
}
