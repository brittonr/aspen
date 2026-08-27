use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

const PLAN_IDENTITY_CONTEXT: &str = "onixresearch.molten.dag-sync.plan.identity.v1";

pub fn plan_dag_sync(graph: &DagGraph, request: &DagSyncRequest) -> DagPlanResult {
    let mut issues = validate_shape(graph, request);
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return DagPlanResult { plan: None, issues };
    }
    let nodes = graph.nodes.iter().map(|node| (node.node_ref.clone(), node)).collect::<BTreeMap<_, _>>();
    let roots = graph.roots.iter().map(|root| (root.root_ref.clone(), root)).collect::<BTreeMap<_, _>>();
    let reachable = match reachable_nodes(request, &roots, &nodes) {
        Ok(reachable) => reachable,
        Err(issue) => return failed(issue),
    };
    let topological_nodes = match topological_order(&reachable, &nodes, &request.bounds) {
        Ok(order) => order,
        Err(issue) => return failed(issue),
    };
    let inventory = match unique_objects(&request.inventory.available) {
        Ok(inventory) => inventory,
        Err(issue) => return failed(issue),
    };
    let verified = match validate_progress(request, &reachable, &nodes) {
        Ok(verified) => verified,
        Err(issue) => return failed(issue),
    };
    let objects = strategy_objects(request.strategy, &topological_nodes, &nodes);
    let missing = objects
        .into_iter()
        .filter(|object| !inventory.contains(object) && !verified.contains(object))
        .collect::<Vec<_>>();
    if missing.len() > request.bounds.max_steps {
        return failed(DagSyncIssue::StepBoundExceeded);
    }
    let peers = request.peers.iter().cloned().collect::<BTreeSet<_>>();
    let requests = missing
        .iter()
        .enumerate()
        .map(|(sequence, object_ref)| DagFetchRequest {
            object_ref: object_ref.clone(),
            assigned_peer: assigned_peer(request.strategy, object_ref, &peers),
            sequence,
        })
        .collect::<Vec<_>>();
    let mut root_refs = request.root_refs.clone();
    root_refs.sort();
    root_refs.dedup();
    let plan_ref = identify_plan(request, &root_refs, &topological_nodes, &requests);
    DagPlanResult {
        plan: Some(DagSyncPlan {
            plan_ref,
            epoch_ref: request.epoch_ref.clone(),
            generation: request.generation,
            strategy: request.strategy,
            roots: root_refs,
            topological_nodes,
            complete: missing.is_empty(),
            missing,
            requests,
        }),
        issues: Vec::new(),
    }
}

pub fn admit_dag_response(
    plan: &DagSyncPlan,
    progress: &DagSyncProgress,
    observation: &DagResponseObservation,
) -> Result<DagSyncProgress, DagSyncIssue> {
    if observation.epoch_ref != plan.epoch_ref || progress.epoch_ref != plan.epoch_ref {
        return Err(DagSyncIssue::ResponseEpochMismatch);
    }
    if observation.generation != plan.generation || progress.generation != plan.generation {
        return Err(DagSyncIssue::ResponseGenerationMismatch);
    }
    let Some(expected) = plan.requests.iter().find(|request| request.object_ref == observation.object_ref) else {
        return Err(DagSyncIssue::UnsolicitedResponse);
    };
    if expected.assigned_peer != observation.assigned_peer {
        return Err(DagSyncIssue::ResponsePeerMismatch);
    }
    if !observation.identity_verified {
        return Err(DagSyncIssue::ResponseIdentityMismatch);
    }
    if !observation.authorization_admitted {
        return Err(DagSyncIssue::ResponseUnauthorized);
    }
    if observation.encoded_bytes == 0 || observation.encoded_bytes > MAX_DAG_BYTES {
        return Err(DagSyncIssue::ResponseByteMismatch);
    }
    let mut verified = progress.verified.clone();
    if !verified.contains(&observation.object_ref) {
        verified.push(observation.object_ref.clone());
    }
    verified.sort();
    let steps_completed = progress.steps_completed.checked_add(1).ok_or(DagSyncIssue::StepBoundExceeded)?;
    if steps_completed > MAX_DAG_STEPS {
        return Err(DagSyncIssue::StepBoundExceeded);
    }
    Ok(DagSyncProgress {
        epoch_ref: progress.epoch_ref.clone(),
        generation: progress.generation,
        strategy: progress.strategy,
        policy_ref: progress.policy_ref.clone(),
        verified,
        steps_completed,
    })
}

