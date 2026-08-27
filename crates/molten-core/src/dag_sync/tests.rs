use super::*;

const DIGEST_HEX_LENGTH: usize = 64;
const NODE_BYTES: u64 = 10;
const GENERATION: u64 = 1;
const DEEP_GRAPH_LIMIT: usize = 1;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn node_ref(byte: char) -> DagNodeRef {
    DagNodeRef::new(digest(byte)).expect("node ref")
}

fn content_ref(byte: char) -> DagContentRef {
    DagContentRef::new(digest(byte)).expect("content ref")
}

fn edge(kind: DagEdgeKind, target: char) -> DagEdge {
    DagEdge {
        kind,
        target: node_ref(target),
    }
}

fn node(byte: char, payload: char, edges: Vec<DagEdge>) -> DagNode {
    DagNode {
        node_ref: node_ref(byte),
        schema_ref: DagSchemaRef::new(digest('9')).expect("schema"),
        payload_ref: Some(content_ref(payload)),
        encoded_bytes: NODE_BYTES,
        edges,
    }
}

fn graph() -> DagGraph {
    DagGraph {
        roots: vec![DagRoot {
            root_ref: DagRootRef::new(digest('0')).expect("root"),
            domain: "fixture".to_string(),
            node_ref: node_ref('a'),
            schema_ref: DagSchemaRef::new(digest('9')).expect("schema"),
        }],
        nodes: vec![
            node('a', '1', vec![edge(DagEdgeKind::Child, 'b'), edge(DagEdgeKind::Child, 'c')]),
            node('b', '2', vec![edge(DagEdgeKind::Dependency, 'd')]),
            node('c', '3', vec![edge(DagEdgeKind::Dependency, 'd')]),
            node('d', '4', Vec::new()),
        ],
    }
}

fn request(strategy: DagSyncStrategy) -> DagSyncRequest {
    DagSyncRequest {
        root_refs: vec![DagRootRef::new(digest('0')).expect("root")],
        strategy,
        inventory: DagInventory::default(),
        progress: None,
        peers: if strategy == DagSyncStrategy::PeerPartitioned {
            vec![
                DagPeerId::new("peer-a").expect("peer"),
                DagPeerId::new("peer-b").expect("peer"),
            ]
        } else {
            Vec::new()
        },
        epoch_ref: DagEpochRef::new(digest('5')).expect("epoch"),
        generation: GENERATION,
        policy_ref: DagPolicyRef::new(digest('6')).expect("policy"),
        bounds: DagBounds::default(),
    }
}

#[test]
fn stable_topology_and_plan_identity_ignore_input_order() {
    let first = plan_dag_sync(&graph(), &request(DagSyncStrategy::StemFirst)).plan.expect("plan");
    let mut reordered = graph();
    reordered.nodes.reverse();
    for node in &mut reordered.nodes {
        node.edges.reverse();
    }
    let second = plan_dag_sync(&reordered, &request(DagSyncStrategy::StemFirst)).plan.expect("reordered plan");
    assert_eq!(first.plan_ref, second.plan_ref);
    assert_eq!(first.topological_nodes, second.topological_nodes);
    assert_eq!(first.requests, second.requests);
    assert_eq!(first.topological_nodes.first(), Some(&node_ref('a')));
    assert_eq!(first.topological_nodes.last(), Some(&node_ref('d')));
}

#[test]
fn cycles_unknown_edges_duplicates_and_bounds_fail_closed() {
    let mut cycle = graph();
    cycle.nodes.last_mut().expect("leaf node").edges.push(edge(DagEdgeKind::Reference, 'a'));
    assert_eq!(plan_dag_sync(&cycle, &request(DagSyncStrategy::Full)).issues, vec![DagSyncIssue::Cycle]);

    let mut unknown = graph();
    unknown.nodes.first_mut().expect("root node").edges.push(edge(DagEdgeKind::Reference, 'e'));
    assert!(
        plan_dag_sync(&unknown, &request(DagSyncStrategy::Full))
            .issues
            .contains(&DagSyncIssue::UnknownEdgeTarget)
    );

    let mut duplicate = graph();
    let duplicate_node = duplicate.nodes.first().expect("root node").clone();
    duplicate.nodes.push(duplicate_node);
    assert!(
        plan_dag_sync(&duplicate, &request(DagSyncStrategy::Full))
            .issues
            .contains(&DagSyncIssue::DuplicateNode)
    );

    let mut bounded = request(DagSyncStrategy::Full);
    bounded.bounds.max_depth = DEEP_GRAPH_LIMIT;
    assert_eq!(plan_dag_sync(&graph(), &bounded).issues, vec![DagSyncIssue::DepthBoundExceeded]);
}

