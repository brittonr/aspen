use std::cell::RefCell;
use std::rc::Rc;

use molten_core::dag_sync::*;

use super::*;
use crate::error::Result;

const DIGEST_HEX_LENGTH: usize = 64;
const NODE_BYTES: u64 = 10;
const GENERATION: u64 = 1;
const DEFER_AFTER_FIRST_RESPONSE: usize = 1;

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

fn graph() -> DagGraph {
    DagGraph {
        roots: vec![DagRoot {
            root_ref: DagRootRef::new(digest('2')).expect("root"),
            domain: "fixture".to_string(),
            node_ref: node_ref(),
            schema_ref: DagSchemaRef::new(digest('5')).expect("schema"),
        }],
        nodes: vec![DagNode {
            node_ref: node_ref(),
            schema_ref: DagSchemaRef::new(digest('5')).expect("schema"),
            payload_ref: Some(DagContentRef::new(digest('6')).expect("content")),
            encoded_bytes: NODE_BYTES,
            edges: Vec::new(),
        }],
    }
}

#[test]
fn canonical_records_are_stable_and_domain_separated() {
    let graph = graph();
    let node = graph.nodes.first().expect("node").clone();
    let root = graph.roots.first().expect("root").clone();
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

struct Authority;

impl DagAuthorityPort for Authority {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation> {
        Ok(DagAuthorityObservation {
            authority_ref: digest('7'),
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: true,
        })
    }
}

struct Resources;

impl DagResourcePort for Resources {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation> {
        Ok(DagResourceObservation {
            reservation_ref: digest('8'),
            plan_ref: plan.plan_ref.clone(),
            admitted: true,
        })
    }
}

struct Transport {
    calls: usize,
    defer_after: Option<usize>,
}

impl DagTransportPort for Transport {
    fn request(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome> {
        if self.defer_after == Some(self.calls) {
            return Ok(DagTransferOutcome::Deferred(digest('a')));
        }
        self.calls += 1;
        Ok(DagTransferOutcome::Received(DagTransportEnvelope {
            object_ref: request.object_ref.clone(),
            assigned_peer: request.assigned_peer.clone(),
            encoded_bytes: NODE_BYTES,
            transport_observation_ref: digest('b'),
        }))
    }
}

struct Content {
    corrupt: bool,
}

impl DagContentVerificationPort for Content {
    fn verify(
        &mut self,
        plan: &DagSyncPlan,
        envelope: &DagTransportEnvelope,
        _authority_ref: &str,
    ) -> Result<DagResponseObservation> {
        Ok(DagResponseObservation {
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            object_ref: envelope.object_ref.clone(),
            assigned_peer: envelope.assigned_peer.clone(),
            identity_verified: !self.corrupt,
            authorization_admitted: true,
            encoded_bytes: envelope.encoded_bytes,
        })
    }
}

struct Progress {
    loaded: Option<DagSyncProgress>,
    stored: Vec<DagSyncProgress>,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagProgressPort for Progress {
    fn load(&mut self, _epoch_ref: &DagEpochRef) -> Result<Option<DagSyncProgress>> {
        Ok(self.loaded.clone())
    }

    fn store(&mut self, progress: &DagSyncProgress) -> Result<String> {
        self.events.borrow_mut().push("store-progress");
        self.stored.push(progress.clone());
        Ok(digest('c'))
    }
}

struct Observations {
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagObservationPort for Observations {
    fn publish_response(&mut self, _response: &CanonicalDagRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-response");
        Ok(())
    }

    fn publish_progress(&mut self, _progress: &CanonicalDagRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-progress");
        Ok(())
    }
}

struct Receipts {
    events: Rc<RefCell<Vec<&'static str>>>,
    count: usize,
}

impl DagReceiptPort for Receipts {
    fn publish_receipt(&mut self, _receipt: &CanonicalDagRecord) -> Result<()> {
        self.events.borrow_mut().push("publish-receipt");
        self.count += 1;
        Ok(())
    }
}

#[test]
fn receiver_driven_shell_persists_each_verified_object_and_receipt_last() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts {
        events: events.clone(),
        count: 0,
    };
    let outcome = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("complete DAG sync");
    assert_eq!(outcome.receipt.decision, DagSyncDecision::Complete);
    assert!(outcome.receipt.missing.is_empty());
    assert_eq!(progress.stored.len(), outcome.plan.requests.len());
    assert_eq!(receipts.count, 1);
    assert_eq!(events.borrow().last(), Some(&"publish-receipt"));
}

#[test]
fn deferral_and_corruption_never_publish_false_completion() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = Transport {
        calls: 0,
        defer_after: Some(DEFER_AFTER_FIRST_RESPONSE),
    };
    let mut content = Content { corrupt: false };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts {
        events: events.clone(),
        count: 0,
    };
    let partial = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    })
    .expect("partial DAG sync");
    assert_eq!(partial.receipt.decision, DagSyncDecision::Partial);
    assert_eq!(partial.receipt.issues, vec![DagSyncIssue::TransferDeferred]);
    assert!(!partial.receipt.missing.is_empty());

    let events = Rc::new(RefCell::new(Vec::new()));
    let mut authority = Authority;
    let mut resources = Resources;
    let mut transport = Transport {
        calls: 0,
        defer_after: None,
    };
    let mut content = Content { corrupt: true };
    let mut progress = Progress {
        loaded: None,
        stored: Vec::new(),
        events: events.clone(),
    };
    let mut observations = Observations { events: events.clone() };
    let mut receipts = Receipts { events, count: 0 };
    let corrupt = run_dag_sync(&graph(), request(), DagSyncPorts {
        authority: &mut authority,
        resources: &mut resources,
        transport: &mut transport,
        content: &mut content,
        progress: &mut progress,
        observations: &mut observations,
        receipts: &mut receipts,
    });
    assert!(corrupt.is_err());
    assert!(progress.stored.is_empty());
    assert_eq!(receipts.count, 0);
}
