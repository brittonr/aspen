use std::collections::BTreeMap;
use std::collections::BTreeSet;

mod support;

use self::support::*;
use super::*;
use crate::dag_sync::DagEdge;
use crate::dag_sync::DagEdgeKind;
use crate::dag_sync::DagGraph;
use crate::dag_sync::DagInventory;
use crate::dag_sync::DagNode;
use crate::dag_sync::DagRoot;
use crate::dag_sync::DagSyncRequest;
use crate::dag_sync::plan_dag_sync;

// r[impl molten.world_distribution.closure]
#[allow(
    tigerstyle::function_length,
    reason = "the pure projection keeps validation, typed edge construction, and byte accounting in visible order"
)]
pub fn project_world_dag(input: &WorldDagProjectionInput) -> Result<WorldDagProjection, Vec<WorldDistributionIssue>> {
    let issue_capacity = input.commits.len().saturating_add(input.roots.len()).saturating_add(1);
    let mut issues = Vec::with_capacity(issue_capacity);
    validate_projection_bounds(input, &mut issues);
    let mut commits = BTreeMap::new();
    let mut roots = BTreeMap::new();

    for commit in &input.commits {
        if commits.insert(commit.commit_ref.clone(), commit).is_some() {
            issues.push(WorldDistributionIssue::DuplicateCommit(commit.commit_ref.as_str().to_string()));
        }
        validate_commit(commit, &input.bounds, &mut issues);
    }
    for root in &input.roots {
        if roots.insert(root.root.clone(), root).is_some() {
            issues.push(WorldDistributionIssue::DuplicateRoot(root.root.as_str().to_string()));
        }
        if root.encoded_bytes == 0 {
            issues.push(WorldDistributionIssue::EmptyRootObject(root.root.as_str().to_string()));
        }
    }
    if !commits.contains_key(&input.requested) {
        issues.push(WorldDistributionIssue::MissingRequestedCommit);
    }
    validate_dependencies(&commits, &roots, &mut issues);
    if !issues.is_empty() {
        return Err(normalize_issues(issues));
    }

    let commit_schema = commit_schema_ref().map_err(|issue| vec![issue])?;
    let mut nodes = Vec::with_capacity(commits.len().saturating_add(roots.len()));
    let mut objects = Vec::with_capacity(nodes.capacity());
    let mut total_bytes = 0_u64;

    for commit in commits.values() {
        let encoded_bytes =
            u64::try_from(commit.canonical_bytes.len()).map_err(|_| vec![WorldDistributionIssue::ByteLimitExceeded])?;
        total_bytes = add_bytes(total_bytes, encoded_bytes)?;
        let mut edges = Vec::with_capacity(commit.core.parents.len().saturating_add(commit.core.roots.len()));
        for parent in &commit.core.parents {
            edges.push(DagEdge {
                kind: DagEdgeKind::Child,
                target: node_ref(parent.as_str())?,
            });
        }
        for root in &commit.core.roots {
            edges.push(DagEdge {
                kind: DagEdgeKind::Dependency,
                target: node_ref(root.as_str())?,
            });
        }
        edges.sort();
        nodes.push(DagNode {
            node_ref: node_ref(commit.commit_ref.as_str())?,
            schema_ref: commit_schema.clone(),
            payload_ref: None,
            encoded_bytes,
            edges,
        });
        objects.push(WorldObjectDescriptor {
            object_ref: WorldObjectRef::Commit(commit.commit_ref.clone()),
            domain: WorldObjectDomain::Commit,
            schema_ref: commit_schema.clone(),
            encoded_bytes,
        });
    }
    for root in roots.values() {
        total_bytes = add_bytes(total_bytes, root.encoded_bytes)?;
        nodes.push(DagNode {
            node_ref: node_ref(root.root.as_str())?,
            schema_ref: root.schema_ref.clone(),
            payload_ref: None,
            encoded_bytes: root.encoded_bytes,
            edges: Vec::new(),
        });
        objects.push(WorldObjectDescriptor {
            object_ref: WorldObjectRef::Root(root.root.clone()),
            domain: WorldObjectDomain::Root(root.root.kind()),
            schema_ref: root.schema_ref.clone(),
            encoded_bytes: root.encoded_bytes,
        });
    }
    nodes.sort_by(|left, right| left.node_ref.cmp(&right.node_ref));
    objects.sort_by(|left, right| left.object_ref.cmp(&right.object_ref));
    let graph = DagGraph {
        roots: vec![DagRoot {
            root_ref: root_ref(input.requested.as_str())?,
            domain: WORLD_DAG_DOMAIN.to_string(),
            node_ref: node_ref(input.requested.as_str())?,
            schema_ref: commit_schema,
        }],
        nodes,
    };
    Ok(WorldDagProjection {
        requested: input.requested.clone(),
        graph,
        objects,
        total_bytes,
    })
}