#[test]
fn strategies_are_closed_deterministic_and_resumable() {
    let full = plan_dag_sync(&graph(), &request(DagSyncStrategy::Full)).plan.expect("full");
    let stem = plan_dag_sync(&graph(), &request(DagSyncStrategy::StemFirst)).plan.expect("stem");
    let leaves = plan_dag_sync(&graph(), &request(DagSyncStrategy::LeafOnly)).plan.expect("leaves");
    assert_eq!(full.requests.len(), stem.requests.len());
    assert_eq!(leaves.requests.len(), 1);
    assert_eq!(leaves.requests.first().expect("leaf request").object_ref, DagObjectRef::Content(content_ref('4')));
    assert_ne!(full.plan_ref, stem.plan_ref);

    let mut resume = request(DagSyncStrategy::Resumable);
    resume.progress = Some(DagSyncProgress {
        epoch_ref: resume.epoch_ref.clone(),
        generation: resume.generation,
        strategy: resume.strategy,
        policy_ref: resume.policy_ref.clone(),
        verified: vec![DagObjectRef::Node(node_ref('a'))],
        steps_completed: 1,
    });
    let resumed = plan_dag_sync(&graph(), &resume).plan.expect("resume");
    assert!(!resumed.missing.contains(&DagObjectRef::Node(node_ref('a'))));

    resume.progress.as_mut().expect("progress").generation += 1;
    assert_eq!(plan_dag_sync(&graph(), &resume).issues, vec![DagSyncIssue::ProgressGenerationMismatch]);
}

#[test]
fn peer_partition_and_response_admission_are_generation_fenced() {
    let plan = plan_dag_sync(&graph(), &request(DagSyncStrategy::PeerPartitioned)).plan.expect("partitioned plan");
    assert!(plan.requests.iter().all(|request| request.assigned_peer.is_some()));
    let first = plan.requests.first().expect("request");
    let progress = DagSyncProgress {
        epoch_ref: plan.epoch_ref.clone(),
        generation: plan.generation,
        strategy: plan.strategy,
        policy_ref: DagPolicyRef::new(digest('6')).expect("policy"),
        verified: Vec::new(),
        steps_completed: 0,
    };
    let observation = DagResponseObservation {
        epoch_ref: plan.epoch_ref.clone(),
        generation: plan.generation,
        object_ref: first.object_ref.clone(),
        assigned_peer: first.assigned_peer.clone(),
        identity_verified: true,
        authorization_admitted: true,
        encoded_bytes: NODE_BYTES,
    };
    let advanced = admit_dag_response(&plan, &progress, &observation).expect("admitted response");
    assert_eq!(advanced.verified, vec![first.object_ref.clone()]);

    let mut unsolicited = observation.clone();
    unsolicited.object_ref = DagObjectRef::Node(node_ref('f'));
    assert_eq!(admit_dag_response(&plan, &progress, &unsolicited), Err(DagSyncIssue::UnsolicitedResponse));
    let mut corrupt = observation.clone();
    corrupt.identity_verified = false;
    assert_eq!(admit_dag_response(&plan, &progress, &corrupt), Err(DagSyncIssue::ResponseIdentityMismatch));
    let mut stale = observation;
    stale.generation += 1;
    assert_eq!(admit_dag_response(&plan, &progress, &stale), Err(DagSyncIssue::ResponseGenerationMismatch));
}
