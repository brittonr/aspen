#![allow(
    tigerstyle::non_trait_imports,
    reason = "bounded graph validation uses ordered maps and sets repeatedly to keep decisions canonical"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

pub(super) fn validate_shape(graph: &DagGraph, request: &DagSyncRequest) -> Vec<DagSyncIssue> {
    let mut issues = BTreeSet::new();
    validate_request_shape(graph, request, &mut issues);
    validate_root_shape(graph, request, &mut issues);
    validate_node_shape(graph, request, &mut issues);
    issues.into_iter().collect()
}

fn validate_request_shape(graph: &DagGraph, request: &DagSyncRequest, issues: &mut BTreeSet<DagSyncIssue>) {
    let bounds = request.bounds;
    if bounds.max_nodes == 0
        || bounds.max_edges == 0
        || bounds.max_roots == 0
        || bounds.max_depth == 0
        || bounds.max_bytes == 0
        || bounds.max_steps == 0
        || bounds.max_peers == 0
    {
        issues.insert(DagSyncIssue::InvalidBounds);
    }
    if request.generation == 0 {
        issues.insert(DagSyncIssue::ProgressGenerationMismatch);
    }
    if request.root_refs.is_empty() {
        issues.insert(DagSyncIssue::EmptyRoots);
    }
    if graph.roots.len() > bounds.max_roots || request.root_refs.len() > bounds.max_roots {
        issues.insert(DagSyncIssue::TooManyRoots);
    }
    if graph.nodes.len() > bounds.max_nodes {
        issues.insert(DagSyncIssue::TooManyNodes);
    }
    if request.peers.len() > bounds.max_peers {
        issues.insert(DagSyncIssue::TooManyPeers);
    }
    if request.strategy == DagSyncStrategy::PeerPartitioned && request.peers.is_empty() {
        issues.insert(DagSyncIssue::StrategyRequiresPeers);
    }
}

fn validate_root_shape(graph: &DagGraph, request: &DagSyncRequest, issues: &mut BTreeSet<DagSyncIssue>) {
    let mut root_refs = BTreeSet::new();
    for root in &graph.roots {
        if !root_refs.insert(root.root_ref.clone()) {
            issues.insert(DagSyncIssue::DuplicateRoot);
        }
        if root.validate_domain().is_err() {
            issues.insert(DagSyncIssue::InvalidDomain);
        }
    }
    let mut requested_roots = BTreeSet::new();
    for root in &request.root_refs {
        if !requested_roots.insert(root.clone()) {
            issues.insert(DagSyncIssue::DuplicateRoot);
        }
        if !root_refs.contains(root) {
            issues.insert(DagSyncIssue::UnknownRoot);
        }
    }
}

fn validate_node_shape(graph: &DagGraph, request: &DagSyncRequest, issues: &mut BTreeSet<DagSyncIssue>) {
    let mut node_refs = BTreeSet::new();
    let mut edge_count = 0_usize;
    for node in &graph.nodes {
        if !node_refs.insert(node.node_ref.clone()) {
            issues.insert(DagSyncIssue::DuplicateNode);
        }
        if node.encoded_bytes == 0 || node.encoded_bytes > request.bounds.max_bytes {
            issues.insert(DagSyncIssue::InvalidNodeLength);
        }
        edge_count = edge_count.saturating_add(node.edges.len());
        let mut edges = BTreeSet::new();
        if node.edges.iter().any(|edge| !edges.insert(edge.clone())) {
            issues.insert(DagSyncIssue::DuplicateEdge);
        }
    }
    if edge_count > request.bounds.max_edges {
        issues.insert(DagSyncIssue::TooManyEdges);
    }
    if graph.nodes.iter().any(|node| node.edges.iter().any(|edge| !node_refs.contains(&edge.target))) {
        issues.insert(DagSyncIssue::UnknownEdgeTarget);
    }
}

pub(super) fn reachable_nodes(
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

pub(super) fn topological_order(
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

pub(super) struct ProgressValidationContext<'a> {
    pub reachable: &'a BTreeSet<DagNodeRef>,
    pub nodes: &'a BTreeMap<DagNodeRef, &'a DagNode>,
    pub root_refs: &'a [DagRootRef],
    pub schema_refs: &'a [DagSchemaRef],
    pub peers: &'a [DagPeerId],
}

pub(super) fn validate_progress(
    request: &DagSyncRequest,
    context: &ProgressValidationContext<'_>,
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
    if progress.root_refs != context.root_refs {
        return Err(DagSyncIssue::ProgressRootMismatch);
    }
    if progress.schema_refs != context.schema_refs {
        return Err(DagSyncIssue::ProgressSchemaMismatch);
    }
    if progress.peers != context.peers {
        return Err(DagSyncIssue::ProgressPeerAssignmentMismatch);
    }
    if progress.steps_completed > request.bounds.max_steps {
        return Err(DagSyncIssue::ProgressStepRegression);
    }
    let verified = unique_objects(&progress.verified)?;
    let valid_objects =
        strategy_objects(request.strategy, &context.reachable.iter().cloned().collect::<Vec<_>>(), context.nodes)
            .into_iter()
            .collect::<BTreeSet<_>>();
    if verified.iter().any(|object| !valid_objects.contains(object)) {
        return Err(DagSyncIssue::ProgressContainsUnknownObject);
    }
    Ok(verified)
}

pub(super) fn strategy_objects(
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

pub(super) fn unique_objects(objects: &[DagObjectRef]) -> Result<BTreeSet<DagObjectRef>, DagSyncIssue> {
    let mut unique = BTreeSet::new();
    for object in objects {
        if !unique.insert(object.clone()) {
            return Err(DagSyncIssue::DuplicateInventoryObject);
        }
    }
    Ok(unique)
}
