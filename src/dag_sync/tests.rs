use molten_core::dag_sync::*;

use super::*;

const DIGEST_HEX_LENGTH: usize = 64;
const NODE_BYTES: u64 = 10;
const GENERATION: u64 = 1;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn node_ref() -> DagNodeRef {
    DagNodeRef::new(digest('1')).expect("node")
}

fn request() -> DagSyncRequest {
    DagSyncRequest {
        root_refs: vec![DagRootRef::new(digest('2')).expect("root")],
        strategy: DagSyncStrategy::Full,
        inventory: DagInventory::default(),
        progress: None,
        peers: Vec::new(),
        epoch_ref: DagEpochRef::new(digest('3')).expect("epoch"),
        generation: GENERATION,
        policy_ref: DagPolicyRef::new(digest('4')).expect("policy"),
        bounds: DagBounds::default(),
    }
}

#[test]
fn canonical_records_are_stable_and_domain_separated() {
    let node = DagNode {
        node_ref: node_ref(),
        schema_ref: DagSchemaRef::new(digest('5')).expect("schema"),
        payload_ref: Some(DagContentRef::new(digest('6')).expect("content")),
        encoded_bytes: NODE_BYTES,
        edges: Vec::new(),
    };
    let root = DagRoot {
        root_ref: DagRootRef::new(digest('2')).expect("root"),
        domain: "fixture".to_string(),
        node_ref: node.node_ref.clone(),
        schema_ref: node.schema_ref.clone(),
    };
    let first = canonical_dag_node(&node).expect("node record");
    let repeated = canonical_dag_node(&node).expect("repeated node record");
    let root_record = canonical_dag_root(&root).expect("root record");
    let request_record = canonical_dag_request(&request()).expect("request record");
    assert_eq!(first.record_ref, repeated.record_ref);
    assert_eq!(first.bytes, repeated.bytes);
    assert_ne!(first.record_ref, root_record.record_ref);
    assert_ne!(root_record.record_ref, request_record.record_ref);
}

#[test]
fn receipt_requires_complete_non_claims() {
    let mut receipt = DagSyncReceipt {
        decision: DagSyncDecision::Partial,
        plan_ref: None,
        epoch_ref: DagEpochRef::new(digest('3')).expect("epoch"),
        generation: GENERATION,
        strategy: DagSyncStrategy::Full,
        requested: 1,
        verified: 0,
        missing: vec![DagObjectRef::Node(node_ref())],
        issues: Vec::new(),
        non_claims: DAG_SYNC_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    assert!(canonical_dag_receipt(&receipt).is_ok());
    receipt.non_claims.pop();
    assert!(canonical_dag_receipt(&receipt).is_err());
}