fn validate_shape(graph: &DagGraph, request: &DagSyncRequest) -> Vec<DagSyncIssue> {
    let mut issues = Vec::new();
    let bounds = request.bounds;
    if bounds.max_nodes == 0
        || bounds.max_edges == 0
        || bounds.max_roots == 0
        || bounds.max_depth == 0
        || bounds.max_bytes == 0
        || bounds.max_steps == 0
        || bounds.max_peers == 0
    {
        issues.push(DagSyncIssue::InvalidBounds);
    }
    if request.generation == 0 {
        issues.push(DagSyncIssue::ProgressGenerationMismatch);
    }
    if request.root_refs.is_empty() {
        issues.push(DagSyncIssue::EmptyRoots);
    }
    if graph.roots.len() > bounds.max_roots || request.root_refs.len() > bounds.max_roots {
        issues.push(DagSyncIssue::TooManyRoots);
    }
    if graph.nodes.len() > bounds.max_nodes {
        issues.push(DagSyncIssue::TooManyNodes);
    }
    if request.peers.len() > bounds.max_peers {
        issues.push(DagSyncIssue::TooManyPeers);
    }
    if request.strategy == DagSyncStrategy::PeerPartitioned && request.peers.is_empty() {
        issues.push(DagSyncIssue::StrategyRequiresPeers);
    }
    let mut root_refs = BTreeSet::new();
    for root in &graph.roots {
        if !root_refs.insert(root.root_ref.clone()) {
            issues.push(DagSyncIssue::DuplicateRoot);
        }
        if root.validate_domain().is_err() {
            issues.push(DagSyncIssue::InvalidDomain);
        }
    }
    let mut requested_roots = BTreeSet::new();
    for root in &request.root_refs {
        if !requested_roots.insert(root.clone()) {
            issues.push(DagSyncIssue::DuplicateRoot);
        }
        if !root_refs.contains(root) {
            issues.push(DagSyncIssue::UnknownRoot);
        }
    }
    let mut node_refs = BTreeSet::new();
    let mut edge_count = 0_usize;
    for node in &graph.nodes {
        if !node_refs.insert(node.node_ref.clone()) {
            issues.push(DagSyncIssue::DuplicateNode);
        }
        if node.encoded_bytes == 0 || node.encoded_bytes > bounds.max_bytes {
            issues.push(DagSyncIssue::InvalidNodeLength);
        }
        edge_count = edge_count.saturating_add(node.edges.len());
        let mut edges = BTreeSet::new();
        for edge in &node.edges {
            if !edges.insert(edge.clone()) {
                issues.push(DagSyncIssue::DuplicateEdge);
            }
        }
    }
    if edge_count > bounds.max_edges {
        issues.push(DagSyncIssue::TooManyEdges);
    }
    for node in &graph.nodes {
        if node.edges.iter().any(|edge| !node_refs.contains(&edge.target)) {
            issues.push(DagSyncIssue::UnknownEdgeTarget);
        }
    }
    issues
}

fn reachable_nodes(
    request: &DagSyncRequest,
    roots: &BTreeMap<DagRootRef, &DagRoot>,
    nodes: &BTreeMap<DagNodeRef, &DagNode>,
) -> Result<BTreeSet<DagNodeRef>, DagSyncIssue> {
    let mut stack = request
        .root_refs
        .iter()
        .filter_map(|root_ref| roots.get(root_ref).map(|root| root.node_ref.clone()))
        .collect::<Vec<_>>();
    stack.sort_by(|left, right| right.cmp(left));
    let mut visited = BTreeSet::new();
    let mut steps = 0_usize;
    while let Some(node_ref) = stack.pop() {
        steps = steps.checked_add(1).ok_or(DagSyncIssue::StepBoundExceeded)?;
        if steps > request.bounds.max_steps {
            return Err(DagSyncIssue::StepBoundExceeded);
        }
        if !visited.insert(node_ref.clone()) {
            continue;
        }
        let node = nodes.get(&node_ref).ok_or(DagSyncIssue::UnknownRoot)?;
        let mut targets = node.edges.iter().map(|edge| edge.target.clone()).collect::<Vec<_>>();
        targets.sort_by(|left, right| right.cmp(left));
        stack.extend(targets);
    }
    if visited.len() > request.bounds.max_nodes {
        return Err(DagSyncIssue::TooManyNodes);
    }
    Ok(visited)
}

