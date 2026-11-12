#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "the integration test names each external type and command at the CLI boundary"
)]
#![allow(
    tigerstyle::no_unwrap,
    reason = "fixture setup fails fast and the test name identifies the exercised path"
)]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use molten::world_head::LocalWorldHeadStore;
use molten::world_head::WorldHeadFreshAdmission;
use molten::world_head::WorldHeadStatePort;
use molten::world_head::WorldHeadTransitionReceiptInput;
use molten::world_head::canonical_world_head_transition_receipt;
use molten::world_promotion::LocalWorldPromotionStore;
use molten::world_promotion::WorldPromotionBookkeepingPort;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadAuthorityObservation;
use molten_core::world_head::WorldHeadClaimRef;
use molten_core::world_head::WorldHeadCurrentnessClass;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadStatementRef;
use molten_core::world_head::WorldHeadTransitionPlan;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use molten_node_host::node_state::RelativePath;
use serde_json::json;

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn seed_head(root: &DirectoryRoot, policy_ref: &WorldHeadPolicyRef) {
    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let mut heads = LocalWorldHeadStore::open(&storage).expect("head store");
    let state = WorldHeadState {
        branch_id: WorldBranchId::new("release").expect("branch"),
        branch_class: WorldBranchClass::Candidate,
        head: WorldCommitRef::new(reference("active")).expect("head"),
        generation: 1,
        policy_ref: policy_ref.clone(),
    };
    let claim_ref = WorldHeadClaimRef::new(reference("seed-claim")).expect("claim");
    let plan = WorldHeadTransitionPlan {
        claim_ref: claim_ref.clone(),
        before: None,
        after: state,
        choregraph_before_identity: None,
        choregraph_after_identity: reference("seed-choregraph"),
        currentness: WorldHeadCurrentnessClass::RelativeToObservedStore,
    };
    let statement_ref = WorldHeadStatementRef::new(reference("seed-statement")).expect("statement");
    let receipt = canonical_world_head_transition_receipt(&WorldHeadTransitionReceiptInput {
        decision: "admitted",
        plan: Some(&plan),
        claim_ref: &claim_ref,
        statement_ref: &statement_ref,
        authentication_decision_ref: &reference("seed-authentication"),
        authority_ref: &reference("seed-authority"),
        issue_codes: &[],
    })
    .expect("receipt");
    heads
        .apply_transition(&plan, &receipt, |_observed| {
            Ok(WorldHeadFreshAdmission {
                authentication_passed: true,
                authority: WorldHeadAuthorityObservation {
                    authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(reference("seed-authority"))
                        .expect("authority"),
                    policy_ref: policy_ref.clone(),
                    admitted: true,
                    observed_generation: 0,
                },
            })
        })
        .expect("seed");
}

fn write(path: &Path, value: serde_json::Value) {
    std::fs::write(path, serde_json::to_vec(&value).expect("json")).expect("write");
}

fn fixtures(root: &Path, policy_ref: &WorldHeadPolicyRef) -> (String, PathBuf, PathBuf, PathBuf) {
    let signer = SigningKey::from_bytes(&[11_u8; 32]);
    let signer_hex = hex::encode(signer.verifying_key().to_bytes());
    let authority_ref = reference("authority");
    let operation_ref = reference("operation");
    let intent_ref = reference("intent-release");
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

fn run_promote(
    root: &Path,
    request_path: &Path,
    authority_path: &Path,
    closure_path: &Path,
    signer_hex: &str,
) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_molten"))
        .args([
            "world-promotion",
            "promote",
            "--state-root",
            root.to_str().expect("path"),
            "--request",
            request_path.to_str().expect("path"),
            "--authority",
            authority_path.to_str().expect("path"),
            "--closure",
            closure_path.to_str().expect("path"),
            "--trusted-signer",
            signer_hex,
        ])
        .output()
        .expect("command");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn public_promote_command_commits_head_reservations_and_receipt() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = DirectoryRoot::open(temporary.path()).expect("state root");
    root.create_layout().expect("layout");
    let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
    seed_head(&root, &policy_ref);
    let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
    let stdout = run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
    assert!(stdout.contains("persistence="), "stdout={stdout}");

    let storage = root.namespace(NamespaceKind::Storage).expect("storage");
    let store = LocalWorldPromotionStore::open(&storage).expect("promotion store");
    assert_eq!(store.list_reservations().expect("reservations").len(), 1);
    let head = store.read_head(&WorldBranchId::new("release").expect("branch")).expect("head").expect("head");
    assert_eq!(head.head.as_str(), reference("candidate"));
    let receipts = storage
        .existing_subdir(&RelativePath::parse("world-promotion/receipts").expect("path"))
        .expect("dir")
        .expect("receipt directory");
    assert_eq!(receipts.list_entries().expect("entries").len(), 1);
}