// r[impl molten.world_distribution.closure]
// r[impl molten.world_distribution.partial]
pub fn plan_world_closure(
    projection: &WorldDagProjection,
    context: &WorldSyncContext,
) -> Result<WorldClosurePlan, Vec<WorldDistributionIssue>> {
    let known = projection.objects.iter().map(|descriptor| descriptor.object_ref.clone()).collect::<BTreeSet<_>>();
    let mut issues = context
        .inventory
        .iter()
        .filter(|object| !known.contains(*object))
        .map(|object| WorldDistributionIssue::InventoryObjectUnknown(object.as_str().to_string()))
        .collect::<Vec<_>>();
    if context.bounds.max_nodes > MAX_WORLD_DISTRIBUTION_OBJECTS
        || context.bounds.max_steps > MAX_WORLD_DISTRIBUTION_OBJECTS
        || context.bounds.max_bytes > MAX_WORLD_DISTRIBUTION_BYTES
    {
        issues.push(WorldDistributionIssue::InvalidBounds("world-sync-context"));
    }
    if !issues.is_empty() {
        return Err(normalize_issues(issues));
    }
    let available = context
        .inventory
        .iter()
        .map(world_object_to_dag)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|issue| vec![issue])?;
    let request = DagSyncRequest {
        root_refs: vec![root_ref(projection.requested.as_str())?],
        strategy: context.strategy,
        inventory: DagInventory { available },
        progress: context.progress.clone(),
        peers: context.peers.clone(),
        epoch_ref: context.epoch_ref.clone(),
        generation: context.generation,
        policy_ref: context.policy_ref.clone(),
        bounds: context.bounds,
    };
    let result = plan_dag_sync(&projection.graph, &request);
    let shared_plan = result.plan.ok_or_else(|| {
        normalize_issues(
            result
                .issues
                .into_iter()
                .map(|issue| WorldDistributionIssue::DagPlanningDenied(format!("{issue:?}")))
                .collect(),
        )
    })?;
    let missing = shared_plan
        .missing
        .iter()
        .map(|object| {
            dag_object_to_world(object, &projection.objects).ok_or_else(|| {
                WorldDistributionIssue::DagPlanningDenied("missing object has no world descriptor".to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|issue| vec![issue])?;
    Ok(WorldClosurePlan {
        projection: projection.clone(),
        request,
        complete: shared_plan.complete,
        shared_plan,
        missing,
        activation_authorized: false,
        non_claims: distribution_non_claims(),
    })
}

// r[impl molten.world_distribution.closure]
pub fn admit_world_activation(facts: &WorldActivationFacts) -> WorldActivationDecision {
    let checks = [
        ("closure-incomplete", facts.closure_complete),
        ("object-domain-unverified", facts.domains_verified),
        ("schema-not-admitted", facts.schemas_admitted),
        ("current-policy-denied", facts.current_policy_admitted),
        ("current-authority-denied", facts.current_authority_admitted),
        ("head-claim-not-admitted", facts.claim_admitted),
    ];
    let diagnostics = checks
        .into_iter()
        .filter(|(_diagnostic, admitted)| !admitted)
        .map(|(diagnostic, _admitted)| diagnostic.to_string())
        .collect::<Vec<_>>();
    WorldActivationDecision {
        admitted: diagnostics.is_empty(),
        diagnostics,
        non_claims: distribution_non_claims(),
    }
}