fn topological_order(
    reachable: &BTreeSet<DagNodeRef>,
    nodes: &BTreeMap<DagNodeRef, &DagNode>,
    bounds: &DagBounds,
) -> Result<Vec<DagNodeRef>, DagSyncIssue> {
    let mut indegree = reachable.iter().map(|node_ref| (node_ref.clone(), 0_usize)).collect::<BTreeMap<_, _>>();
    let mut bytes = 0_u64;
    for node_ref in reachable {
        let node = nodes.get(node_ref).ok_or(DagSyncIssue::UnknownEdgeTarget)?;
        bytes = bytes.checked_add(node.encoded_bytes).ok_or(DagSyncIssue::ByteBoundExceeded)?;
        for edge in &node.edges {
            if reachable.contains(&edge.target) {
                let value = indegree.get_mut(&edge.target).ok_or(DagSyncIssue::UnknownEdgeTarget)?;
                *value = value.checked_add(1).ok_or(DagSyncIssue::TooManyEdges)?;
            }
        }
    }
    if bytes > bounds.max_bytes {
        return Err(DagSyncIssue::ByteBoundExceeded);
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node_ref, degree)| (*degree == 0).then_some(node_ref.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(reachable.len());
    let mut depth = reachable.iter().map(|node_ref| (node_ref.clone(), 0_usize)).collect::<BTreeMap<_, _>>();
    while let Some(node_ref) = ready.pop_first() {
        order.push(node_ref.clone());
        let node_depth = *depth.get(&node_ref).ok_or(DagSyncIssue::DepthBoundExceeded)?;
        if node_depth > bounds.max_depth {
            return Err(DagSyncIssue::DepthBoundExceeded);
        }
        let node = nodes.get(&node_ref).ok_or(DagSyncIssue::UnknownEdgeTarget)?;
        let mut edges = node.edges.iter().collect::<Vec<_>>();
        edges.sort();
        for edge in edges {
            if !reachable.contains(&edge.target) {
                continue;
            }
            let target_depth = depth.get_mut(&edge.target).ok_or(DagSyncIssue::DepthBoundExceeded)?;
            *target_depth = (*target_depth).max(node_depth.saturating_add(1));
            let degree = indegree.get_mut(&edge.target).ok_or(DagSyncIssue::UnknownEdgeTarget)?;
            *degree = degree.checked_sub(1).ok_or(DagSyncIssue::Cycle)?;
            if *degree == 0 {
                ready.insert(edge.target.clone());
            }
        }
    }
    if order.len() != reachable.len() {
        return Err(DagSyncIssue::Cycle);
    }
    Ok(order)
}

fn validate_progress(
    request: &DagSyncRequest,
    reachable: &BTreeSet<DagNodeRef>,
    nodes: &BTreeMap<DagNodeRef, &DagNode>,
) -> Result<BTreeSet<DagObjectRef>, DagSyncIssue> {
    let Some(progress) = &request.progress else {
        return Ok(BTreeSet::new());
    };
    if progress.epoch_ref != request.epoch_ref {
        return Err(DagSyncIssue::ProgressEpochMismatch);
    }
    if progress.generation != request.generation {
        return Err(DagSyncIssue::ProgressGenerationMismatch);
    }
    if progress.strategy != request.strategy {
        return Err(DagSyncIssue::ProgressStrategyMismatch);
    }
    if progress.policy_ref != request.policy_ref {
        return Err(DagSyncIssue::ProgressPolicyMismatch);
    }
    if progress.steps_completed > request.bounds.max_steps {
        return Err(DagSyncIssue::ProgressStepRegression);
    }
    let verified = unique_objects(&progress.verified)?;
    let valid_objects = strategy_objects(request.strategy, &reachable.iter().cloned().collect::<Vec<_>>(), nodes)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if verified.iter().any(|object| !valid_objects.contains(object)) {
        return Err(DagSyncIssue::ProgressContainsUnknownObject);
    }
    Ok(verified)
}

fn strategy_objects(
    strategy: DagSyncStrategy,
    order: &[DagNodeRef],
    nodes: &BTreeMap<DagNodeRef, &DagNode>,
) -> Vec<DagObjectRef> {
    let mut objects = Vec::new();
    match strategy {
        DagSyncStrategy::StemFirst => {
            objects.extend(order.iter().cloned().map(DagObjectRef::Node));
            objects.extend(order.iter().filter_map(|node_ref| {
                nodes.get(node_ref).and_then(|node| node.payload_ref.clone()).map(DagObjectRef::Content)
            }));
        }
        DagSyncStrategy::LeafOnly => {
            objects.extend(order.iter().filter_map(|node_ref| {
                let node = nodes.get(node_ref)?;
                node.edges.is_empty().then(|| node.payload_ref.clone().map(DagObjectRef::Content)).flatten()
            }));
        }
        DagSyncStrategy::Full | DagSyncStrategy::Resumable | DagSyncStrategy::PeerPartitioned => {
            for node_ref in order {
                objects.push(DagObjectRef::Node(node_ref.clone()));
                if let Some(content_ref) = nodes.get(node_ref).and_then(|node| node.payload_ref.clone()) {
                    objects.push(DagObjectRef::Content(content_ref));
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    objects.retain(|object| seen.insert(object.clone()));
    objects
}

fn unique_objects(objects: &[DagObjectRef]) -> Result<BTreeSet<DagObjectRef>, DagSyncIssue> {
    let mut unique = BTreeSet::new();
    for object in objects {
        if !unique.insert(object.clone()) {
            return Err(DagSyncIssue::DuplicateInventoryObject);
        }
    }
    Ok(unique)
}

fn assigned_peer(
    strategy: DagSyncStrategy,
    object_ref: &DagObjectRef,
    peers: &BTreeSet<DagPeerId>,
) -> Option<DagPeerId> {
    if strategy != DagSyncStrategy::PeerPartitioned || peers.is_empty() {
        return None;
    }
    let digest = blake3::hash(object_ref.as_str().as_bytes());
    let mut prefix = [0_u8; std::mem::size_of::<u64>()];
    prefix.copy_from_slice(&digest.as_bytes()[..std::mem::size_of::<u64>()]);
    let index = usize::try_from(u64::from_be_bytes(prefix)).unwrap_or(usize::MAX) % peers.len();
    peers.iter().nth(index).cloned()
}

fn identify_plan(
    request: &DagSyncRequest,
    roots: &[DagRootRef],
    nodes: &[DagNodeRef],
    requests: &[DagFetchRequest],
) -> DagPlanRef {
    let mut hasher = blake3::Hasher::new_derive_key(PLAN_IDENTITY_CONTEXT);
    update(&mut hasher, request.epoch_ref.as_str());
    update(&mut hasher, &request.generation.to_string());
    update(&mut hasher, request.strategy.as_str());
    update(&mut hasher, request.policy_ref.as_str());
    for root in roots {
        update(&mut hasher, root.as_str());
    }
    for node in nodes {
        update(&mut hasher, node.as_str());
    }
    for fetch in requests {
        update(&mut hasher, fetch.object_ref.kind());
        update(&mut hasher, fetch.object_ref.as_str());
        update(&mut hasher, fetch.assigned_peer.as_ref().map_or("", DagPeerId::as_str));
    }
    DagPlanRef::generated(hasher.finalize())
}

fn update(hasher: &mut blake3::Hasher, value: &str) {
    let length = u64::try_from(value.len()).unwrap_or(u64::MAX);
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
}

fn failed(issue: DagSyncIssue) -> DagPlanResult {
    DagPlanResult {
        plan: None,
        issues: vec![issue],
    }
}
