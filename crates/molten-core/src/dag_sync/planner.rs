#![allow(
    tigerstyle::non_trait_imports,
    reason = "the pure planner uses ordered maps and sets repeatedly to preserve canonical plans"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

pub fn plan_dag_sync(graph: &DagGraph, request: &DagSyncRequest) -> DagPlanResult {
    let issues = validate_shape(graph, request);
    if !issues.is_empty() {
        return DagPlanResult { plan: None, issues };
    }
    match derive_plan_context(graph, request).and_then(|context| finish_plan(request, context)) {
        Ok(plan) => DagPlanResult {
            plan: Some(plan),
            issues: Vec::new(),
        },
        Err(issue) => failed(issue),
    }
}

struct PlanContext<'a> {
    nodes: BTreeMap<DagNodeRef, &'a DagNode>,
    topological_nodes: Vec<DagNodeRef>,
    inventory: BTreeSet<DagObjectRef>,
    verified: BTreeSet<DagObjectRef>,
    root_refs: Vec<DagRootRef>,
    schema_refs: Vec<DagSchemaRef>,
    peers: BTreeSet<DagPeerId>,
    peer_refs: Vec<DagPeerId>,
}

fn derive_plan_context<'a>(graph: &'a DagGraph, request: &DagSyncRequest) -> Result<PlanContext<'a>, DagSyncIssue> {
    let nodes = graph.nodes.iter().map(|node| (node.node_ref.clone(), node)).collect::<BTreeMap<_, _>>();
    let roots = graph.roots.iter().map(|root| (root.root_ref.clone(), root)).collect::<BTreeMap<_, _>>();
    let reachable = reachable_nodes(request, &roots, &nodes)?;
    let topological_nodes = topological_order(&reachable, &nodes, &request.bounds)?;
    let inventory = unique_objects(&request.inventory.available)?;
    let mut root_refs = request.root_refs.clone();
    root_refs.sort();
    root_refs.dedup();
    let schema_refs = progress_schema_refs(&root_refs, &roots, &reachable, &nodes)?;
    let peers = request.peers.iter().cloned().collect::<BTreeSet<_>>();
    let peer_refs = peers.iter().cloned().collect::<Vec<_>>();
    let progress_context = ProgressValidationContext {
        reachable: &reachable,
        nodes: &nodes,
        root_refs: &root_refs,
        schema_refs: &schema_refs,
        peers: &peer_refs,
    };
    let verified = validate_progress(request, &progress_context)?;
    Ok(PlanContext {
        nodes,
        topological_nodes,
        inventory,
        verified,
        root_refs,
        schema_refs,
        peers,
        peer_refs,
    })
}

fn finish_plan(request: &DagSyncRequest, context: PlanContext<'_>) -> Result<DagSyncPlan, DagSyncIssue> {
    let objects = strategy_objects(request.strategy, &context.topological_nodes, &context.nodes);
    let missing = objects
        .into_iter()
        .filter(|object| !context.inventory.contains(object) && !context.verified.contains(object))
        .collect::<Vec<_>>();
    if missing.len() > request.bounds.max_steps {
        return Err(DagSyncIssue::StepBoundExceeded);
    }
    let requests = missing
        .iter()
        .enumerate()
        .map(|(sequence, object_ref)| DagFetchRequest {
            object_ref: object_ref.clone(),
            assigned_peer: assigned_peer(request.strategy, object_ref, &context.peers),
            sequence,
        })
        .collect::<Vec<_>>();
    let identity_input = PlanIdentityInput {
        request,
        roots: &context.root_refs,
        schemas: &context.schema_refs,
        peers: &context.peer_refs,
        nodes: &context.topological_nodes,
        requests: &requests,
    };
    let plan_ref = identify_plan(&identity_input)?;
    Ok(DagSyncPlan {
        plan_ref,
        epoch_ref: request.epoch_ref.clone(),
        generation: request.generation,
        strategy: request.strategy,
        roots: context.root_refs,
        schema_refs: context.schema_refs,
        peers: context.peer_refs,
        topological_nodes: context.topological_nodes,
        complete: missing.is_empty(),
        missing,
        requests,
    })
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
    if progress.verified.contains(&observation.object_ref) {
        return Err(DagSyncIssue::DuplicateResponse);
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
    verified.push(observation.object_ref.clone());
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
        root_refs: progress.root_refs.clone(),
        schema_refs: progress.schema_refs.clone(),
        peers: progress.peers.clone(),
        verified,
        steps_completed,
    })
}

fn failed(issue: DagSyncIssue) -> DagPlanResult {
    DagPlanResult {
        plan: None,
        issues: vec![issue],
    }
}
